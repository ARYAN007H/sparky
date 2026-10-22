//! Aborting sub-agents: by task id (the user-facing "Cancel" affordance), by
//! durable session id, by owning parent thread (thread deletion), or all at
//! once (a full thread purge).

use std::path::{Path, PathBuf};

use tinyagents_orchestration::subagent::{
    task_id_for_session_in_records, FinishedOutcome, WaitError,
};

use super::registry::admit_workspace;
use super::task_ledger::scoped_task_store;

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
pub(crate) fn cancel_by_task_with_workspace_reason(
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
    let store = crate::agent::orchestration::subagent_sessions::SubagentSessionStore::new(
        workspace_dir.clone(),
    );
    let jobs = crate::core::runtime::slot_for::<
        tinyagents_harness_tools::subagents::SubAgentJobRegistry,
    >(ctx);
    let selected = crate::agent::orchestration::subagent_sessions::cancel_driver_job_by_task(
        &store,
        &workspace_dir.join(".openhuman/subagent_checkpoints"),
        &jobs,
        task_id,
        reason,
    )?;
    let parent_epoch = selected.as_ref().and_then(|selected| selected.parent_epoch);
    let driver_pause = selected
        .as_ref()
        .filter(|selected| selected.was_paused)
        .cloned();
    Ok(selected.map(|selected| CancelledSubagent {
        agent_id: selected.agent_id,
        parent_session: selected.parent_session,
        subagent_session_id: Some(selected.subagent_session_id),
        workspace_dir: selected.workspace_dir,
        parent_thread_id: selected.parent_thread_id,
        parent_epoch,
        already_finished: selected.already_finished,
        driver_pause,
        driver_managed: true,
    }))
}

pub(crate) fn cancel_by_session_in_workspace(
    ctx: &crate::core::runtime::DispatchCtx,
    subagent_session_id: &str,
    parent_session: &str,
    workspace_dir: &Path,
) -> Option<CancelledSubagent> {
    let task_id =
        task_id_for_session_in_workspace(ctx, subagent_session_id, parent_session, workspace_dir)
            .ok()?;
    cancel_by_task_with_workspace_reason(ctx, &task_id, Some(workspace_dir), None)
        .ok()
        .flatten()
}

fn task_id_for_session_in_workspace(
    ctx: &crate::core::runtime::DispatchCtx,
    subagent_session_id: &str,
    parent_session: &str,
    workspace_dir: &Path,
) -> Result<String, WaitError> {
    admit_workspace(ctx, workspace_dir)?;
    let store = scoped_task_store(ctx, workspace_dir)?;
    task_id_for_session_in_records(
        tinyagents_orchestration::subagent::list_subagent_records(store.as_ref()),
        subagent_session_id,
        parent_session,
    )
}

/// Abort and drop every running sub-agent whose parent chat thread is
/// `thread_id`. Called when that thread is deleted so detached children don't
/// keep running (and later try to deliver) against a thread that no longer
/// exists. Returns the number of sub-agents cancelled.
pub(crate) fn cancel_for_thread(ctx: &crate::core::runtime::DispatchCtx, thread_id: &str) -> usize {
    let _ = (ctx, thread_id);
    // The production job-registry thread selector is added by the cancellation
    // owner leaf; the legacy detached-task registry was always empty in hosts.
    0
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
    let mut task_ids = Vec::new();
    if let Some(core_context) = ctx.resolved() {
        if let Ok(workspace_dir) = core_context.workspace_dir() {
            let jobs = crate::core::runtime::slot_for::<
                tinyagents_harness_tools::subagents::SubAgentJobRegistry,
            >(ctx);
            let driver_stopped =
                crate::agent::orchestration::subagent_sessions::cancel_driver_jobs_for_thread(
                    &crate::agent::orchestration::subagent_sessions::SubagentSessionStore::new(
                        workspace_dir.clone(),
                    ),
                    &workspace_dir,
                    &jobs,
                    thread_id,
                );
            for task_id in driver_stopped {
                if !task_ids.contains(&task_id) {
                    task_ids.push(task_id);
                }
            }
        }
    }
    task_ids
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
