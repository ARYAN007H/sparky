//! Host adapters for canonical TinyChannels outbound messages.

use tinychannels_bus::{
    ChannelOutboundMessage, ChannelOutboundIntent, ChannelRef, ConversationRef, DeliveryDurability,
    OutboundPayload,
};

/// Build a text message addressed to one channel conversation.
pub(crate) fn text_message(
    channel: &str,
    conversation: &str,
    text: impl Into<String>,
    thread_id: Option<String>,
) -> ChannelOutboundMessage {
    let intent = ChannelOutboundIntent {
        idempotency_key: uuid::Uuid::new_v4().to_string(),
        channel_id: channel.to_owned(),
        conversation_id: conversation.to_owned(),
        channel_ref: Some(ChannelRef {
            id: channel.to_owned(),
            account_id: None,
        }),
        conversation_ref: Some(ConversationRef {
            id: conversation.to_owned(),
            thread_id: thread_id.clone(),
            ..Default::default()
        }),
        thread_id,
        durability: DeliveryDurability::BestEffort,
        payload: OutboundPayload::Text { text: text.into() },
        ..Default::default()
    };
    ChannelOutboundMessage::from_intent(&intent)
}
