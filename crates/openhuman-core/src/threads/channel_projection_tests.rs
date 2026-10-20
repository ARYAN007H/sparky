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
fn channel_activity_emits_atomic_keyed_merge_patches() {
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
    let patch = channel_activity_merge_patch(&added).unwrap();
    let reaction_entries = patch["channelActivity"]["reactions"].as_object().unwrap();
    assert_eq!(reaction_entries.len(), 1);
    let (reaction_key, reaction_value) = reaction_entries.iter().next().unwrap();
    assert_eq!(reaction_value["actorId"], "alice");
    assert_eq!(reaction_value["value"], "🔥");

    // The key encodes tuple boundaries, so provider values containing a
    // separator cannot alias another actor/reaction tuple.
    let separator_collision = reaction_digest(
        "a|b",
        &ChannelReaction {
            value: "c".into(),
            custom_emoji_id: None,
        },
    )
    .unwrap();
    let distinct_tuple = reaction_digest(
        "a",
        &ChannelReaction {
            value: "b|c".into(),
            custom_emoji_id: None,
        },
    )
    .unwrap();
    assert_ne!(separator_collision, distinct_tuple);
    let custom = reaction_digest(
        "alice",
        &ChannelReaction {
            value: "🔥".into(),
            custom_emoji_id: Some("provider-custom-1".into()),
        },
    )
    .unwrap();
    assert_ne!(*reaction_key, custom);

    let read = event(ChannelEventKind::ReadReceipt {
        message_id: "provider-target".into(),
        actor: Some(tinychannels_bus::SenderRef {
            id: "bob".into(),
            ..Default::default()
        }),
        timestamp: Some(42),
    });
    let read_patch = channel_activity_merge_patch(&read).unwrap();
    let read_entries = read_patch["channelActivity"]["readReceipts"]
        .as_object()
        .unwrap();
    assert_eq!(read_entries.len(), 1);
    assert_eq!(read_entries.values().next().unwrap()["actorId"], "bob");
    assert_eq!(read_entries.values().next().unwrap()["timestamp"], 42);

    let removed = event(ChannelEventKind::ReactionRemoved {
        message_id: "provider-target".into(),
        provider_message_id: None,
        actor: tinychannels_bus::SenderRef {
            id: "alice".into(),
            ..Default::default()
        },
        reaction,
    });
    let remove_patch = channel_activity_merge_patch(&removed).unwrap();
    assert_eq!(
        remove_patch["channelActivity"]["reactions"][reaction_key],
        Value::Null
    );
}
