//! Scoped callback used by the compiled TinyChannels progressive engine for a
//! hosted relay turn. The callback is created per admitted turn and captures
//! the trusted tenant/account/conversation address; request metadata cannot
//! choose a destination.

use std::sync::Arc;

use tinybus::Error;
use tinychannels_bus::{
    module_progressive::{ProgressiveTransportRequest, ProgressiveTransportResult},
    ChannelBackendRequest, ChannelBackendResult,
};

use super::channel::RelayChannel;

pub(crate) struct RelayProgressiveTransport {
    ctx: crate::core::runtime::DispatchCtx,
    channel: Arc<RelayChannel>,
    channel_ref: tinychannels_bus::ChannelRef,
    conversation: tinychannels_bus::ConversationRef,
    turn_id: String,
    projection: Arc<crate::web_chat::Surface>,
}

impl RelayProgressiveTransport {
    pub(crate) fn new(
        ctx: crate::core::runtime::DispatchCtx,
        channel: Arc<RelayChannel>,
        channel_ref: tinychannels_bus::ChannelRef,
        conversation: tinychannels_bus::ConversationRef,
        turn_id: String,
        projection: Arc<crate::web_chat::Surface>,
    ) -> Self {
        Self {
            ctx,
            channel,
            channel_ref,
            conversation,
            turn_id,
            projection,
        }
    }

    fn check_scope(
        &self,
        route: &str,
        turn_id: Option<&str>,
        channel_ref: Option<&tinychannels_bus::ChannelRef>,
        conversation: Option<&tinychannels_bus::ConversationRef>,
    ) -> tinybus::Result<()> {
        // The module keys a scoped output by the provider id; the conversation
        // is verified separately through the full scope below.
        if route != self.channel_ref.id
            || turn_id != Some(self.turn_id.as_str())
            || channel_ref != Some(&self.channel_ref)
            || conversation != Some(&self.conversation)
        {
            return Err(Error::failed("hosted relay output scope mismatch"));
        }
        Ok(())
    }

    async fn dispatch(
        &self,
        request: ProgressiveTransportRequest,
    ) -> tinybus::Result<ProgressiveTransportResult> {
        match request {
            ProgressiveTransportRequest::OutputEvent {
                channel,
                turn_id,
                channel_ref,
                conversation,
                event,
                ..
            } => {
                self.check_scope(
                    &channel,
                    turn_id.as_deref(),
                    channel_ref.as_ref(),
                    conversation.as_ref(),
                )?;
                // Project canonical events through the same compiled web
                // adapter used by desktop and TUI turns. Provider delivery is
                // still performed only by the typed terminal send below.
                self.projection.output(*event).await;
                Ok(ProgressiveTransportResult::Acknowledged)
            }
            ProgressiveTransportRequest::SendTyped {
                channel,
                turn_id,
                channel_ref,
                conversation,
                sequence,
                segment_index,
                idempotency_key,
                parts,
                text,
            } => {
                self.check_scope(
                    &channel,
                    turn_id.as_deref(),
                    channel_ref.as_ref(),
                    conversation.as_ref(),
                )?;
                let (Some(sequence), Some(segment_index), Some(idempotency_key)) =
                    (sequence, segment_index, idempotency_key)
                else {
                    return Err(Error::failed("hosted relay send has no typed identity"));
                };
                if idempotency_key.is_empty() || idempotency_key.len() > 128 {
                    return Err(Error::failed("hosted relay send identity is invalid"));
                }
                self.channel.publish_typed_reply(
                    &self.ctx,
                    &self.channel_ref,
                    &self.conversation,
                    &self.turn_id,
                    sequence,
                    segment_index,
                    &idempotency_key,
                    &parts,
                    &text,
                );
                Ok(ProgressiveTransportResult::Accepted)
            }
            _ => Err(Error::failed("hosted relay operation is unavailable")),
        }
    }
}

#[tinybus::interface(name = "ai.tinyhumans.tinychannels.ChannelsHost")]
impl RelayProgressiveTransport {
    async fn dispatch_controller_backend(
        &self,
        request: ChannelBackendRequest,
    ) -> tinybus::Result<ChannelBackendResult> {
        let ChannelBackendRequest::ProgressiveTransport(request) = request else {
            return Err(Error::failed(
                "hosted relay callback capability unavailable",
            ));
        };
        Ok(ChannelBackendResult::ProgressiveTransport(Box::new(
            self.dispatch(*request).await?,
        )))
    }
}
