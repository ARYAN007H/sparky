//! Public entry points (`start`/`stop`/`resume`) and the spawned engine loop
//! that drives a run's phase DAG to completion.

use crate::core::runtime::DispatchCtx;
use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::sync::{Arc, OnceLock};
use tinyagents_orchestration::workflow::{
    reset_running_phases, WorkflowEngine, WorkflowRunHandle, WorkflowRunSupervisor,
};

use crate::agent::harness::ParentExecutionContext;
use crate::agent::orchestration::parent_context::with_root_parent;
use crate::config::Config;
use tinyagents_session::run_ledger::{WorkflowRun, WorkflowRunStatus, WorkflowRunUpsert};

use super::ops::{
    compare_and_swap_workflow_run_lifecycle, definition_by_id, get_workflow_run, workflow_store,
};
use super::LOG_TARGET;
use tinyagents_orchestration::workflow::WorkflowDefinition;

fn workflow_supervisor() -> &'static WorkflowRunSupervisor {
    static SUPERVISOR: OnceLock<WorkflowRunSupervisor> = OnceLock::new();
    SUPERVISOR.get_or_init(WorkflowRunSupervisor::default)
}

/// Install the owner generation, then build OpenHuman's root context and
/// origin around the scheduled drive. The host retains the callback handle
/// only so failure projection can reject a stale generation.
fn schedule_workflow_drive(
    ctx: &DispatchCtx,
    run_id: String,
    definition: WorkflowDefinition,
    resume: bool,
) -> WorkflowRunHandle {
    let supervisor = workflow_supervisor().clone();
    let task_ctx = ctx.clone();
    let handle_slot = Arc::new(OnceLock::<WorkflowRunHandle>::new());
    let config_slot = Arc::new(OnceLock::<Config>::new());
    let parent_slot = Arc::new(OnceLock::<ParentExecutionContext>::new());
    let drive_handle_slot = Arc::clone(&handle_slot);
    let drive_config_slot = Arc::clone(&config_slot);
    let drive_parent_slot = Arc::clone(&parent_slot);
    let drive_ctx = task_ctx.clone();
    let drive_run_id = run_id.clone();
    let drive_definition = definition.clone();
    let root_ctx = task_ctx.clone();
    let root_config_slot = Arc::clone(&config_slot);
    let root_parent_slot = Arc::clone(&parent_slot);
    let root_run_id = run_id.clone();
    let root_parent = move |drive| {
        let task_ctx = root_ctx.clone();
        let config_slot = Arc::clone(&root_config_slot);
        let parent_slot = Arc::clone(&root_parent_slot);
        let run_id = root_run_id.clone();
        async move {
            match crate::config::ops::load_current_or_init(&task_ctx).await {
                Ok(task_config) => {
                    let _ = config_slot.set(task_config.clone());
                    let _ = with_root_parent(
                        &task_ctx,
                        None,
                        &task_config,
                        "workflow_engine",
                        "workflow",
                        "workflow",
                        |parent| async move {
                            let _ = parent_slot.set(parent);
                            drive.await;
                        },
                    )
                    .await;
                }
                Err(error) => log::error!(
                    target: LOG_TARGET,
                    "[workflow_run_engine] drive.config_load_failed run={run_id} err={error}"
                ),
            }
        }
    };
    let inherited_origin = crate::security::turn_origin::capture();
    let origin = move |future| {
        crate::security::turn_origin::with_inherited_origin(inherited_origin, future)
    };
    let make_future = move |cancel_token| {
        let ctx = drive_ctx.clone();
        let run_id = drive_run_id.clone();
        let definition = drive_definition.clone();
        let handle_slot = Arc::clone(&drive_handle_slot);
        let config_slot = Arc::clone(&drive_config_slot);
        let parent_slot = Arc::clone(&drive_parent_slot);
        async move {
            let (Some(handle), Some(config), Some(parent)) = (
                handle_slot.get().cloned(),
                config_slot.get().cloned(),
                parent_slot.get().cloned(),
            ) else {
                return;
            };
            run_engine_loop(
                &config,
                &run_id,
                definition,
                handle,
                cancel_token,
                Some(parent),
            )
            .await;
        }
    };
    let spawn_ctx = task_ctx;
    let spawn_handle_slot = Arc::clone(&handle_slot);
    let spawn = move |handle, future| {
        let _ = spawn_handle_slot.set(handle);
        spawn_ctx.spawn(future);
    };

    if resume {
        supervisor.resume_scoped_with_handle(run_id, make_future, root_parent, origin, spawn)
    } else {
        supervisor.start_scoped_with_handle(run_id, make_future, root_parent, origin, spawn)
    }
}

