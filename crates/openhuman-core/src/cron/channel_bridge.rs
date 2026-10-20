//! Cron's handle on the live channel send adapters. Conversation context and
//! delivered replies are stored in durable agent transcripts, not here.
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tinychannels_bus::SendMessage;
use tinychannels_runtime::Channel;

#[derive(Clone)]
struct Bridge {
    channels: Arc<HashMap<String, Arc<dyn Channel>>>,
}

fn slot() -> &'static RwLock<Option<Bridge>> {
    static SLOT: RwLock<Option<Bridge>> = RwLock::new(None);
    &SLOT
}

fn current() -> Option<Bridge> {
    slot().read().ok().and_then(|guard| guard.clone())
}

/// Register the running channel runtime's send adapters. The latest
/// registration wins (the runtime restarts on logout/login).
pub fn register_channel_bridge(channels: Arc<HashMap<String, Arc<dyn Channel>>>) {
    if let Ok(mut guard) = slot().write() {
        *guard = Some(Bridge { channels });
        tracing::debug!("[cron] channel delivery bridge registered");
    }
}

/// Send `text` to `reply_target` on the named channel through the registered
/// runtime. Errors when the runtime or channel is not available, or the send
/// fails.
pub async fn send_to_channel(
    channel: &str,
    reply_target: &str,
    thread_id: Option<&str>,
    text: &str,
    idempotency_key: &str,
) -> Result<(), String> {
    let Some(bridge) = current() else {
        return Err("channel runtime is not running".to_string());
    };
    let key = channel.trim().to_ascii_lowercase();
    let Some(ch) = bridge.channels.get(&key) else {
        return Err(format!("channel '{key}' is not available"));
    };
    // Run-specific key: a retry after an ambiguous send must not duplicate the
    // reminder, while identical reminders from different runs must not collide
    // (the content-derived default key would reuse one key for both).
    let mut message = SendMessage::new(text, reply_target).in_thread(thread_id.map(str::to_string));
    message.idempotency_key = Some(idempotency_key.to_string());
    ch.send(&message)
        .await
        .map_err(|e| format!("channel send failed: {e}"))
}

#[cfg(test)]
#[path = "channel_bridge_tests.rs"]
mod tests;
