//! Resolving a durable `subagent_session_id` (or a transient `task_id`) back
//! to the live registry entry or, failing that, the durable task-store record
//! — used by tool/control paths that only know the stable session id a
//! sub-agent is addressed by.

use std::path::Path;

use tinyagents_orchestration::subagent::{
    resume_ref_from_record, subagent_record_for_task, task_id_for_session_in_records,
    SubagentResumeRef, WaitError,
};

use super::registry::admit_workspace;
use super::task_ledger::task_store_for_workspace;

/// Resolve a durable `subagent_session_id` to the currently-running transient
/// `task_id`, enforcing parent-session ownership.
pub(crate) fn task_id_for_session(
    ctx: &crate::core::runtime::DispatchCtx,
    subagent_session_id: &str,
    parent_session: &str,
) -> Result<String, WaitError> {
    let context = ctx.resolved().ok_or(WaitError::NotOwned)?;
    let workspace = context.workspace_dir().map_err(|_| WaitError::NotOwned)?;
    task_id_for_session_in_workspace(ctx, subagent_session_id, parent_session, &workspace)
}

pub(crate) fn task_id_for_session_in_workspace(
    ctx: &crate::core::runtime::DispatchCtx,
    subagent_session_id: &str,
    parent_session: &str,
    workspace_dir: &Path,
) -> Result<String, WaitError> {
    admit_workspace(ctx, workspace_dir)?;
    let store = scoped_task_store(ctx, workspace_dir)?;
    let task_id = task_id_for_session_in_records(
        tinyagents_orchestration::subagent::list_subagent_records(store.as_ref()),
        subagent_session_id,
        parent_session,
    )?;
    log::debug!(
        "[running_subagents] resolved session from task store subagent_session_id={} task_id={} workspace_dir={}",
        subagent_session_id,
        task_id,
        workspace_dir.display()
    );
    Ok(task_id)
}

pub(crate) fn resume_ref_for_task(
    ctx: &crate::core::runtime::DispatchCtx,
    task_id: &str,
    parent_session: &str,
) -> Result<SubagentResumeRef, WaitError> {
    let context = ctx.resolved().ok_or(WaitError::NotOwned)?;
    let workspace = context.workspace_dir().map_err(|_| WaitError::NotOwned)?;
    resume_ref_for_task_in_workspace(ctx, task_id, parent_session, &workspace)
}

pub(crate) fn resume_ref_for_task_in_workspace(
    ctx: &crate::core::runtime::DispatchCtx,
    task_id: &str,
    parent_session: &str,
    workspace_dir: &Path,
) -> Result<SubagentResumeRef, WaitError> {
    admit_workspace(ctx, workspace_dir)?;
    let store = scoped_task_store(ctx, workspace_dir)?;
    let record = subagent_record_for_task(store.as_ref(), task_id, parent_session)?;
    let reference = resume_ref_from_record(task_id, &record);
    log::debug!(
        "[running_subagents] resolved resume ref from task store task_id={} workspace_dir={}",
        task_id,
        workspace_dir.display()
    );
    Ok(reference)
}

pub(super) fn scoped_task_store(
    ctx: &crate::core::runtime::DispatchCtx,
    workspace_dir: &Path,
) -> Result<std::sync::Arc<dyn tinyagents_tasks::TaskStore>, WaitError> {
    if let Some(context) = ctx.resolved() {
        return Ok(crate::core::runtime::CoreContext::sync_scope(
            context,
            || task_store_for_workspace(workspace_dir),
        ));
    }
    if crate::core::runtime::is_saas() {
        return Err(WaitError::NotOwned);
    }
    Ok(task_store_for_workspace(workspace_dir))
}
