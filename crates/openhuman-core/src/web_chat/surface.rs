//! The host's side of the `tinychannels` web adapter: where a turn's typed
//! events go.
//!
//! The adapter projects canonical output events onto the deployed web wire and
//! hands each result to a [`WebEventSink`]. This host's sink is the
//! process-wide `WebChannelEvent` broadcast (`event_bus_channel`), which
//! Socket.IO, the JSON-RPC `/events` stream, the relay and the TUI already
//! consume, stamped with the tenant of the context the turn runs under.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use tinychannels_bus::module_progressive::ChannelOutputEvent;
use tinychannels_bus::web as bus_web;
use tinychannels_runtime::adapters::web::{WebChannelAdapter, WebEventSink, WebScope};
use tinychannels_runtime::channel::ChannelSendError;

use super::event_bus_channel::publish_web_channel_event_in;
use super::WebChannelEvent;
use crate::core::runtime::DispatchCtx;

/// Addressing of one turn on the web wire.
#[derive(Clone, Debug)]
pub(crate) struct TurnIds {
    pub(crate) client_id: String,
    pub(crate) thread_id: String,
    pub(crate) request_id: String,
}

/// [`WebEventSink`] over the in-process broadcast.
struct SurfaceSink {
    ctx: DispatchCtx,
    /// The round of the narration the next `chat_interim` carries. The
    /// canonical `Clarification` output has no round, and the frontend keys one
    /// interim bubble per round, so the host supplies it here.
    interim_round: AtomicU32,
}

#[async_trait]
impl WebEventSink for SurfaceSink {
    async fn publish(
        &self,
        _scope: &WebScope,
        event: bus_web::WebChannelEvent,
    ) -> Result<(), ChannelSendError> {
        let mut flat = WebChannelEvent::from(event);
        if flat.event == "chat_interim" && flat.round.is_none() {
            flat.round = Some(self.interim_round.load(Ordering::Relaxed)).filter(|r| *r > 0);
        }
        publish_web_channel_event_in(&self.ctx, flat);
        Ok(())
    }
}

/// One turn's web adapter plus the sink it publishes through.
#[derive(Clone)]
pub(crate) struct Surface {
    adapter: Arc<WebChannelAdapter>,
    sink: Arc<SurfaceSink>,
    ids: TurnIds,
}

impl Surface {
    /// A surface for the turn `ids`, publishing under `ctx`. One adapter per
    /// turn: it carries that turn's `seq` counter, and dropping it with the
    /// turn is its cleanup.
    pub(crate) fn new(ctx: &DispatchCtx, ids: TurnIds) -> Self {
        let sink = Arc::new(SurfaceSink {
            ctx: ctx.clone(),
            interim_round: AtomicU32::new(0),
        });
        let adapter = Arc::new(WebChannelAdapter::new(
            None,
            WebScope::local(),
            sink.clone() as Arc<dyn WebEventSink>,
        ));
        Self { adapter, sink, ids }
    }

    /// Project one canonical output event onto the web wire, in `seq` order.
    pub(crate) async fn output(&self, event: ChannelOutputEvent) {
        let TurnIds {
            client_id,
            thread_id,
            request_id,
        } = &self.ids;
        if let Err(error) = self
            .adapter
            .publish_output(client_id, thread_id, request_id, event)
            .await
        {
            log::warn!(
                "[web-channel] output event not published request_id={request_id}: {}",
                error.message
            );
        }
    }

    /// The leading narration of `round`, flushed as an interim bubble.
    pub(crate) async fn interim(&self, round: u32, text: String) {
        self.sink.interim_round.store(round, Ordering::Relaxed);
        self.output(ChannelOutputEvent::Clarification { prompt: text })
            .await;
    }

    /// Publish a host-authored event unchanged: a terminal event, or one with
    /// no canonical output form.
    pub(crate) async fn event(&self, event: bus_web::WebChannelEvent) {
        if let Err(error) = self.adapter.publish_event(event).await {
            log::warn!(
                "[web-channel] event not published request_id={}: {}",
                self.ids.request_id,
                error.message
            );
        }
    }

    /// The addressing context for an event of this turn.
    pub(crate) fn context(&self) -> bus_web::WebEventContext {
        bus_web::WebEventContext {
            client_id: self.ids.client_id.clone(),
            thread_id: self.ids.thread_id.clone(),
            request_id: self.ids.request_id.clone(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
#[path = "surface_tests.rs"]
mod tests;
