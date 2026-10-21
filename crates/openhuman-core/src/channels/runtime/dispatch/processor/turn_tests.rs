use super::*;

#[tokio::test]
async fn native_inbound_preparation_preserves_image_envelope_for_typed_turns() {
    use tinychannels_bus::{
        channel::{
            ChannelRef, ContentPart, ConversationKind, ConversationRef, MediaKind, MediaReference,
            SenderRef,
        },
        module_inbound::{ChannelInboundSource, PreparedChannelInbound},
    };

    let envelope = tinychannels_bus::ChannelInboundEnvelope {
        channel: ChannelRef {
            id: "telegram".into(),
            account_id: Some("work".into()),
        },
        message_id: "provider-message-7".into(),
        conversation: ConversationRef {
            kind: ConversationKind::Dm,
            id: "chat-12".into(),
            thread_id: Some("thread-3".into()),
            topic_id: Some("topic-4".into()),
            ..Default::default()
        },
        sender: SenderRef {
            id: "sender-2".into(),
            ..Default::default()
        },
        text: String::new(),
        parts: vec![ContentPart::Image {
            media: MediaReference {
                opaque_id: Some("opaque-module-media".into()),
                content_type: Some("image/png".into()),
                size_bytes: Some(64),
                kind: MediaKind::Image,
                ..Default::default()
            },
            caption: None,
            alt_text: Some("diagram".into()),
        }],
        ..Default::default()
    };

    let request = channel_inbound_preparation_request(ChannelInboundSource::Envelope {
        envelope: envelope.clone(),
        timestamp: 1,
    });
    let prepared: PreparedChannelInbound = crate::modules::channel_controller::prepare_inbound(
        &crate::channels::module_fixture::client().await,
        request,
    )
    .await
    .expect("native channel preparation must retain typed image input");

    assert_eq!(prepared.envelope.parts, envelope.parts);
    assert_eq!(prepared.envelope.channel, envelope.channel);
    assert_eq!(prepared.envelope.conversation, envelope.conversation);
    assert_eq!(prepared.envelope.message_id, "provider-message-7");
    assert!(prepared.message.content.is_empty());
}
