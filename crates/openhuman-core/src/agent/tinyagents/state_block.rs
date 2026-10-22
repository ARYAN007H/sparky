//! The run-state block for OpenHuman turns.
//!
//! TinyAgents' [`StateBlockMiddleware`] adds one block to every model request
//! of the turn: the thread goal, the `todo` plan and facts derived from the
//! turn's own tool calls. It is request-only (never on the transcript) and
//! emits no events, so the UI keeps showing only the goal chip.
//!
//! It is installed on root orchestrator turns, the scope of the requirements
//! check. On those turns the turn start does not prepend the goal to the user's
//! message (`session_host::runtime_session::prelude`): the block shows the goal
//! once, current on every call, and a paused goal comes with the choice to
//! resume it, replace it or leave it, which the model makes with the goal
//! tools after reading the message.

use std::sync::Arc;

use tinyagents_harness::runtime::AgentHarness;
use tinyagents_harness::store::Store;
use tinyagents_harness_tools::state_block::StateBlockMiddleware;

/// Whether a turn gets the run-state block.
pub(crate) fn applies(is_subagent: bool, agent_definition_id: Option<&str>) -> bool {
    super::verify_before_finish::applies(is_subagent, agent_definition_id)
}

/// Installs the block for a turn in scope that has the session store the goal
/// and `todo` tools write, with the key the `todo` tool falls back to when the
/// turn has no thread (the session id).
pub(super) fn install<C: Send + Sync + 'static>(
    harness: &mut AgentHarness<(), C>,
    source: Option<(Arc<dyn Store>, String)>,
    is_subagent: bool,
    agent_definition_id: Option<&str>,
) {
    match source {
        Some((store, fallback_thread)) if applies(is_subagent, agent_definition_id) => {
            harness.push_middleware(Arc::new(
                StateBlockMiddleware::new(store).with_fallback_thread(fallback_thread),
            ));
        }
        _ => tracing::debug!(
            is_subagent,
            agent = agent_definition_id.unwrap_or("<none>"),
            "[state_block] not installed for this turn"
        ),
    }
}

#[cfg(test)]
#[path = "state_block_tests.rs"]
mod tests;
