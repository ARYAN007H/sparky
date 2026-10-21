//! OpenHuman's adapter for TinyAgents' caller-owned thread reasoning store.

use std::sync::Arc;

use tinyagents_harness::ids::ThreadId;
use tinyagents_harness::thread_reasoning::ThreadReasoningStore;
use tinyinference_llm::model::ReasoningEffort;

use crate::core::runtime::DispatchCtx;

/// The reasoning store belongs to this dispatch context/profile. Context
/// clones share the slot; separately derived profiles remain isolated.
pub(crate) fn store(ctx: &DispatchCtx) -> Arc<ThreadReasoningStore> {
    crate::core::runtime::slot_for::<ThreadReasoningStore>(ctx)
}

/// Applies a chat request's optional reasoning choice, preserving the wire
/// error text used by existing clients for an unrecognized effort.
pub(crate) fn apply_requested_effort(
    ctx: &DispatchCtx,
    thread_id: &str,
    raw: Option<&str>,
) -> Result<(), String> {
    let Some(raw) = raw else {
        return Ok(());
    };
    let trimmed = raw.trim();
    let requested = if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("default")
        || trimmed.eq_ignore_ascii_case("auto")
    {
        None
    } else {
        Some(ReasoningEffort::parse(trimmed)
            .ok_or_else(|| format!("unknown reasoning_effort '{trimmed}'"))?)
    };

    let thread_id = ThreadId::new(thread_id);
    let store = store(ctx);
    let previous = match requested {
        Some(effort) => store.set_effort(&thread_id, effort),
        None => store.clear(&thread_id),
    };
    if previous != requested {
        log::info!(
            "[agent][reasoning] thread reasoning effort changed thread_id={} {:?} -> {:?}",
            thread_id.as_str(),
            previous.map(ReasoningEffort::as_str),
            requested.map(ReasoningEffort::as_str)
        );
    }
    Ok(())
}

#[cfg(test)]
#[path = "thread_reasoning_tests.rs"]
mod tests;
