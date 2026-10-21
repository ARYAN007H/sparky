//! Map effective OpenHuman runtime reasoning settings to TinyInference config.
//!
//! Per-model settings take precedence over the global setting, which takes
//! precedence over `reasoning_enabled = false`. A thread's explicit choice is
//! applied by the TinyAgents host adapter before this mapping is consulted.

use tinyinference_llm::model::{ReasoningConfig, ReasoningEffort};

use crate::config::Config;

/// The config-level reasoning choice for the selected model, or `None` to
/// leave reasoning to the provider.
pub(crate) fn reasoning_for_config(config: &Config) -> Option<ReasoningConfig> {
    let per_model = config
        .default_model
        .as_deref()
        .and_then(|model| config.runtime.reasoning_effort_by_model.get(model.trim()));
    if let Some(raw) = per_model
        .or(config.runtime.reasoning_effort.as_ref())
        .map(|raw| raw.trim())
        .filter(|raw| !raw.is_empty())
    {
        return match ReasoningEffort::parse(raw) {
            Some(effort) => Some(ReasoningConfig::effort(effort)),
            None => {
                log::warn!(
                    "[agent][reasoning] ignoring unknown runtime.reasoning_effort {raw:?}; \
                     using the provider default"
                );
                None
            }
        };
    }
    if config.runtime.reasoning_enabled == Some(false) {
        return Some(ReasoningConfig::effort(ReasoningEffort::None));
    }
    None
}

/// A thread preference takes precedence over the selected model's config.
pub(crate) fn turn_reasoning(
    dispatch: &crate::core::runtime::DispatchCtx,
    thread_id: Option<&str>,
    config: Option<&Config>,
) -> Option<ReasoningConfig> {
    if let Some(effort) = thread_id.and_then(|thread_id| {
        crate::agent::tinyagents::thread_reasoning::thread_effort(dispatch, thread_id)
    }) {
        return Some(ReasoningConfig::effort(effort));
    }
    config.and_then(reasoning_for_config)
}

/// Reasoning configuration for one root turn. The store snapshots an explicit
/// thread choice once; subsequent updates apply to later turns only.
pub(crate) fn turn_reasoning_for(
    ctx: &crate::agent::tinyagents::host::OpenHumanRunContext,
    config: Option<&Config>,
    max_output_tokens: Option<u32>,
) -> Option<ReasoningConfig> {
    if ctx.spawn_depth > 0 {
        return None;
    }

    let dispatch = ctx.dispatch_ctx();
    let reasoning = if let Some(thread_id) = ctx.thread_id.as_deref() {
        crate::agent::tinyagents::thread_reasoning::store(&dispatch)
            .reasoning_for_turn(
                Some(&tinyagents_harness::ids::ThreadId::new(thread_id)),
                max_output_tokens,
            )
    } else {
        None
    }
    .or_else(|| {
        reasoning_for_config(config?)
            .map(|reasoning| reasoning.with_output_room(max_output_tokens))
    });

    if let Some(reasoning) = reasoning.as_ref() {
        log::debug!(
            "[agent][reasoning] turn reasoning effort={:?} budget_tokens={:?} thread_id={:?}",
            reasoning.effort,
            reasoning.budget_tokens,
            ctx.thread_id
        );
    }
    reasoning
}

#[cfg(test)]
#[path = "reasoning_tests.rs"]
mod tests;
