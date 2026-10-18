//! `AgentProgress` -> canonical `ChannelOutputEvent`.
//!
//! The one place a turn's progress becomes the shared output vocabulary. The
//! web adapter (`tinychannels-runtime`) projects it onto the web wire, so this
//! file builds no `WebChannelEvent`. Payload caps are applied here, before the
//! projection, so a multi-megabyte tool result never reaches the socket.

use crate::agent::subagent_host::SubagentUsageExt as _;
use serde::Serialize;
use serde_json::Value;
use tinychannels_bus::module_progressive::{
    ChannelOutputEvent, SubagentOutput, SubagentUsage, ToolFailureSummary,
};
use tinychannels_bus::web::{cap_web_args, cap_web_output};

use crate::agent::progress::AgentProgress;

type Failure = tinytools::status::ClassifiedFailure;

/// Wire spelling of a unit-variant enum (`ClassifiedFailure` serializes its
/// class and category as the bare variant name).
fn variant_name<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|json| json.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn failure_summary(failure: &Failure) -> ToolFailureSummary {
    ToolFailureSummary {
        class: variant_name(&failure.class),
        category: variant_name(&failure.category),
        cause_plain: failure.cause_plain.clone(),
        next_action: failure.next_action.clone(),
        recoverable: failure.recoverable,
    }
}

/// The skeleton of a sub-agent output in `state`; callers fill what the state
/// carries.
fn subagent(agent_id: &str, task_id: &str, state: &str) -> SubagentOutput {
    SubagentOutput {
        agent_id: agent_id.to_owned(),
        task_id: task_id.to_owned(),
        state: state.to_owned(),
        mode: None,
        dedicated_thread: None,
        parent_call_id: None,
        call_id: None,
        iteration: None,
        max_iterations: None,
        prompt_chars: None,
        message: None,
        display_name: None,
        tool_name: None,
        arguments: None,
        success: None,
        elapsed_ms: None,
        output_chars: None,
        output: None,
        structured: None,
        usage: None,
        failure: None,
        display_label: None,
        display_detail: None,
        worker_thread_id: None,
        worktree_path: None,
        changed_files: Vec::new(),
        dirty_status: None,
        stop: None,
        extended_policy: None,
    }
}

fn lifecycle(
    stage: &str,
    message: Option<String>,
    iteration: Option<u32>,
    max: Option<u32>,
) -> ChannelOutputEvent {
    ChannelOutputEvent::Lifecycle {
        stage: stage.to_owned(),
        message,
        iteration,
        max_iterations: max,
        stop: None,
    }
}

pub(super) fn inference_start() -> ChannelOutputEvent {
    lifecycle("inference_start", None, None, None)
}

pub(super) fn inference_heartbeat() -> ChannelOutputEvent {
    lifecycle("inference_heartbeat", None, None, None)
}

pub(super) fn iteration_start(iteration: u32, max_iterations: u32) -> ChannelOutputEvent {
    lifecycle(
        "iteration_start",
        Some(format!("Iteration {iteration}/{max_iterations}")),
        Some(iteration),
        Some(max_iterations),
    )
}

/// A tool call of the parent turn, as it started.
pub(super) fn tool_started(
    call_id: String,
    tool_name: String,
    arguments: Value,
    iteration: u32,
    display_label: Option<String>,
    display_detail: Option<String>,
) -> ChannelOutputEvent {
    ChannelOutputEvent::ToolCallStarted {
        call_id,
        tool_name,
        arguments: cap_web_args(Some(arguments)).unwrap_or(Value::Null),
        iteration,
        display_label,
        display_detail,
    }
}

