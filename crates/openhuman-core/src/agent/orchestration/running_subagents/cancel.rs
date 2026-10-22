//! Aborting sub-agents: by task id (the user-facing "Cancel" affordance), by
//! durable session id, by owning parent thread (thread deletion), or all at
//! once (a full thread purge).

use std::path::{Path, PathBuf};

use tinyagents_orchestration::subagent::{FinishedOutcome, ParentJobScope, SubAgentJobStatus};

/// Metadata captured when a sub-agent is cancelled, so the caller can surface
/// the cancellation back in the parent chat (record a "cancelled" completion
/// for idle-gated delivery).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CancelledSubagent {
    pub(crate) agent_id: String,
    pub(crate) parent_session: String,
    pub(crate) subagent_session_id: Option<String>,
    pub(crate) workspace_dir: PathBuf,
    pub(crate) parent_thread_id: Option<String>,
    /// Admission generation captured for this child, if it originated in a
    /// thread. Completion after a later stop/resume must retain this epoch.
    pub(crate) parent_epoch: Option<u64>,
    /// How the run had already ended when the cancel arrived, if it had.
    pub(crate) already_finished: Option<FinishedOutcome>,
    /// The selected durable driver row when a paused child needs host-side
    /// terminal projection (its driver callback already returned on pause).
    pub(crate) driver_pause:
        Option<crate::agent::orchestration::subagent_sessions::DriverTaskCancellation>,
    /// TinyAgents' awaited completion handler owns terminal delivery for this
    /// driver-backed child; the RPC handler must not publish a duplicate.
    pub(crate) driver_managed: bool,
}

