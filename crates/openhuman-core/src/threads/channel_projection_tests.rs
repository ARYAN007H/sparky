use super::*;
use crate::storage::{Scope, StorageBackend};
use std::sync::Arc;
use tinychannels_bus::ConversationKind;

fn repo(backend: &Arc<dyn StorageBackend>) -> Repo {
    Repo::on(backend, &Scope::local(), DOMAIN, collections).unwrap()
}

fn key(
    tenant_key: &str,
    account_id: Option<&str>,
    conversation_id: &str,
    topic_id: Option<&str>,
    provider_message_id: &str,
) -> MessageBindingKey {
    MessageBindingKey {
        tenant_key: tenant_key.to_owned(),
        channel: ChannelRef {
            id: "telegram".into(),
            account_id: account_id.map(str::to_owned),
        },
        conversation: ConversationRef {
            kind: ConversationKind::Thread,
            id: conversation_id.into(),
            scope_id: Some("workspace-1".into()),
            parent_id: Some("group-1".into()),
            thread_id: Some("thread-address-1".into()),
            topic_id: topic_id.map(str::to_owned),
        },
        provider_message_id: provider_message_id.into(),
    }
}

#[test]
fn provider_receipt_maps_to_the_exact_thread_and_row_after_restart() {
    let backend: Arc<dyn StorageBackend> = Arc::new(crate::storage::MemoryStorage::new());
    let first = repo(&backend);
    let binding = key(
        "tenant-agent-a",
        Some("account-1"),
        "conversation-1",
        Some("topic-1"),
        "provider-17",
    );

    bind_in(&first, binding.clone(), "channel:thread-a").unwrap();
    assert_eq!(
        resolve_in(&first, &binding).unwrap(),
        None,
        "a provider receipt is not a persisted UI row"
    );
    bind_row_in(&first, binding.clone(), "channel:thread-a", "user:row-9").unwrap();

    // A fresh Repository simulates process restart against the same backend.
    let restarted = repo(&backend);
    assert_eq!(
        resolve_in(&restarted, &binding).unwrap(),
        Some(ProjectedMessageTarget {
            thread_id: "channel:thread-a".into(),
            row_id: "user:row-9".into(),
        })
    );
}

#[test]
fn provider_receipts_are_scoped_by_tenant_account_and_full_conversation() {
    let backend: Arc<dyn StorageBackend> = Arc::new(crate::storage::MemoryStorage::new());
    let repo = repo(&backend);
    let original = key(
        "tenant-agent-a",
        Some("account-1"),
        "conversation-1",
        Some("topic-1"),
        "provider-17",
    );
    bind_in(&repo, original.clone(), "channel:thread-a").unwrap();
    bind_row_in(&repo, original.clone(), "channel:thread-a", "row-a").unwrap();

    for other in [
        key(
            "tenant-agent-b",
            Some("account-1"),
            "conversation-1",
            Some("topic-1"),
            "provider-17",
        ),
        key(
            "tenant-agent-a",
            Some("account-2"),
            "conversation-1",
            Some("topic-1"),
            "provider-17",
        ),
        key(
            "tenant-agent-a",
            Some("account-1"),
            "conversation-1",
            Some("topic-2"),
            "provider-17",
        ),
        key(
            "tenant-agent-a",
            Some("account-1"),
            "conversation-2",
            Some("topic-1"),
            "provider-17",
        ),
        key(
            "tenant-agent-a",
            Some("account-1"),
            "conversation-1",
            Some("topic-1"),
            "provider-18",
        ),
    ] {
        assert_eq!(resolve_in(&repo, &other).unwrap(), None);
    }
}

#[test]
fn sender_scoped_threads_can_share_a_conversation_without_overwriting() {
    let backend: Arc<dyn StorageBackend> = Arc::new(crate::storage::MemoryStorage::new());
    let repo = repo(&backend);
    let alice = key(
        "tenant",
        Some("account"),
        "group",
        Some("topic"),
        "alice-message",
    );
    let bob = key(
        "tenant",
        Some("account"),
        "group",
        Some("topic"),
        "bob-message",
    );

    bind_in(&repo, alice.clone(), "channel:alice").unwrap();
    bind_in(&repo, bob.clone(), "channel:bob").unwrap();
    bind_row_in(&repo, alice.clone(), "channel:alice", "row-alice").unwrap();
    bind_row_in(&repo, bob.clone(), "channel:bob", "row-bob").unwrap();
    assert_eq!(
        resolve_in(&repo, &alice).unwrap().unwrap().thread_id,
        "channel:alice"
    );
    assert_eq!(
        resolve_in(&repo, &bob).unwrap().unwrap().thread_id,
        "channel:bob"
    );
    assert!(bind_in(&repo, alice, "channel:other").is_err());
}

#[test]
fn channel_activity_metadata_applies_reaction_changes_and_read_receipts() {
    use tinychannels_bus::{ChannelEvent, ChannelEventKind, ChannelReaction, ChannelRef};

    let event = |kind| ChannelEvent {
        version: 2,
        event_id: "outer-event-id".into(),
        source_event_id: None,
        source_timestamp: None,
        channel: ChannelRef {
            id: "telegram".into(),
            account_id: Some("account".into()),
        },
        conversation: None,
        event: kind,
    };
    let reaction = ChannelReaction {
        value: "🔥".into(),
        custom_emoji_id: None,
    };
    let added = event(ChannelEventKind::ReactionAdded {
        message_id: "provider-target".into(),
        provider_message_id: None,
        actor: tinychannels_bus::SenderRef {
            id: "alice".into(),
            ..Default::default()
        },
        reaction: reaction.clone(),
    });
    let metadata = channel_activity_metadata(&json!({"existing": true}), &added).unwrap();
    let metadata = channel_activity_metadata(&metadata, &added).unwrap();
    assert_eq!(metadata["existing"], true);
    assert_eq!(metadata["channelActivity"]["reactions"].as_array().unwrap().len(), 1);

    let read = event(ChannelEventKind::ReadReceipt {
        message_id: "provider-target".into(),
        actor: Some(tinychannels_bus::SenderRef {
            id: "bob".into(),
            ..Default::default()
        }),
        timestamp: Some(42),
    });
    let metadata = channel_activity_metadata(&metadata, &read).unwrap();
    assert_eq!(metadata["channelActivity"]["readReceipts"][0]["actorId"], "bob");

    let removed = event(ChannelEventKind::ReactionRemoved {
        message_id: "provider-target".into(),
        provider_message_id: None,
        actor: tinychannels_bus::SenderRef {
            id: "alice".into(),
            ..Default::default()
        },
        reaction,
    });
    let metadata = channel_activity_metadata(&metadata, &removed).unwrap();
    assert!(metadata["channelActivity"]["reactions"].as_array().unwrap().is_empty());
}
