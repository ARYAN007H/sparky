//! Progress projection onto the compiled TinyChannels progressive engine.

use crate::channels::bus::NativeProgressiveReply;
use tinyagents_harness::progress::AgentProgress;
use tinychannels_bus::module_progressive::ChannelOutputEvent;
use tokio::sync::mpsc;

pub(super) async fn open_native_reply(
    ctx: &crate::channels::context::ChannelRuntimeContext,
    envelope: &tinychannels_bus::channel::ChannelInboundEnvelope,
    turn_id: &str,
) -> anyhow::Result<Option<std::sync::Arc<NativeProgressiveReply>>> {
    let object = ctx
        .module_objects
        .get(&envelope.channel.id)
        .or_else(|| ctx.module_objects.values().next())
        .cloned();
    let Some(object) = object else {
        return Ok(None);
    };
    let reply = NativeProgressiveReply::open(
        object,
        envelope.channel.clone(),
        envelope.conversation.clone(),
        turn_id.to_owned(),
        std::time::Duration::from_secs(3_660),
    )
    .await?;
    Ok(Some(std::sync::Arc::new(reply)))
}

pub(super) async fn finish_native_reply(
    reply: &Option<std::sync::Arc<NativeProgressiveReply>>,
    text: &str,
) -> bool {
    let Some(reply) = reply else {
        return false;
    };
    let delivered = match reply.finish(text).await {
        Ok(result) if delivery_succeeded(&result) => true,
        Ok(result) => {
            tracing::error!(
                outcomes = delivery_outcome_count(&result),
                unsupported_parts = delivery_unsupported_count(&result),
                "[dispatch] native progressive final delivery was incomplete"
            );
            false
        }
        Err(error) => {
            tracing::error!("[dispatch] native progressive final delivery failed: {error}");
            if let Err(close_error) = reply.close().await {
                tracing::warn!("[dispatch] native progressive close failed: {close_error}");
            }
            false
        }
    };
    delivered
}

pub(super) fn spawn_native_progress(
    mut events: mpsc::Receiver<AgentProgress>,
    reply: std::sync::Arc<NativeProgressiveReply>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut failed = false;
        while let Some(progress) = events.recv().await {
            let Some(event) = output_event(progress) else {
                continue;
            };
            match reply.feed(vec![event]).await {
                Ok(result) if delivery_succeeded(&result) => {}
                Ok(result) => {
                    tracing::warn!(
                        outcomes = delivery_outcome_count(&result),
                        unsupported_parts = delivery_unsupported_count(&result),
                        "[dispatch] progressive channel update was incomplete"
                    );
                    failed = true;
                    break;
                }
                Err(error) => {
                    tracing::warn!("[dispatch] progressive channel update failed: {error}");
                    failed = true;
                    break;
                }
            }
        }
        if failed {
            if let Err(error) = reply.close().await {
                tracing::warn!("[dispatch] native progressive close failed: {error}");
            }
        }
    })
}

pub(crate) fn delivery_succeeded(
    result: &tinychannels_bus::module_progressive::ProgressiveResult,
) -> bool {
    use tinychannels_bus::module_progressive::ProgressiveResult;
    match result {
        ProgressiveResult::Updated => true,
        ProgressiveResult::DeliveryOutcome { outcome } => outcome.error.is_none(),
        ProgressiveResult::DeliveryOutcomes { outcomes } => {
            outcomes.iter().all(|outcome| outcome.error.is_none())
        }
        ProgressiveResult::DeliveryBatch {
            outcomes,
            unsupported,
            failure,
        } => {
            outcomes.iter().all(|outcome| outcome.error.is_none())
                && unsupported.is_empty()
                && failure.is_none()
        }
        ProgressiveResult::DeliveryUnsupported { parts } => parts.is_empty(),
        _ => false,
    }
}

fn delivery_outcome_count(
    result: &tinychannels_bus::module_progressive::ProgressiveResult,
) -> usize {
    use tinychannels_bus::module_progressive::ProgressiveResult;
    match result {
        ProgressiveResult::DeliveryOutcome { .. } => 1,
        ProgressiveResult::DeliveryOutcomes { outcomes }
        | ProgressiveResult::DeliveryBatch { outcomes, .. } => outcomes.len(),
        _ => 0,
    }
}

fn delivery_unsupported_count(
    result: &tinychannels_bus::module_progressive::ProgressiveResult,
) -> usize {
    use tinychannels_bus::module_progressive::ProgressiveResult;
    match result {
        ProgressiveResult::DeliveryUnsupported { parts }
        | ProgressiveResult::DeliveryBatch {
            unsupported: parts, ..
        } => parts.len(),
        _ => 0,
    }
}

pub(crate) fn output_event(progress: AgentProgress) -> Option<ChannelOutputEvent> {
    match progress {
        AgentProgress::TextDelta { delta, iteration } => Some(ChannelOutputEvent::TextDelta {
            text: delta,
            iteration: Some(iteration),
        }),
        AgentProgress::ThinkingDelta { delta, iteration } => {
            Some(ChannelOutputEvent::ThinkingDelta {
                text: delta,
                iteration: Some(iteration),
            })
        }
        AgentProgress::ToolCallStarted {
            call_id,
            tool_name,
            arguments,
            iteration,
            display_label,
            display_detail,
        } => Some(ChannelOutputEvent::ToolCallStarted {
            call_id,
            tool_name,
            arguments,
            iteration,
            display_label,
            display_detail,
        }),
        AgentProgress::ToolCallProgress { call_id, message } => {
            Some(ChannelOutputEvent::ToolCallProgress { call_id, message })
        }
        AgentProgress::ToolCallCompleted {
            call_id,
            tool_name,
            success,
            output_chars,
            output,
            arguments,
            elapsed_ms,
            iteration,
            display_label,
            display_detail,
            structured,
            ..
        } => Some(ChannelOutputEvent::ToolCallCompleted {
            call_id,
            tool_name,
            success,
            output_chars,
            output,
            arguments,
            elapsed_ms,
            iteration,
            failure: None,
            display_label,
            display_detail,
            structured,
        }),
        _ => None,
    }
}

#[cfg(test)]
#[path = "progressive_tests.rs"]
mod tests;
