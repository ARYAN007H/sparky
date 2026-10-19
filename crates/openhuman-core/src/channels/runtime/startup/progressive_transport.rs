//! Progressive output transport through the configured TinyChannels module adapter.

use super::callback::ProgressiveChannelTransport;
use super::module_providers::ModuleChannel;
use async_trait::async_trait;
use tinychannels_bus::{ChannelInteraction, ChannelInteractionResult, SendMessage};

#[async_trait]
impl ProgressiveChannelTransport for ModuleChannel {
    fn configured_name(&self) -> &str {
        &self.name
    }

    async fn dispatch_progressive(
        &self,
        request: tinychannels_bus::module_progressive::ProgressiveTransportRequest,
    ) -> tinybus::Result<tinychannels_bus::module_progressive::ProgressiveTransportResult> {
        use tinychannels_bus::module_progressive::{
            ProgressiveTransportRequest as Request, ProgressiveTransportResult as Result,
        };

        match request {
            Request::SendTyped {
                channel,
                turn_id,
                channel_ref,
                conversation,
                sequence: _,
                segment_index: _,
                idempotency_key,
                parts,
                ..
            } => {
                let outbound = tinychannels_bus::channel::ChannelOutboundMessage {
                    channel_id: channel,
                    conversation_id: conversation
                        .as_ref()
                        .map(|reference| reference.id.clone())
                        .unwrap_or_default(),
                    channel_ref,
                    conversation_ref: conversation,
                    reply_to_id: None,
                    thread_id: None,
                    turn_id,
                    idempotency_key: idempotency_key.unwrap_or_default(),
                    parts,
                };
                let outcome = self
                    .send_typed_with_receipt(&outbound)
                    .await
                    .map_err(|_| tinybus::Error::failed("progressive channel send failed"))?;
                Ok(Result::Outcome(Box::new(outcome)))
            }
            Request::Send {
                channel,
                turn_id,
                channel_ref,
                conversation,
                idempotency_key,
                message,
                ..
            } => {
                let legacy: SendMessage = serde_json::from_value(message)
                    .map_err(|_| tinybus::Error::failed("invalid progressive send payload"))?;
                let mut outbound =
                    tinychannels_bus::channel::ChannelOutboundMessage::from_legacy_message(
                        channel_ref
                            .as_ref()
                            .map(|reference| reference.id.as_str())
                            .unwrap_or(&channel),
                        &legacy,
                    );
                outbound.channel_ref = channel_ref;
                outbound.conversation_ref = conversation.or(outbound.conversation_ref);
                outbound.turn_id = turn_id;
                if let Some(key) = idempotency_key {
                    outbound.idempotency_key = key;
                }
                let outcome = self
                    .send_typed_with_receipt(&outbound)
                    .await
                    .map_err(|_| tinybus::Error::failed("progressive channel send failed"))?;
                Ok(Result::Outcome(Box::new(outcome)))
            }
            Request::Edit {
                channel,
                turn_id,
                channel_ref,
                conversation,
                message_id: _,
                receipt,
                text,
            } => {
                let Some(receipt) = receipt else {
                    return Ok(Result::Unavailable);
                };
                let channel_id = channel_ref
                    .as_ref()
                    .map(|reference| reference.id.clone())
                    .unwrap_or(channel);
                let conversation_id = conversation
                    .as_ref()
                    .map(|reference| reference.id.clone())
                    .unwrap_or_default();
                let intent = tinychannels_bus::channel::ChannelOutboundIntent {
                    idempotency_key: format!(
                        "progressive-edit:{}:{}",
                        turn_id.as_deref().unwrap_or("unscoped"),
                        receipt
                            .primary_platform_message_id
                            .as_deref()
                            .unwrap_or("unknown")
                    ),
                    channel_id,
                    conversation_id,
                    channel_ref,
                    conversation_ref: conversation.clone(),
                    reply_to_id: receipt.reply_to_id.clone(),
                    thread_id: receipt.thread_id.clone(),
                    turn_id,
                    durability: tinychannels_bus::channel::DeliveryDurability::Required,
                    payload: tinychannels_bus::channel::OutboundPayload::Text { text },
                };
                match self
                    .object
                    .call_confidential::<ChannelInteractionResult>(
                        tinychannels_bus::methods::CHANNEL_INTERACTION,
                        tinychannels_bus::ChannelInteractionRequest {
                            channel: self.name.clone(),
                            interaction: ChannelInteraction::EditMessage { receipt, intent },
                        },
                    )
                    .await
                {
                    Ok(ChannelInteractionResult::Sent(outcome)) => {
                        match outcome.error.as_ref().map(|error| error.kind) {
                            Some(tinychannels_bus::channel::SendErrorKind::NotFound) => {
                                Ok(Result::MessageGone)
                            }
                            Some(tinychannels_bus::channel::SendErrorKind::Forbidden) => {
                                Ok(Result::EditUnsupported)
                            }
                            _ => Ok(Result::Outcome(outcome)),
                        }
                    }
                    Ok(_) => Err(tinybus::Error::failed(
                        "channel returned an invalid edit result",
                    )),
                    Err(error) => Err(error),
                }
            }
            Request::Delete { receipt, .. } => {
                let Some(receipt) = receipt else {
                    return Ok(Result::Unavailable);
                };
                match self
                    .object
                    .call_confidential::<ChannelInteractionResult>(
                        tinychannels_bus::methods::CHANNEL_INTERACTION,
                        tinychannels_bus::ChannelInteractionRequest {
                            channel: self.name.clone(),
                            interaction: ChannelInteraction::DeleteMessage { receipt },
                        },
                    )
                    .await
                {
                    Ok(ChannelInteractionResult::Completed) => Ok(Result::Acknowledged),
                    Ok(_) => Err(tinybus::Error::failed(
                        "channel returned an invalid delete result",
                    )),
                    Err(error) => Err(error),
                }
            }
            Request::Typing { conversation, .. } => {
                let recipient = conversation
                    .map(|reference| reference.id)
                    .unwrap_or_default();
                self.interact(ChannelInteraction::StartTyping { recipient })
                    .await
                    .map_err(|_| tinybus::Error::failed("progressive typing callback failed"))?;
                Ok(Result::Acknowledged)
            }
            Request::OutputEvent { .. } => Ok(Result::Unavailable),
        }
    }
}
