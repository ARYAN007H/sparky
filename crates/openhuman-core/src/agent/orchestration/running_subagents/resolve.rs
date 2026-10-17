//! Resolving a durable `subagent_session_id` (or a transient `task_id`) back
//! to the live registry entry or, failing that, the durable task-store record
//! — used by tool/control paths that only know the stable session id a
//! sub-agent is addressed by.

use std::path::Path;

use tinyagents_orchestration::subagent::{
    SubagentResumeRef, WaitError, resolve_detached_or_durable,
};

use super::registry::{admit_workspace, owner_key, registry};
use super::task_ledger::task_store_for_workspace;

/// Resolve a durable `subagent_session_id` to the currently-running transient
/// `task_id`, enforcing parent-session ownership.
pub(crate) fn task_id_for_session(
    ctx: &crate::core::runtime::DispatchCtx,
    subagent_session_id: &str,
    parent_session: &str,
) -> Result<String, WaitError> {
    let workspace = ctx
        .resolved()
        .and_then(|context| context.workspace_dir().ok())
        .ok_or(WaitError::NotOwned)?;
    task_id_for_session_in_workspace(ctx, subagent_session_id, parent_session, &workspace)
}

pub(crate) fn task_id_for_session_in_workspace(
    ctx: &crate::core::runtime::DispatchCtx,
    subagent_session_id: &str,
    parent_session: &str,
    workspace_dir: &Path,
) -> Result<String, WaitError> {
    admit_workspace(ctx, workspace_dir)?;
    let resolved = resolve_detached_or_durable(
        registry(),
        task_store_for_workspace(workspace_dir).as_ref(),
        subagent_session_id,
        &owner_key(ctx, parent_session)?,
    )?;
    let task_id = resolved.task_id;
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
    let workspace = ctx
        .resolved()
        .and_then(|context| context.workspace_dir().ok())
        .ok_or(WaitError::NotOwned)?;
    resume_ref_for_task_in_workspace(ctx, task_id, parent_session, &workspace)
}

pub(crate) fn resume_ref_for_task_in_workspace(
    ctx: &crate::core::runtime::DispatchCtx,
    task_id: &str,
    parent_session: &str,
    workspace_dir: &Path,
) -> Result<SubagentResumeRef, WaitError> {
    admit_workspace(ctx, workspace_dir)?;
    let reference = resolve_detached_or_durable(
        registry(),
        task_store_for_workspace(workspace_dir).as_ref(),
        task_id,
        &owner_key(ctx, parent_session)?,
    )?;
    log::debug!(
        "[running_subagents] resolved resume ref from task store task_id={} workspace_dir={}",
        task_id,
        workspace_dir.display()
    );
    Ok(reference)
}
