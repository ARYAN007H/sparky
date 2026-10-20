use super::*;
use tinyflows_bus::schedule::types::ChannelConversationIdentity;

#[test]
fn conversation_ref_preserves_full_provider_scope() {
    let identity = ChannelConversationIdentity {
        kind: "group".into(),
        id: "room-1".into(),
        scope_id: Some("account-scope".into()),
        parent_id: Some("parent-1".into()),
        thread_id: Some("thread-2".into()),
        topic_id: Some("topic-3".into()),
    };
    let conversation = conversation_ref(&identity).unwrap();
    assert_eq!(conversation.kind, ConversationKind::Group);
    assert_eq!(conversation.id, "room-1");
    assert_eq!(conversation.scope_id.as_deref(), Some("account-scope"));
    assert_eq!(conversation.parent_id.as_deref(), Some("parent-1"));
    assert_eq!(conversation.thread_id.as_deref(), Some("thread-2"));
    assert_eq!(conversation.topic_id.as_deref(), Some("topic-3"));
}

#[test]
fn conversation_ref_rejects_unknown_kind_claims() {
    let identity = ChannelConversationIdentity {
        kind: "invented".into(),
        id: "room-1".into(),
        scope_id: None,
        parent_id: None,
        thread_id: None,
        topic_id: None,
    };
    assert!(conversation_ref(&identity).is_err());
}
