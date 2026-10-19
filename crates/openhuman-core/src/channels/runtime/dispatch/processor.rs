//! Core message processing loop for the channel runtime.
//!
//! Contains:
//! * [`channel_has_approval_surface`] — gate controlling per-channel approval
//!   context scoping.
//! * [`admit`] — preparation, the received event, and the runtime-command and
//!   approval-reply intercepts that run before any debounce.
//! * [`try_route_approval_reply`] (in [`approval`]) — intercepts yes/no
//!   approval replies before dispatching a fresh agent turn.
//! * [`process_channel_message`] — full per-message pipeline: typing, ACK
//!   reaction, history, agent turn, draft updates, reply.
//! * [`bus_turn`] — the orchestrator path: an unbound channel's turn over
//!   the native bus. A channel bound to a host agent runs through
//!   [`super::host_agent`] instead.
//! * [`run_message_dispatch_loop`] — admits inbound messages, debounces them
//!   per conversation through [`crate::inbound_debounce`], runs one turn per
//!   batch with a bounded set of host workers, and drains everything when the
//!   inbound queue closes.

mod admit;
mod approval;
mod bus_turn;
mod turn;

use super::inbound_batch::{merge_channel_batch, QueuedInbound};
use crate::channels::context::ChannelRuntimeContext;
use crate::config::InboundDebounceConfig;
use crate::inbound_debounce::{KeyedDebouncer, Push};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tinychannels_bus::{ChannelInboundEnvelope, ChannelMessage};
use tokio::task::JoinSet;
use tokio::time::Instant;

/// Host-side inbound work item. Relay authentication remains out-of-band and
/// can only be set by the native callback after receiving the trusted module
/// callback; it is never reconstructed from envelope metadata.
#[derive(Clone)]
pub(crate) struct RuntimeChannelMessage {
    pub(crate) message: ChannelMessage,
    pub(crate) inbound_envelope: Option<ChannelInboundEnvelope>,
    pub(crate) authenticated_relay: bool,
    pub(crate) prepared_inbound: Option<tinychannels_bus::module_inbound::PreparedChannelInbound>,
}

impl RuntimeChannelMessage {
    pub(crate) fn with_inbound_envelope(
        message: ChannelMessage,
        inbound_envelope: ChannelInboundEnvelope,
    ) -> Self {
        Self {
            message,
            inbound_envelope: Some(inbound_envelope),
            authenticated_relay: false,
            prepared_inbound: None,
        }
    }

    pub(crate) fn from_authenticated_relay(
        delivery: tinychannels_bus::module_relay::RelayInboundDelivery,
    ) -> Self {
        Self {
            // The module's PrepareChannelInbound operation supplies the
            // normalized legacy projection before the processor consumes it.
            message: ChannelMessage::default(),
            inbound_envelope: Some(delivery.envelope),
            authenticated_relay: delivery.delivered_via_authenticated_relay,
            prepared_inbound: None,
        }
    }
}

impl From<ChannelMessage> for RuntimeChannelMessage {
    fn from(message: ChannelMessage) -> Self {
        Self {
            message,
            inbound_envelope: None,
            authenticated_relay: false,
            prepared_inbound: None,
        }
    }
}

use admit::admit_runtime_message;
pub(crate) use admit::AdmittedInbound;
#[cfg(test)]
pub(crate) use approval::channel_has_approval_surface;
use turn::run_admitted_turn;
pub(crate) use turn::{process_channel_message, process_channel_runtime_message};

