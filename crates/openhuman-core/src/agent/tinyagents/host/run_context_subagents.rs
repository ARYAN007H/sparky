//! Durable root and child run identity helpers for delegated turns.

use super::OpenHumanRunContext;

/// Allocate a durable-unique root [`RunConfig`](tinyagents_harness::context::RunConfig).
///
/// A root config id is a cross-turn lifecycle identity, not a display label:
/// callers must retain the returned config/id for every boundary belonging to
/// that one root turn. Child configs are derived with [`RunContext::child`],
/// never by reusing a root literal such as `"agent_turn"`.
pub(crate) fn fresh_root_run_config(kind: &str) -> tinyagents_harness::context::RunConfig {
    tinyagents_harness::context::RunConfig::new(format!("{kind}-{}", uuid::Uuid::new_v4()))
}

/// Builds an owned direct child for the neutral subagent lifecycle.
///
/// The durable key is derived from `parent` *before* a child exists, preserving
/// the parent's root id, immediate run id, and thread. `child_config.run_id`
/// must be a durable-unique child execution id; this helper cannot prove global
/// uniqueness for a caller-supplied value. The resulting TinyAgents and
/// OpenHuman cancellation carriers are the same shared token, so either the
/// lifecycle executor or host child observes cancellation across the tree.
pub(crate) fn direct_subagent_child(
    parent: &tinyagents_harness::context::RunContext<OpenHumanRunContext>,
    task_id: impl Into<String>,
    child_config: tinyagents_harness::context::RunConfig,
) -> tinyagents_harness::Result<(
    tinyagents_orchestration::subagent::SubagentTaskKey,
    tinyagents_harness::context::RunContext<OpenHumanRunContext>,
)> {
    let task_id = task_id.into();
    let task_key =
        tinyagents_orchestration::subagent::SubagentTaskKey::from_context(parent, task_id, None);
    let child_data = parent
        .data
        .child()
        .with_cancellation(parent.cancellation.clone());
    let mut child = parent.child(child_config, child_data)?;
    // TinyAgents gives the child a linked `child_token()` of the parent's, so a
    // parent cancel cascades down but cancelling the child (policy timeout,
    // steering) leaves the parent running. Point the host carrier at that same
    // child token; otherwise it keeps the parent's token and the host tools
    // observe (and could trigger) the wrong scope.
    child.data.cancellation = child.cancellation.clone();
    Ok((task_key, child))
}
