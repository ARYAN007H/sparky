use super::*;
#[cfg(feature = "legacy-import")]
use crate::storage::Precondition;
#[cfg(feature = "legacy-import")]
use serde_json::json;
use tinystoragedrivers_testkit::MemoryBackend;
use tinystoragedrivers_types::Meta;

fn known() -> Known {
    Known {
        agents: vec!["has a space".into(), "asha".into()],
        profiles: vec!["has a space".into()],
        modules: vec!["tinymcp".into()],
    }
}

fn mapped(src: &str) -> String {
    match map_scope(src, &known()) {
        Mapping::Move(scope) => scope.as_str().to_owned(),
        other => format!("{other:?}"),
    }
}

#[test]
fn every_v1_shape_maps_to_a_v2_scope() {
    assert_eq!(mapped("asha"), "local/agent:asha");
    assert_eq!(mapped("default"), "local/agent:default");
    assert_eq!(mapped("profile:alice"), "u:alice");
    assert_eq!(mapped("alice~asha"), "u:alice/agent:asha");
    assert_eq!(mapped("alice~default"), "u:alice/agent:default");
    assert_eq!(
        mapped("a%7Eb~asha"),
        format!("{}/agent:asha", scope_for_profile("a~b"))
    );
    assert_eq!(mapped("alice~module:tinymcp"), "u:alice/module:tinymcp");
    assert_eq!(mapped("module:tinymcp@local"), "local/module:tinymcp");
    assert_eq!(mapped("module:tinymcp@asha"), "local/module:tinymcp");
}

#[test]
fn hashed_v1_scopes_map_through_known_ids_only() {
    let agent = format!("sha256:{}", sha_hex(b"has a space"));
    assert_eq!(mapped(&agent), scope_for_agent("has a space").as_str());
    let profile = format!("profile-sha256:{}", sha_hex(b"has a space"));
    assert_eq!(mapped(&profile), scope_for_profile("has a space").as_str());
    assert_eq!(mapped(&format!("sha256:{}", sha_hex(b"nobody"))), "Unknown");
}

#[test]
fn v2_scopes_are_left_alone() {
    for scope in [
        "local",
        "u:alice",
        "u:alice/agent:x",
        "local/module:m",
        "cluster",
    ] {
        assert_eq!(map_scope(scope, &known()), Mapping::V2, "{scope}");
    }
}

#[tokio::test]
#[cfg(feature = "legacy-import")]
async fn a_v1_scope_moves_its_documents_streams_and_blobs() {
    let backend: Arc<dyn StorageBackend> = Arc::new(MemoryBackend::new());
    let source = "alice~asha";
    let inventory = crate::storage::import::scope_move::LegacyScopeInventory {
        scopes: vec![source.into()],
        documents: vec![crate::storage::import::scope_move::LegacyDocument {
            scope: source.into(),
            collection: "notes".into(),
            id: "n1".into(),
            body: json!({"a": 1}),
            meta: Meta {
                schema: 4,
                created_ms: 123,
                updated_ms: 456,
                expires_ms: None,
            },
        }],
        streams: vec![crate::storage::import::scope_move::LegacyStream {
            scope: source.into(),
            name: "journal".into(),
            base: 0,
            sealed: false,
            sealed_ms: None,
            entries: vec![(0, json!({"k":1})), (1, json!({"k":2}))],
        }],
        blobs: vec![crate::storage::import::scope_move::LegacyBlob {
            scope: source.into(),
            key: "b/1".into(),
            content_type: Some("text/plain".into()),
            expires_ms: None,
            bytes: b"hi".to_vec(),
        }],
    };
    let report = copy_inventory(&backend, &known(), &inventory)
        .await
        .unwrap();
    assert_eq!(report.moved, 1);
    assert!(report.documents >= 1);
    assert_eq!(report.entries, 2);
    assert_eq!(report.blobs, 1);
    assert_eq!(report.updated_times_not_restored, 1);

    let new = backend
        .for_scope(&scope_for_profile("alice").agent("asha").unwrap())
        .unwrap();
    let copied = new.documents().get("notes", "n1").await.unwrap().unwrap();
    assert_eq!(copied.doc, json!({"a": 1}));
    assert_eq!(copied.meta.schema, 4);
    assert_eq!(copied.meta.created_ms, 123);
    assert_eq!(new.streams().len("journal").await.unwrap(), 2);
    assert_eq!(new.blobs().get("b/1").await.unwrap().unwrap().bytes, b"hi");

    // The explicit source inventory remains available for operator review.
    assert_eq!(inventory.documents.len(), 1);
    assert_eq!(inventory.streams[0].entries.len(), 2);
    // Repeating the typed copy preserves rows without appending stream entries.
    let again = copy_inventory(&backend, &known(), &inventory)
        .await
        .unwrap();
    assert_eq!(again.entries, 0);
    assert_eq!(new.streams().len("journal").await.unwrap(), 2);
}