/// A finished tool call of the parent turn, with its size-capped result.
pub(super) fn tool_completed(event: AgentProgress) -> Option<ChannelOutputEvent> {
    let AgentProgress::ToolCallCompleted {
        call_id,
        tool_name,
        success,
        output_chars,
        output,
        arguments,
        elapsed_ms,
        iteration,
        failure,
        display_label,
        display_detail,
        structured,
    } = event
    else {
        return None;
    };
    Some(ChannelOutputEvent::ToolCallCompleted {
        call_id,
        tool_name,
        success,
        output_chars,
        output: cap_web_output(output),
        arguments: cap_web_args(arguments),
        elapsed_ms,
        iteration,
        failure: failure.as_ref().map(failure_summary),
        display_label,
        display_detail,
        structured,
    })
}

/// The parent's cumulative spend so far, as a live cost readout.
pub(super) fn cost_update(
    model: String,
    iteration: u32,
    input_tokens: u64,
    output_tokens: u64,
    cached_input_tokens: u64,
    total_usd: f64,
) -> ChannelOutputEvent {
    ChannelOutputEvent::Usage {
        model: Some(model),
        iteration: Some(iteration),
        input_tokens,
        output_tokens,
        cached_input_tokens,
        total_usd: Some(total_usd),
    }
}

pub(super) fn sub_spawned(
    agent_id: &str,
    task_id: &str,
    round: u32,
    mode: String,
    dedicated_thread: bool,
    prompt_chars: usize,
    worker_thread_id: Option<String>,
    display_name: Option<String>,
    parent_call_id: Option<String>,
) -> ChannelOutputEvent {
    let label = display_name.as_deref().unwrap_or(agent_id).to_owned();
    let mut out = subagent(agent_id, task_id, "spawned");
    out.message = Some(format!("Sub-agent '{label}' spawned"));
    out.tool_name = Some(agent_id.to_owned());
    out.iteration = Some(round);
    out.mode = Some(mode);
    out.dedicated_thread = Some(dedicated_thread);
    out.prompt_chars = Some(prompt_chars);
    out.worker_thread_id = worker_thread_id;
    out.display_name = display_name;
    out.parent_call_id = parent_call_id;
    ChannelOutputEvent::Subagent(Box::new(out))
}

/// A finished sub-agent. `iterations` is the child's own total, carried as the
/// output's iteration, which the projection reports as the detail's
/// `iterations`.
#[allow(clippy::too_many_arguments)]
pub(super) fn sub_completed(
    agent_id: &str,
    task_id: &str,
    elapsed_ms: u64,
    iterations: u32,
    output_chars: usize,
    output: String,
    usage: Option<&crate::agent::subagent_host::SubagentUsage>,
    worktree_path: Option<String>,
    changed_files: Vec<String>,
    dirty_status: Option<bool>,
    parent_call_id: Option<String>,
) -> ChannelOutputEvent {
    let mut out = subagent(agent_id, task_id, "completed");
    out.message = Some(format!(
        "Sub-agent '{agent_id}' completed in {elapsed_ms}ms"
    ));
    out.tool_name = Some(agent_id.to_owned());
    out.success = Some(true);
    out.iteration = Some(iterations);
    out.elapsed_ms = Some(elapsed_ms);
    out.output_chars = Some(output_chars);
    out.output = Some(cap_web_output(output));
    // Present only when this child's spend is not already inside the parent
    // turn's totals; the consumer adds it unconditionally, and reads an
    // unknown cost as zero.
    out.usage = usage.map(|usage| SubagentUsage {
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        cached_input_tokens: usage.cached_input_tokens,
        charged_amount_usd: usage.cost().usd().unwrap_or_default(),
        cost_source: Default::default(),
    });
    out.worktree_path = worktree_path;
    out.changed_files = changed_files;
    out.dirty_status = dirty_status;
    out.parent_call_id = parent_call_id;
    ChannelOutputEvent::Subagent(Box::new(out))
}