/// Cancel with an optional user-provided reason for the driver-owned
/// completion callback. Store failures are returned so the RPC does not report
/// an unknown task when its durable selector could not be read. TinyAgents' job
/// registry is the live authority; the former host detached-task table was
/// never populated by production spawns.
pub(crate) async fn cancel_by_task_with_workspace_reason(
    ctx: &crate::core::runtime::DispatchCtx,
    task_id: &str,
    fallback_workspace: Option<&Path>,
    reason: Option<&str>,
) -> Result<Option<CancelledSubagent>, String> {
    let workspace_dir = match ctx.resolved() {
        Some(context) => context.workspace_dir().ok(),
        None if !crate::core::runtime::is_saas() => fallback_workspace.map(Path::to_path_buf),
        None => None,
    };
    let Some(workspace_dir) = workspace_dir else {
        return Ok(None);
    };
    let Some(owner_scope) =
        crate::agent::orchestration::completion_owners::driver_task_owner(ctx, task_id)
    else {
        return terminal_legacy_session(ctx, &workspace_dir, task_id).await;
    };
    let Some(parent_thread_id) = owner_scope.parent_thread_id.as_deref() else {
        return Ok(None);
    };
    if owner_scope.workspace_dir != workspace_dir {
        return Ok(None);
    }
    let owner = crate::agent::tinyagents::tool_registration::subagent_owner_key(
        &owner_scope.workspace_dir,
        &owner_scope.parent_session,
        Some(parent_thread_id),
    );
    let scope = ParentJobScope::new(owner_scope.workspace.clone(), parent_thread_id);
    let jobs = crate::core::runtime::slot_for::<
        tinyagents_harness_tools::subagents::SubAgentJobRegistry,
    >(ctx);
    let job = jobs
        .list_for_parent_scope(&owner, &scope)
        .map_err(|error| format!("failed to list owned driver jobs: {error}"))?
        .into_iter()
        .find(|job| {
            job.extensions
                .get("task_id")
                .and_then(serde_json::Value::as_str)
                == Some(task_id)
        });
    let session_store = crate::agent::orchestration::subagent_sessions::open_for_context(
        ctx,
        owner_scope.workspace_dir.clone(),
        owner_scope.parent_session.clone(),
        owner_scope.parent_thread_id.clone(),
        Some((*jobs).clone()),
    )
    .map_err(|error| format!("failed to open owner-scoped driver sessions: {error}"))?;
    let session = session_store
        .list_for_parent(&owner_scope.parent_session, Some(parent_thread_id))
        .await
        .map_err(|error| format!("failed to load owner-scoped driver sessions: {error}"))?
        .into_iter()
        .find(|session| session.current_task_id.as_deref() == Some(task_id));
    let Some(session) = session else {
        return Ok(None);
    };
    let parent_epoch = crate::agent::orchestration::subagent_sessions::parent_epoch_for_context(
        ctx,
        &owner_scope.workspace_dir,
        session.subagent_session_id.clone(),
    )
    .map_err(|error| format!("failed to load parent epoch: {error}"))?;
    let typed_finished = || -> Result<Option<FinishedOutcome>, String> {
        let Some(key) = session.lifecycle_key.as_ref() else {
            return Ok(None);
        };
        let Some(outcome) =
            crate::agent::subagent_host::load_terminal_outcome(&owner_scope.workspace_dir, key)?
        else {
            return Ok(None);
        };
        Ok(match outcome.status {
            tinyagents_orchestration::subagent::SubagentOutcomeKind::Completed => {
                Some(FinishedOutcome::Completed)
            }
            tinyagents_orchestration::subagent::SubagentOutcomeKind::Incomplete(_) => {
                Some(FinishedOutcome::Failed)
            }
            tinyagents_orchestration::subagent::SubagentOutcomeKind::Cancelled
            | tinyagents_orchestration::subagent::SubagentOutcomeKind::AwaitingInput(_) => None,
        })
    };
    let already_finished = match session.status {
        tinyagents_orchestration::subagent::DurableSubagentStatus::Idle => typed_finished()?,
        tinyagents_orchestration::subagent::DurableSubagentStatus::Failed => {
            match typed_finished()? {
                Some(finished) => Some(finished),
                None if session.lifecycle_key.is_none() => Some(FinishedOutcome::Failed),
                None => None,
            }
        }
        tinyagents_orchestration::subagent::DurableSubagentStatus::Running
        | tinyagents_orchestration::subagent::DurableSubagentStatus::AwaitingUser
        | tinyagents_orchestration::subagent::DurableSubagentStatus::Closed => None,
    };
    let was_paused =
        session.status == tinyagents_orchestration::subagent::DurableSubagentStatus::AwaitingUser;
    if already_finished.is_none() {
        let Some(job) = job else {
            return Ok(None);
        };
        if let Some(reason) = reason {
            let reason = reason.trim();
            if !reason.is_empty() {
                crate::agent::orchestration::completion_owners::note_driver_cancel_reason(
                    ctx, task_id, reason,
                );
            }
        }
        if jobs.cancel_owned(job.id.as_str(), &owner).is_err() {
            crate::agent::orchestration::completion_owners::clear_driver_cancel_reason(
                ctx, task_id,
            );
            return Ok(None);
        }
    }
    let selected = crate::agent::orchestration::subagent_sessions::DriverTaskCancellation {
        task_id: task_id.to_owned(),
        agent_id: session.agent_id,
        parent_session: owner_scope.parent_session,
        subagent_session_id: session.subagent_session_id,
        workspace_dir: owner_scope.workspace_dir,
        parent_thread_id: owner_scope.parent_thread_id,
        parent_epoch,
        already_finished,
        was_paused,
        lifecycle_key: session.lifecycle_key,
    };
    let driver_pause = was_paused.then_some(selected.clone());
    Ok(Some(CancelledSubagent {
        agent_id: selected.agent_id,
        parent_session: selected.parent_session,
        subagent_session_id: Some(selected.subagent_session_id),
        workspace_dir: selected.workspace_dir,
        parent_thread_id: selected.parent_thread_id,
        parent_epoch: selected.parent_epoch,
        already_finished: selected.already_finished,
        driver_pause,
        driver_managed: true,
    }))
}

