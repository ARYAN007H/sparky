//! Product-to-harness configuration for one hosted chat turn.

use std::{collections::HashSet, sync::Arc};

use tinyagents_session::transcript::TranscriptMessage;
use tinytools::Tool;

use crate::agent::tinyagents::{host::OpenHumanRunContext, TurnModels, TurnSpec};

/// Map the session's request-scoped execution inputs onto TinyAgents' one
/// hosted turn specification. The driver still owns authority validation and
/// invokes `run_hosted` inside its cancellation and snapshot boundary.
#[allow(clippy::too_many_arguments)]
pub(super) fn build_chat_turn_spec<'a>(
    run_context: OpenHumanRunContext,
    turn_models: TurnModels,
    model: &'a str,
    messages: Vec<TranscriptMessage>,
    tools: Arc<Vec<Box<dyn Tool>>>,
    synthesized_tools: Arc<Vec<Box<dyn Tool>>>,
    visible_tool_names: HashSet<String>,
    max_iterations: usize,
    context_window: Option<u64>,
    run_queue: Option<
        Arc<tinyagents_harness::run_queue::RunQueue<crate::agent::queued_turn::QueuedTurn>>,
    >,
    context_mw: crate::agent::tinyagents::middleware::turn_context::TurnContextMiddleware,
    tool_policy: crate::agent::tinyagents::ToolPolicyEnforcement,
) -> TurnSpec<'a> {
    let provider_id = turn_models.provider_id().to_string();
    TurnSpec {
        run_context,
        turn_models,
        provider_id,
        model,
        history: messages,
        // Durable and synthesized registrations follow the same order as the
        // runtime snapshot, so advertised names resolve to matching instances.
        tool_sets: vec![tools, synthesized_tools],
        // `Some(empty)` is authoritative deny-all; `None` exposes the
        // construction-time registry.
        allowed: Some(visible_tool_names),
        max_iterations,
        subagent_scope: None,
        context_window,
        run_queue,
        message_run_queue: None,
        early_exit_tools: &["ask_user_clarification"],
        pause_at_cap: true,
        max_output_tokens: Some(crate::inference::provider::AGENT_TURN_MAX_OUTPUT_TOKENS),
        context_mw,
        tool_policy: Some(tool_policy),
        deterministic_cacheable: false,
        defer_turn_completed_to_caller: true,
    }
}
