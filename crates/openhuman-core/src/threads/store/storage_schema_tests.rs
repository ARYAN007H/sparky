use super::*;
use serde_json::{Value, json};
use tinystoragedrivers::schema::ScopeClass;
use tinystoragedrivers::{Meta, Precondition, StorageBackend};

#[test]
fn a_recorded_key_moves_under_the_threads_owner_keeping_its_body() {
    let out = transcript_keys_v2(vec![SourceRecord {
        id: "thread/1".into(),
        body: json!({ "key": "profile~agent" }),
        meta: Meta::unknown(),
    }])
    .unwrap();
    assert_eq!(out[0].collection, TRANSCRIPT_KEYS);
    assert_eq!(out[0].id, "thread%2F1");
    assert_eq!(out[0].body, json!({ "key": "profile~agent" }) as Value);
}

#[test]
fn channel_attachment_and_viseme_records_are_declared_in_the_threads_owner() {
    let schema = storage_schema();
    for (name, scope) in [
        (CHANNEL_MESSAGES, ScopeClass::Tenant),
        (STAGED_ATTACHMENTS, ScopeClass::Agent),
        (REPLY_VISEME_TIMELINES, ScopeClass::Agent),
    ] {
        assert!(
            schema
                .items
                .iter()
                .any(|item| item.name() == name && item.scope() == scope),
            "missing {name} collection with {scope:?} scope"
        );
    }
}

#[tokio::test]
async fn schema_prepares_the_scoped_projection_and_media_collections() {
    use crate::storage::{MemoryStorage, scope_for_profile};

    let registry = tinystoragedrivers::schema::Registry::compose([storage_schema()]).unwrap();
    let backend = MemoryStorage::new();
    registry.prepare(&backend).await.unwrap();

    let tenant_scope = scope_for_profile("schema-test");
    let tenant = backend.for_scope(&tenant_scope).unwrap();
    tenant
        .documents()
        .put(
            CHANNEL_MESSAGES,
            "provider-message",
            json!({ "threadId": "thread-1" }),
            Precondition::Absent,
        )
        .await
        .unwrap();

    let agent_scope = tenant_scope.agent("assistant").unwrap();
    let agent = backend.for_scope(&agent_scope).unwrap();
    for collection in [STAGED_ATTACHMENTS, REPLY_VISEME_TIMELINES] {
        agent
            .documents()
            .put(collection, "record-1", json!({}), Precondition::Absent)
            .await
            .unwrap();
    }
}

#[test]
fn the_schema_composes_under_its_owner() {
    tinystoragedrivers::schema::Registry::compose([storage_schema()]).unwrap();
}