/// Preserve late-cancel reporting for durable terminal sessions that predate
/// the in-memory TinyAgents owner index. A narrow migration adapter locates a
/// terminal host row, imports it into the scoped driver store, and returns the
/// driver's durable projection; live cancellation still requires the owner
/// registry.
async fn terminal_legacy_session(
    ctx: &crate::core::runtime::DispatchCtx,
    workspace_dir: &Path,
    task_id: &str,
) -> Result<Option<CancelledSubagent>, String> {
    let Some(session) =
        crate::agent::orchestration::subagent_sessions::import_terminal_task_for_context(
            ctx,
            workspace_dir.to_path_buf(),
            task_id,
        )
        .await
        .map_err(|error| format!("failed to read terminal legacy subagent session: {error}"))?
    else {
        return Ok(None);
    };
    let already_finished = match session.status {
        tinyagents_orchestration::subagent::DurableSubagentStatus::Failed
            if session.lifecycle_key.is_none() =>
        {
            Some(FinishedOutcome::Failed)
        }
        tinyagents_orchestration::subagent::DurableSubagentStatus::Idle => {
            let Some(key) = session.lifecycle_key.as_ref() else {
                return Ok(None);
            };
            let Some(outcome) =
                crate::agent::subagent_host::load_terminal_outcome(workspace_dir, key)?
            else {
                return Ok(None);
            };
            match outcome.status {
                tinyagents_orchestration::subagent::SubagentOutcomeKind::Completed => {
                    Some(FinishedOutcome::Completed)
                }
                tinyagents_orchestration::subagent::SubagentOutcomeKind::Incomplete(_) => {
                    Some(FinishedOutcome::Failed)
                }
                tinyagents_orchestration::subagent::SubagentOutcomeKind::Cancelled
                | tinyagents_orchestration::subagent::SubagentOutcomeKind::AwaitingInput(_) => None,
            }
        }
        tinyagents_orchestration::subagent::DurableSubagentStatus::Running
        | tinyagents_orchestration::subagent::DurableSubagentStatus::AwaitingUser
        | tinyagents_orchestration::subagent::DurableSubagentStatus::Failed
        | tinyagents_orchestration::subagent::DurableSubagentStatus::Closed => None,
    };
    let Some(already_finished) = already_finished else {
        return Ok(None);
    };
    Ok(Some(CancelledSubagent {
        agent_id: session.agent_id,
        parent_session: session.parent_session,
        subagent_session_id: Some(session.subagent_session_id),
        workspace_dir: workspace_dir.to_path_buf(),
        parent_thread_id: session.parent_thread_id,
        parent_epoch: None,
        already_finished: Some(already_finished),
        driver_pause: None,
        driver_managed: false,
    }))
}

pub(crate) async fn cancel_by_session_in_workspace(
    ctx: &crate::core::runtime::DispatchCtx,
    subagent_session_id: &str,
    parent_session: &str,
    workspace_dir: &Path,
) -> Option<CancelledSubagent> {
    let owners = crate::agent::orchestration::completion_owners::driver_tasks_for_parent(
        ctx,
        workspace_dir,
        parent_session,
    );
    let mut task_id = None;
    for (candidate, owner) in owners {
        let sessions = crate::agent::orchestration::subagent_sessions::open_for_context(
            ctx,
            workspace_dir.to_path_buf(),
            parent_session,
            owner.parent_thread_id.clone(),
            None,
        )
        .ok()?
        .list_for_parent(parent_session, owner.parent_thread_id.as_deref())
        .await
        .ok()?;
        if sessions.iter().any(|session| {
            session.subagent_session_id == subagent_session_id
                && session.current_task_id.as_deref() == Some(candidate.as_str())
        }) {
            task_id = Some(candidate);
            break;
        }
    }
    let task_id = task_id?;
    cancel_by_task_with_workspace_reason(ctx, &task_id, Some(workspace_dir), None)
        .await
        .ok()
        .flatten()
}