#[tokio::test]
#[cfg(feature = "legacy-import")]
async fn canonical_cluster_scope_import_converts_profiles_and_leases_in_place() {
    let backend: Arc<dyn StorageBackend> = Arc::new(MemoryBackend::new());
    let inventory = crate::storage::import::scope_move::LegacyScopeInventory {
        scopes: vec!["cluster".into()],
        documents: vec![
            crate::storage::import::scope_move::LegacyDocument {
                scope: "cluster".into(),
                collection: "profiles".into(),
                id: "alice".into(),
                body: json!({
                    "profile_id": "alice",
                    "created_at": 1_700_000_000_i64,
                    "layout_version": 3
                }),
                meta: Meta {
                    created_ms: 10,
                    updated_ms: 11,
                    ..Meta::default()
                },
            },
            crate::storage::import::scope_move::LegacyDocument {
                scope: "cluster".into(),
                collection: "leases".into(),
                id: "alice".into(),
                body: json!({
                    "owner": "worker-a",
                    "epoch": 8,
                    "expires_at_ms": 9_000,
                    "endpoint": "https://worker.example",
                    "released": false
                }),
                meta: Meta {
                    schema: 2,
                    created_ms: 20,
                    updated_ms: 21,
                    expires_ms: Some(10_000),
                },
            },
        ],
        ..Default::default()
    };

    let report = copy_inventory(&backend, &known(), &inventory)
        .await
        .unwrap();
    assert_eq!(report.moved, 1);
    assert_eq!(report.documents, 2);
    assert_eq!(report.updated_times_not_restored, 2);

    let cluster = backend
        .for_scope(&crate::storage::Scope::new("cluster").unwrap())
        .unwrap();
    let profile = cluster
        .documents()
        .get("profiles.registry", "alice")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        profile.doc,
        json!({"profile_id":"alice", "layout_version":3})
    );
    assert_eq!(profile.meta.created_ms, 1_700_000_000_000);

    let lease_id = crate::storage::schema_support::key_of_parts(["profiles", "alice"]).unwrap();
    let lease = cluster
        .documents()
        .get(crate::storage::lease::LEASE_COLLECTION, &lease_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        lease.doc,
        json!({"holder":"worker-a", "epoch":8, "expires_ms":9000, "endpoint":"https://worker.example"})
    );
    assert_eq!(lease.meta.schema, 2);
    assert_eq!(lease.meta.created_ms, 20);
    assert_eq!(lease.meta.expires_ms, Some(10_000));

    let operator = backend.for_scope(&crate::storage::Scope::local()).unwrap();
    assert!(
        operator
            .documents()
            .get("profiles.registry", "alice")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
#[cfg(feature = "legacy-import")]
async fn a_record_already_in_the_target_wins_a_merge() {
    let backend: Arc<dyn StorageBackend> = Arc::new(MemoryBackend::new());
    let target = backend.for_scope(&scope_for_agent("asha")).unwrap();
    target
        .documents()
        .put("notes", "n", json!({"from": "v2"}), Precondition::None)
        .await
        .unwrap();
    let inventory = crate::storage::import::scope_move::LegacyScopeInventory {
        scopes: vec!["asha".into()],
        documents: vec![crate::storage::import::scope_move::LegacyDocument {
            scope: "asha".into(),
            collection: "notes".into(),
            id: "n".into(),
            body: json!({"from": "v1"}),
            meta: Meta::default(),
        }],
        ..Default::default()
    };
    copy_inventory(&backend, &known(), &inventory)
        .await
        .unwrap();
    assert_eq!(
        target
            .documents()
            .get("notes", "n")
            .await
            .unwrap()
            .unwrap()
            .doc,
        json!({"from": "v2"})
    );
}

#[tokio::test]
#[cfg(feature = "legacy-import")]
async fn a_timestamped_stream_refuses_a_lossy_move_before_copying_entries() {
    let backend: Arc<dyn StorageBackend> = Arc::new(MemoryBackend::new());
    let inventory = crate::storage::import::scope_move::LegacyScopeInventory {
        scopes: vec!["asha".into()],
        streams: vec![crate::storage::import::scope_move::LegacyStream {
            scope: "asha".into(),
            name: "journal".into(),
            base: 0,
            sealed: true,
            sealed_ms: Some(123),
            entries: vec![(0, json!({"entry": 1}))],
        }],
        ..Default::default()
    };

    let error = copy_inventory(&backend, &known(), &inventory)
        .await
        .unwrap_err();

    assert!(error.to_string().contains("seal timestamp"));
    let target = backend.for_scope(&scope_for_agent("asha")).unwrap();
    assert_eq!(target.streams().len("journal").await.unwrap(), 0);
}