/// Start a new workflow run and return immediately.
///
/// Resolves `definition_id` to a builtin [`WorkflowDefinition`], creates a
/// `Running` ledger row with `phase_states` initialised to one `pending` entry
/// per phase, persists it, then `tokio::spawn`s the engine loop. The returned
/// [`WorkflowRun`] is the freshly-created row (status `Running`); callers poll
/// `workflow_run_get` to observe progress.
pub async fn start_workflow_run(
    ctx: &DispatchCtx,
    config: &Config,
    definition_id: &str,
    input: Value,
    parent_thread_id: Option<String>,
) -> Result<WorkflowRun> {
    log::debug!(
        target: LOG_TARGET,
        "[workflow_run_engine] start.entry definition={definition_id} parent_thread={parent_thread_id:?}"
    );
    let definition = definition_by_id(definition_id)
        .ok_or_else(|| anyhow!("unknown workflow definition: {definition_id}"))?;
    let safety_tier = super::host::admit_workflow(&definition)?;

    let run_id = format!("wfrun-{}", uuid::Uuid::new_v4());
    let initial_engine = WorkflowEngine::new(
        workflow_store(config)?,
        Arc::new(super::host::OpenHumanWorkflowExecutor::new(
            &run_id,
            None,
            safety_tier,
            None,
        )),
    );
    let run = initial_engine
        .initialise(run_id.clone(), &definition, input.clone(), parent_thread_id)
        .context("persist initial workflow run")?;

    let _handle = schedule_workflow_drive(ctx, run_id.clone(), definition, false);

    log::debug!(
        target: LOG_TARGET,
        "[workflow_run_engine] start.spawned run={run_id} phases={}",
        run.phase_states.as_object().map(|m| m.len()).unwrap_or(0)
    );
    Ok(run)
}

/// Signal a running workflow to stop after its current phase.
///
/// Flips the run's cancellation flag (checked by the loop between phases) and
/// eagerly marks the persisted row `Interrupted` so a poller sees the intent
/// immediately even while the in-flight phase drains. Idempotent: stopping a
/// terminal or unknown run is a no-op that returns the current row.
pub async fn stop_workflow_run(config: &Config, id: &str) -> Result<Option<WorkflowRun>> {
    log::debug!(target: LOG_TARGET, "[workflow_run_engine] stop.entry run={id}");
    let Some(run) = get_workflow_run(config, id)? else {
        log::debug!(target: LOG_TARGET, "[workflow_run_engine] stop.unknown run={id}");
        return Ok(None);
    };

    if matches!(
        run.status,
        WorkflowRunStatus::Completed | WorkflowRunStatus::Failed | WorkflowRunStatus::Cancelled
    ) {
        log::debug!(
            target: LOG_TARGET,
            "[workflow_run_engine] stop.already_terminal run={id} status={}",
            run.status.as_str()
        );
        return Ok(Some(run));
    }

    // Capture the exact live generation before fencing its durable row. If a
    // concurrent resume wins instead, this stop's CAS fails and it must not
    // cancel that successor's fresh token.
    let supervisor = workflow_supervisor();
    let handle = supervisor.current_handle(id);

    let mut phase_states = run.phase_states.clone();
    reset_running_phases(
        &mut phase_states,
        "workflow interrupted by host; phase will retry on resume",
    );
    let updated = compare_and_swap_workflow_run_lifecycle(
        config,
        WorkflowRunUpsert {
            id: run.id.clone(),
            definition_id: run.definition_id.clone(),
            parent_thread_id: run.parent_thread_id.clone(),
            input: run.input.clone(),
            phase_states,
            child_run_ids: run.child_run_ids.clone(),
            status: WorkflowRunStatus::Interrupted,
            summary: run.summary.clone(),
            started_at: Some(run.started_at),
            completed_at: None,
        },
        run.revision,
    )
    .context("persist workflow run interrupt")?;
    let updated = updated.ok_or_else(|| {
        anyhow!("workflow run {id} changed while stop was being applied; reload and retry")
    })?;
    if let Some(handle) = handle.as_ref() {
        let _ = supervisor.stop_generation(handle);
    }

    log::debug!(target: LOG_TARGET, "[workflow_run_engine] stop.marked_interrupted run={id}");
    Ok(Some(updated))
}

/// Resume an interrupted (or otherwise incomplete) workflow run.
///
/// Reloads the run, clears any stale cancellation flag, flips the row back to
/// `Running`, and spawns a fresh engine loop. Phases already `completed` in
/// `phase_states` are skipped; the loop continues from the first incomplete
/// phase whose dependencies are satisfied. Returns the run row (now `Running`),
/// or an error if the run is unknown / already terminal-complete / its
/// definition no longer exists.
pub async fn resume_workflow_run(
    ctx: &DispatchCtx,
    config: &Config,
    id: &str,
) -> Result<WorkflowRun> {
    log::debug!(target: LOG_TARGET, "[workflow_run_engine] resume.entry run={id}");
    let run = get_workflow_run(config, id)?.ok_or_else(|| anyhow!("unknown workflow run: {id}"))?;

    if matches!(run.status, WorkflowRunStatus::Completed) {
        return Err(anyhow!("workflow run {id} is already completed"));
    }

    let definition = definition_by_id(&run.definition_id)
        .ok_or_else(|| anyhow!("definition {} no longer exists", run.definition_id))?;
    let _safety_tier = super::host::admit_workflow(&definition)?;

    let resumed = compare_and_swap_workflow_run_lifecycle(
        config,
        WorkflowRunUpsert {
            id: run.id.clone(),
            definition_id: run.definition_id.clone(),
            parent_thread_id: run.parent_thread_id.clone(),
            input: run.input.clone(),
            phase_states: run.phase_states.clone(),
            child_run_ids: run.child_run_ids.clone(),
            status: WorkflowRunStatus::Running,
            summary: run.summary.clone(),
            started_at: Some(run.started_at),
            completed_at: None,
        },
        run.revision,
    )
    .context("persist workflow run resume")?;
    let resumed = resumed.ok_or_else(|| {
        anyhow!("workflow run {id} changed while resume was being applied; reload and retry")
    })?;

    let _handle = schedule_workflow_drive(ctx, id.to_string(), definition, true);

    log::debug!(target: LOG_TARGET, "[workflow_run_engine] resume.spawned run={id}");
    Ok(resumed)
}