pub(super) fn sub_failed(
    agent_id: &str,
    task_id: &str,
    round: u32,
    error: String,
    parent_call_id: Option<String>,
) -> ChannelOutputEvent {
    let mut out = subagent(agent_id, task_id, "failed");
    out.message = Some(error);
    out.tool_name = Some(agent_id.to_owned());
    out.success = Some(false);
    out.iteration = Some(round);
    out.parent_call_id = parent_call_id;
    ChannelOutputEvent::Subagent(Box::new(out))
}

pub(super) fn sub_awaiting_user(
    agent_id: &str,
    task_id: &str,
    round: u32,
    question: String,
    worker_thread_id: Option<String>,
    parent_call_id: Option<String>,
) -> ChannelOutputEvent {
    let mut out = subagent(agent_id, task_id, "awaiting_user");
    out.message = Some(question);
    out.tool_name = Some(agent_id.to_owned());
    out.success = Some(true);
    out.iteration = Some(round);
    out.worker_thread_id = worker_thread_id;
    out.parent_call_id = parent_call_id;
    ChannelOutputEvent::Subagent(Box::new(out))
}

pub(super) fn sub_iteration(
    agent_id: &str,
    task_id: &str,
    iteration: u32,
    max_iterations: u32,
    extended_policy: bool,
) -> ChannelOutputEvent {
    let mut out = subagent(agent_id, task_id, "iteration_start");
    out.message = Some(if extended_policy {
        format!("Sub-agent '{agent_id}' step {iteration}")
    } else {
        format!("Sub-agent '{agent_id}' iteration {iteration}/{max_iterations}")
    });
    out.tool_name = Some(agent_id.to_owned());
    out.iteration = Some(iteration);
    out.max_iterations = (!extended_policy).then_some(max_iterations);
    ChannelOutputEvent::Subagent(Box::new(out))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn sub_tool_started(
    agent_id: &str,
    task_id: &str,
    call_id: String,
    tool_name: String,
    arguments: Value,
    iteration: u32,
    display_label: Option<String>,
    display_detail: Option<String>,
) -> ChannelOutputEvent {
    let mut out = subagent(agent_id, task_id, "tool_call");
    out.call_id = Some(call_id);
    out.tool_name = Some(tool_name);
    out.arguments = Some(arguments);
    out.iteration = Some(iteration);
    out.display_label = display_label;
    out.display_detail = display_detail;
    ChannelOutputEvent::Subagent(Box::new(out))
}

pub(super) fn sub_tool_completed(event: AgentProgress) -> Option<ChannelOutputEvent> {
    let AgentProgress::SubagentToolCallCompleted {
        agent_id,
        task_id,
        call_id,
        tool_name,
        success,
        output_chars,
        output,
        arguments,
        elapsed_ms,
        iteration,
        failure,
        display_label,
        display_detail,
        structured,
    } = event
    else {
        return None;
    };
    let mut out = subagent(&agent_id, &task_id, "tool_result");
    out.call_id = Some(call_id);
    out.tool_name = Some(tool_name);
    out.success = Some(success);
    out.iteration = Some(iteration);
    out.elapsed_ms = Some(elapsed_ms);
    out.output_chars = Some(output_chars);
    out.output = Some(cap_web_output(output));
    out.arguments = cap_web_args(arguments);
    out.structured = structured;
    out.failure = failure.as_ref().map(failure_summary);
    out.display_label = display_label;
    out.display_detail = display_detail;
    Some(ChannelOutputEvent::Subagent(Box::new(out)))
}

pub(super) fn sub_delta(
    agent_id: &str,
    task_id: &str,
    thinking: bool,
    delta: String,
    iteration: u32,
) -> ChannelOutputEvent {
    let mut out = subagent(
        agent_id,
        task_id,
        if thinking {
            "thinking_delta"
        } else {
            "text_delta"
        },
    );
    out.message = Some(delta);
    out.iteration = Some(iteration);
    ChannelOutputEvent::Subagent(Box::new(out))
}

#[cfg(test)]
#[path = "progress_map_tests.rs"]
mod tests;