/// Run every inbound message through the agent pipeline until `rx` closes.
///
/// Admitted messages are debounced per conversation (`history_key`): a burst
/// becomes one turn, and messages that arrive while a conversation's turn is
/// running are collected into one follow-up turn. At most
/// `max_in_flight_messages` turns run at once.
pub(crate) async fn run_message_dispatch_loop(
    rx: tokio::sync::mpsc::Receiver<RuntimeChannelMessage>,
    ctx: Arc<ChannelRuntimeContext>,
    max_in_flight_messages: usize,
    debounce: InboundDebounceConfig,
) {
    let max_in_flight = max_in_flight_messages.max(1);
    let mut rx = rx;
    let mut inbound_open = true;
    let mut next_seq: u64 = 0;
    let mut admitting = JoinSet::new();
    let mut turns = JoinSet::new();
    let mut turn_keys: HashMap<tokio::task::Id, String> = HashMap::new();
    let mut typing = JoinSet::new();
    let mut debouncer: KeyedDebouncer<String, QueuedInbound> = KeyedDebouncer::new();
    let mut ready: VecDeque<(String, Vec<QueuedInbound>)> = VecDeque::new();

    loop {
        while turns.len() < max_in_flight {
            let Some((key, batch)) = ready.pop_front() else {
                break;
            };
            let batch_len = batch.len();
            match merge_channel_batch(batch) {
                Some(admitted) => {
                    tracing::debug!(
                        channel = %admitted.msg.channel,
                        message_id = %admitted.msg.id,
                        batch_len,
                        in_flight = turns.len() + 1,
                        "[dispatch][debounce] starting turn"
                    );
                    let handle = turns.spawn(run_admitted_turn(Arc::clone(&ctx), admitted));
                    turn_keys.insert(handle.id(), key);
                }
                None => debouncer.complete(&key),
            }
        }
        while let Some(result) = typing.try_join_next() {
            log_join_error("typing", result.err());
        }
        if !inbound_open
            && admitting.is_empty()
            && turns.is_empty()
            && ready.is_empty()
            && debouncer.is_empty()
        {
            break;
        }

        let wakeup = debouncer.next_wakeup();
        tokio::select! {
            received = rx.recv(), if inbound_open && admitting.len() < max_in_flight => {
                match received {
                    Some(message) => {
                        let seq = next_seq;
                        next_seq += 1;
                        let ctx = Arc::clone(&ctx);
                        admitting.spawn(async move { (seq, admit_runtime_message(ctx, message).await) });
                    }
                    None => {
                        tracing::debug!(
                            buffered_keys = debouncer.tracked(),
                            "[dispatch][debounce] inbound queue closed; flushing buffered messages"
                        );
                        inbound_open = false;
                    }
                }
            }
            Some(joined) = admitting.join_next(), if !admitting.is_empty() => {
                match joined {
                    Ok((seq, Some(inbound))) => {
                        let settings = debounce.settings_for(&inbound.msg.channel);
                        let channel = inbound.msg.channel.clone();
                        let reply_target = inbound.msg.reply_target.clone();
                        let key = inbound.history_key.clone();
                        let push = debouncer.push(
                            key,
                            QueuedInbound { seq, inbound },
                            &settings,
                            Instant::now(),
                        );
                        tracing::debug!(
                            channel = %channel,
                            seq,
                            outcome = ?push,
                            tracked = debouncer.tracked(),
                            "[dispatch][debounce] buffered inbound message"
                        );
                        if push == Push::Opened {
                            if let Some(target) = ctx.channels_by_name.get(&channel).cloned() {
                                typing.spawn(async move {
                                    if let Err(error) = target.start_typing(&reply_target).await {
                                        tracing::debug!(
                                            "[dispatch][debounce] typing start failed on {}: {error}",
                                            target.name()
                                        );
                                    }
                                });
                            }
                        }
                    }
                    Ok((_, None)) => {}
                    Err(error) => log_join_error("admission", Some(error)),
                }
            }
            Some(joined) = turns.join_next_with_id(), if !turns.is_empty() => {
                let id = match joined {
                    Ok((id, ())) => id,
                    Err(error) => {
                        let id = error.id();
                        log_join_error("turn", Some(error));
                        id
                    }
                };
                if let Some(key) = turn_keys.remove(&id) {
                    debouncer.complete(&key);
                }
            }
            _ = tokio::time::sleep_until(wakeup.unwrap_or_else(Instant::now)), if wakeup.is_some() => {}
        }

        let now = Instant::now();
        if !inbound_open && admitting.is_empty() {
            debouncer.flush_all(now);
        }
        ready.extend(debouncer.poll_ready(now));
    }
}

fn log_join_error(stage: &str, error: Option<tokio::task::JoinError>) {
    if let Some(error) = error {
        tracing::warn!("channel dispatch {stage} worker failed: {error}");
    }
}