/// Drive the phase DAG with the parent context created by the owner's scoped
/// lifecycle. Errors are recorded on the run row (status `Failed`) rather than
/// propagated — there is no caller to receive them.
pub(crate) async fn run_engine_loop(
    config: &Config,
    run_id: &str,
    definition: WorkflowDefinition,
    handle: WorkflowRunHandle,
    cancel_token: tinyagents_harness::CancellationToken,
    parent_override: Option<ParentExecutionContext>,
) {
    let model_override = get_workflow_run(config, run_id)
        .ok()
        .flatten()
        .and_then(|run| {
            run.input
                .get("modelOverride")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .filter(|model| !model.trim().is_empty());
    let safety_tier = match super::host::admit_workflow(&definition) {
        Ok(tier) => tier,
        Err(error) => {
            log::error!(target: LOG_TARGET, "[workflow_run_engine] loop.safety_admission_failed run={run_id} err={error}");
            if workflow_supervisor().is_current(&handle)
                && let Ok(Some(run)) = get_workflow_run(config, run_id)
            {
                let _ = compare_and_swap_workflow_run_lifecycle(
                    config,
                    WorkflowRunUpsert {
                        id: run.id.clone(),
                        definition_id: run.definition_id.clone(),
                        parent_thread_id: run.parent_thread_id.clone(),
                        input: run.input.clone(),
                        phase_states: run.phase_states.clone(),
                        child_run_ids: run.child_run_ids.clone(),
                        status: WorkflowRunStatus::Failed,
                        summary: Some(format!("workflow safety admission failed: {error}")),
                        started_at: Some(run.started_at),
                        completed_at: Some(chrono::Utc::now()),
                    },
                    run.revision,
                );
            }
            return;
        }
    };
    let store = match workflow_store(config) {
        Ok(store) => store,
        Err(error) => {
            log::error!(target: LOG_TARGET, "[workflow_run_engine] loop.no_store run={run_id} err={error:#}");
            return;
        }
    };
    let parent = parent_override.expect("owner lifecycle created the OpenHuman root parent");
    let engine = WorkflowEngine::new(
        store,
        Arc::new(super::host::OpenHumanWorkflowExecutor::new(
            run_id,
            model_override,
            safety_tier,
            Some(parent),
        )),
    )
    .with_event_sink(Arc::new(tinyagents_graph::GraphTracingSink::new(format!(
        "workflow:{run_id}"
    ))));
    let outcome = engine
        .drive(run_id, &definition, cancel_token)
        .await
        .map_err(anyhow::Error::msg);

    if let Err(err) = outcome {
        if !workflow_supervisor().is_current(&handle) {
            log::debug!(
                target: LOG_TARGET,
                "[workflow_run_engine] loop.owner_lost run={run_id}; suppressing stale failure"
            );
            return;
        }
        log::error!(
            target: LOG_TARGET,
            "[workflow_run_engine] loop.failed run={run_id} err={err}"
        );
        // Best-effort terminal failure write, preserving partial phase state.
        if let Ok(Some(run)) = get_workflow_run(config, run_id) {
            if !matches!(
                run.status,
                WorkflowRunStatus::Completed
                    | WorkflowRunStatus::Failed
                    | WorkflowRunStatus::Cancelled
                    | WorkflowRunStatus::Interrupted
            ) {
                if workflow_supervisor().is_current(&handle) {
                    let _ = compare_and_swap_workflow_run_lifecycle(
                    config,
                    WorkflowRunUpsert {
                        id: run.id.clone(),
                        definition_id: run.definition_id.clone(),
                        parent_thread_id: run.parent_thread_id.clone(),
                        input: run.input.clone(),
                        phase_states: run.phase_states.clone(),
                        child_run_ids: run.child_run_ids.clone(),
                        status: WorkflowRunStatus::Failed,
                        summary: Some(format!("engine error: {err}")),
                        started_at: Some(run.started_at),
                        completed_at: Some(chrono::Utc::now()),
                    },
                        run.revision,
                    );
                }
            }
        }
    }

}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
