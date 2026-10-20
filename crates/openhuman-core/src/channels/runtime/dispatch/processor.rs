//! Core message processing loop for the channel runtime.
//!
//! Contains:
//! * [`channel_has_approval_surface`] — gate controlling per-channel approval
//!   context scoping.
//! * [`try_route_approval_reply`] (in [`approval`]) — intercepts yes/no
//!   approval replies before dispatching a fresh agent turn.
//! * [`process_channel_message`] — full per-message pipeline: typing, ACK
//!   reaction, history, agent turn, draft updates, reply.
//! * [`bus_turn`] — the orchestrator path: an unbound channel's turn over
//!   the native bus. A channel bound to a host agent runs through
//!   [`super::host_agent`] instead.
//! * [`run_message_dispatch_loop`] — feeds messages into
//!   [`process_channel_runtime_message`] with a bounded set of host workers
//!   and joins them when the inbound queue closes.

mod approval;
mod bus_turn;
pub(crate) mod progressive;
mod reset;
mod turn;

use crate::channels::context::ChannelRuntimeContext;
use std::sync::Arc;
use tinychannels_bus::{ChannelInboundEnvelope, ChannelMessage};

/// Host-side inbound work item. Relay authentication remains out-of-band and
/// can only be set by the native callback after receiving the trusted module
/// callback; it is never reconstructed from envelope metadata.
#[derive(Clone)]
pub(crate) struct RuntimeChannelMessage {
    pub(crate) message: ChannelMessage,
    pub(crate) inbound_envelope: Option<ChannelInboundEnvelope>,
    pub(crate) authenticated_relay: bool,
    pub(crate) prepared_inbound: Option<tinychannels_bus::module_inbound::PreparedChannelInbound>,
    pub(crate) imported_history: Vec<tinyagents_session::transcript::TranscriptMessage>,
    /// Trusted per-turn callback target installed by a hosted relay. Native
    /// configured channels resolve their target from the prepared envelope.
    pub(crate) progressive_target: Option<Arc<progressive::NativeProgressiveTarget>>,
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
            imported_history: Vec::new(),
            progressive_target: None,
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
            imported_history: Vec::new(),
            progressive_target: None,
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
            imported_history: Vec::new(),
            progressive_target: None,
        }
    }
}

#[cfg(test)]
pub(crate) use approval::channel_has_approval_surface;
pub(crate) use reset::reset_durable_channel_session;
pub(crate) use turn::{process_channel_message, process_channel_runtime_message};

/// Run every inbound message through the agent pipeline, at most
/// `max_in_flight_messages` at a time, until `rx` closes.
pub(crate) async fn run_message_dispatch_loop(
    rx: tokio::sync::mpsc::Receiver<RuntimeChannelMessage>,
    ctx: Arc<ChannelRuntimeContext>,
    max_in_flight_messages: usize,
) {
    let mut rx = rx;
    let mut workers = tokio::task::JoinSet::new();
    while let Some(message) = rx.recv().await {
        while workers.len() >= max_in_flight_messages.max(1) {
            if let Some(Err(error)) = workers.join_next().await {
                tracing::warn!("channel dispatch worker failed: {error}");
            }
        }
        workers.spawn(process_channel_runtime_message(Arc::clone(&ctx), message));
        while let Some(result) = workers.try_join_next() {
            if let Err(error) = result {
                tracing::warn!("channel dispatch worker failed: {error}");
            }
        }
    }
    while let Some(result) = workers.join_next().await {
        if let Err(error) = result {
            tracing::warn!("channel dispatch worker failed: {error}");
        }
    }
}
