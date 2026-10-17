//! Blocking-with-timeout collection of a sub-agent's terminal result, with a
//! durable task-store fallback for a sub-agent this process never registered
//! in-memory (e.g. after a core restart).

use std::path::Path;
use std::time::Duration;

use tinyagents_orchestration::subagent::{WaitError, WaitOutcome, wait_detached_or_durable};

use super::registry::{admit_workspace, owner_key, registry};

/// Block until `task_id` reaches a terminal status or `timeout` elapses.
pub(crate) async fn wait(
    ctx: &crate::core::runtime::DispatchCtx,
    task_id: &str,
    parent_session: &str,
    timeout: Duration,
) -> Result<WaitOutcome, WaitError> {
    let workspace_dir = ctx
        .resolved()
        .and_then(|context| context.workspace_dir().ok())
        .ok_or(WaitError::NotOwned)?;
    let owner = owner_key(ctx, parent_session)?;
    wait_detached_or_durable(
        registry(),
        super::task_ledger::task_store_for_workspace(&workspace_dir).as_ref(),
        &super::super::background_completions::router_for_workspace(&workspace_dir),
        task_id,
        &owner,
        timeout,
    )
    .await
}

pub(crate) async fn wait_in_workspace(
    ctx: &crate::core::runtime::DispatchCtx,
    task_id: &str,
    parent_session: &str,
    workspace_dir: &Path,
    timeout: Duration,
) -> Result<WaitOutcome, WaitError> {
    admit_workspace(ctx, workspace_dir)?;
    let owner = owner_key(ctx, parent_session)?;
    wait_detached_or_durable(
        registry(),
        super::task_ledger::task_store_for_workspace(workspace_dir).as_ref(),
        &super::super::background_completions::router_for_workspace(workspace_dir),
        task_id,
        &owner,
        timeout,
    )
    .await
}
