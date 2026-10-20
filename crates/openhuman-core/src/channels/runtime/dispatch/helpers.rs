//! OpenHuman-specific dispatch prompt context plus TinyChannels helpers.

use crate::channels::context::CHANNEL_TYPING_REFRESH_INTERVAL_SECS;
use crate::channels::Channel;
use tinychannels_bus as traits;

/// Maximum characters shown in the debug reply println.
pub(super) const REPLY_LOG_TRUNCATE_CHARS: usize = 200;

#[cfg(test)]
pub(super) use tinychannels::runtime::select_acknowledgment_reaction;

pub(super) fn log_worker_join_result(result: Result<(), tokio::task::JoinError>) {
    if let Err(error) = result {
        tracing::warn!(
            "{}",
            tinychannels_runtime::context::worker_crash_message(&error)
        );
    }
}

/// Start the portable typing lifecycle with OpenHuman's configured interval.
pub(super) fn spawn_typing_task(
    dispatch: &crate::core::runtime::DispatchCtx,
    channel: std::sync::Arc<dyn crate::channels::Channel>,
    recipient: String,
    cancellation_token: tokio_util::sync::CancellationToken,
) -> tokio::task::JoinHandle<()> {
    let (finished_tx, finished_rx) = tokio::sync::oneshot::channel();
    // The worker retains ownership across cancellation of the caller while
    // StartTypingRefresh is in flight, then closes the returned module handle.
    dispatch.spawn(async move {
        match channel
            .begin_typing_refresh(&recipient, CHANNEL_TYPING_REFRESH_INTERVAL_SECS)
            .await
        {
            Ok(Some(handle)) => {
                cancellation_token.cancelled().await;
                if channel.end_typing_refresh(&handle).await.is_err() {
                    tracing::debug!(channel = channel.name(), "typing refresh cleanup failed");
                }
            }
            Ok(None) => cancellation_token.cancelled().await,
            Err(_) => tracing::debug!(channel = channel.name(), "typing refresh unavailable"),
        }
        if channel.stop_typing(&recipient).await.is_err() {
            tracing::debug!(channel = channel.name(), "typing stop failed");
        }
        let _ = finished_tx.send(());
    });
    dispatch.spawn(async move {
        let _ = finished_rx.await;
    })
}

/// Build the `[Channel context]` block for a channel turn.
pub(super) fn build_channel_context_block(msg: &traits::ChannelMessage) -> String {
    tokio_util::task::AbortOnDropHandle::new(tokio::spawn(async move {
        let mut accumulated = String::new();
        let mut last_thinking_update = None;
        const THINKING_UPDATE_INTERVAL_MS: u128 = 2000;

        while let Some(progress) = progress.recv().await {
            match progress {
                AgentProgress::TextDelta { delta, .. } => {
                    accumulated.push_str(&delta);
                    if let Err(error) = channel
                        .update_draft(&reply_target, &draft_id, &accumulated)
                        .await
                    {
                        tracing::debug!("Draft update failed: {error}");
                    }
                }
                AgentProgress::ThinkingDelta { .. } => {
                    if accumulated.is_empty() {
                        let now = std::time::Instant::now();
                        let should_update = match last_thinking_update {
                            None => true,
                            Some(last) => {
                                now.duration_since(last).as_millis() > THINKING_UPDATE_INTERVAL_MS
                            }
                        };

                        if should_update {
                            if let Err(error) = channel
                                .update_draft(&reply_target, &draft_id, "Thinking...")
                                .await
                            {
                                tracing::debug!("Thinking update failed: {error}");
                            }
                            last_thinking_update = Some(now);
                        }
                    }
                }
                AgentProgress::ToolCallStarted { tool_name, .. } if accumulated.is_empty() => {
                    let _ = channel
                        .update_draft(
                            &reply_target,
                            &draft_id,
                            &format!("Working ({tool_name})..."),
                        )
                        .await;
                }
                _ => {}
            }
        }
    }))
}

/// Build the `[Channel context]` block for a channel turn.
///
/// Reminders and scheduled tasks created from this chat are delivered back to
/// it automatically (the job records its origin), so the model is told only
/// that, not a target to copy.
pub(super) fn build_channel_context_block(msg: &traits::ChannelMessage) -> String {
    let channel = msg.channel.trim();
    if channel.is_empty()
        || channel.eq_ignore_ascii_case("web")
        || channel.eq_ignore_ascii_case("cli")
    {
        return String::new();
    }
    let reply_target = msg.reply_target.trim();
    if reply_target.is_empty() {
        return String::new();
    }
    format!(
        "[Channel context]\n\
         You are responding via the \"{channel}\" channel. Reminders and scheduled tasks you \
         create here are delivered back to this chat automatically; do not set a delivery \
         target.\n\n"
    )
}

#[cfg(test)]
#[path = "helpers_tests.rs"]
mod tests;