/// Abort and drop every running sub-agent whose parent chat thread is
/// `thread_id`. Called when that thread is deleted so detached children don't
/// keep running (and later try to deliver) against a thread that no longer
/// exists. Returns the number of sub-agents cancelled.
pub(crate) fn cancel_for_thread(ctx: &crate::core::runtime::DispatchCtx, thread_id: &str) -> usize {
    let Some(core_context) = ctx.resolved() else {
        return 0;
    };
    let Ok(workspace_dir) = core_context.workspace_dir() else {
        return 0;
    };
    let scopes = crate::agent::orchestration::completion_owners::driver_tasks_for_thread(
        ctx,
        &workspace_dir,
        thread_id,
    );
    let jobs = crate::core::runtime::slot_for::<
        tinyagents_harness_tools::subagents::SubAgentJobRegistry,
    >(ctx);
    let mut cancelled = 0;
    let mut scopes_seen = Vec::new();
    for (_, owner) in scopes {
        let scope = ParentJobScope::new(owner.workspace, thread_id);
        if scopes_seen.contains(&scope) {
            continue;
        }
        scopes_seen.push(scope.clone());
        match jobs.cancel_for_thread(&scope) {
            Ok(count) => cancelled += count,
            Err(error) => log::warn!(
                "[running_subagents] driver thread deletion cancellation failed: {error}"
            ),
        }
    }
    cancelled
}

/// Abort every running detached sub-agent spawned from chat thread
/// `thread_id` because the user pressed Stop on that thread.
///
/// Unlike [`cancel_for_thread`] (thread deletion) the thread survives, so each
/// child's durable sub-agent session is marked failed ("cancelled by user")
/// rather than left looking resumable. No "you cancelled" completion is
/// recorded: delivering one would start a fresh system turn on the thread,
/// which is exactly what Stop is meant to prevent. Returns the cancelled task
/// ids.
pub(crate) fn stop_for_thread(
    ctx: &crate::core::runtime::DispatchCtx,
    thread_id: &str,
) -> Vec<String> {
    if let Some(core_context) = ctx.resolved() {
        if let Ok(workspace_dir) = core_context.workspace_dir() {
            let jobs = crate::core::runtime::slot_for::<
                tinyagents_harness_tools::subagents::SubAgentJobRegistry,
            >(ctx);
            let scopes = crate::agent::orchestration::completion_owners::driver_tasks_for_thread(
                ctx,
                &workspace_dir,
                thread_id,
            );
            let mut task_ids = Vec::new();
            let mut stopped_scopes = Vec::new();
            for (task_id, owner_scope) in scopes {
                let owner = crate::agent::tinyagents::tool_registration::subagent_owner_key(
                    &owner_scope.workspace_dir,
                    &owner_scope.parent_session,
                    owner_scope.parent_thread_id.as_deref(),
                );
                let Some(parent_thread_id) = owner_scope.parent_thread_id.as_deref() else {
                    continue;
                };
                let scope = ParentJobScope::new(owner_scope.workspace.clone(), parent_thread_id);
                let has_live_task =
                    jobs.list_for_parent_scope(&owner, &scope)
                        .ok()
                        .is_some_and(|jobs| {
                            jobs.iter().any(|job| {
                                job.extensions
                                    .get("task_id")
                                    .and_then(serde_json::Value::as_str)
                                    == Some(task_id.as_str())
                                    && matches!(
                                        job.status,
                                        SubAgentJobStatus::Queued
                                            | SubAgentJobStatus::Running
                                            | SubAgentJobStatus::AwaitingInput
                                    )
                            })
                        });
                if has_live_task {
                    task_ids.push(task_id);
                    if !stopped_scopes.contains(&scope) {
                        stopped_scopes.push(scope);
                    }
                }
            }
            for scope in stopped_scopes {
                if let Err(error) = jobs.cancel_for_thread(&scope) {
                    log::warn!("[running_subagents] driver thread stop failed: {error}");
                }
            }
            task_ids.sort();
            task_ids.dedup();
            return task_ids;
        }
    }
    Vec::new()
}

/// Abort and drop **every** registered sub-agent. Called on a full thread purge
/// where no parent thread survives. Returns the **distinct parent thread ids**
/// that had sub-agents, so the purge path can tombstone them in
/// [`super::super::background_completions`] and drop any straggler completion
/// that wins the cooperative-abort race. Headless sub-agents (no parent thread)
/// are still aborted but contribute no id.
pub(crate) fn cancel_all(ctx: &crate::core::runtime::DispatchCtx) -> Vec<String> {
    let _ = ctx;
    // Known legacy limitation tracked by #216: full purge still does not
    // enumerate upstream driver jobs. Keep this behavior explicit here.
    Vec::new()
}

pub(crate) fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
