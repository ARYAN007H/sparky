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
    if let Err(error) = reply.finish(text).await {
        tracing::error!("[dispatch] native progressive final delivery failed: {error}");
    }
    true
}

pub(super) fn spawn_native_progress(
    mut events: mpsc::Receiver<AgentProgress>,
    reply: std::sync::Arc<NativeProgressiveReply>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(progress) = events.recv().await {
            let Some(event) = output_event(progress) else {
                continue;
            };
            if let Err(error) = reply.feed(vec![event]).await {
                tracing::warn!("[dispatch] progressive channel update failed: {error}");
                break;
            }
        }
    })
}

fn output_event(progress: AgentProgress) -> Option<ChannelOutputEvent> {
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
