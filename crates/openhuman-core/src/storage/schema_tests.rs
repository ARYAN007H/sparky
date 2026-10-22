use super::*;

#[test]
fn every_owner_composes_without_collision() {
    let registry = registry().expect("the owners compose");
    let owners: Vec<&str> = registry.owners().collect();
    for expected in [
        "harness",
        "session",
        "graph",
        "tasks",
        "approvals",
        "devices",
        "notifications",
        "task_sources",
        "artifacts",
        "cost",
        "config",
        "profiles",
        "threads",
        "subagents",
        "agents",
    ] {
        assert!(owners.contains(&expected), "{expected} missing: {owners:?}");
    }
}

#[test]
fn module_items_are_the_owners_module_scoped_declarations() {
    let mcp = module_items("mcp");
    assert!(mcp
        .collections
        .iter()
        .any(|spec| spec.name == "mcp.servers"));
    assert!(mcp.streams.iter().any(|name| name == "mcp.audit"));
    assert!(module_items("no-such-owner").collections.is_empty());
}

#[test]
fn tinyjuice_ccr_blobs_stay_inside_the_juice_owner_namespace() {
    assert_eq!(module_items("juice").blobs, ["juice/ccr/"]);
}

#[cfg(feature = "modules")]
#[tokio::test]
async fn tinyjuice_legacy_ccr_blobs_migrate_with_the_owner_contract() {
    use tinystoragedrivers::schema::{MigrateOptions, Registry};
    use tinystoragedrivers::{MemoryStorage, Scope, StorageBackend};

    let backend = MemoryStorage::new();
    let scoped = backend.for_scope(&Scope::local()).unwrap();
    let blobs = scoped.blobs();
    blobs
        .put(
            "tinyjuice_ccr/0123456789abcdef",
            b"original text".to_vec(),
            Some("text/plain; charset=utf-8"),
        )
        .await
        .unwrap();
    blobs
        .put("tinyjuice_ccr/fedcba9876543210", b"two".to_vec(), None)
        .await
        .unwrap();
    blobs
        .put("other/keep", b"keep".to_vec(), None)
        .await
        .unwrap();

    let registry = Registry::compose([tinyjuice_storage_schema()]).unwrap();
    let report = registry
        .migrate(&scoped, &MigrateOptions::new())
        .await
        .unwrap();
    assert_eq!(report.ran[0].written, 2);
    assert!(blobs
        .list(tinyjuice_bus::storage::LEGACY_CCR_PREFIX)
        .await
        .unwrap()
        .is_empty());
    let moved = blobs
        .get("juice/ccr/0123456789abcdef")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(moved.bytes, b"original text");
    assert_eq!(
        moved.meta.content_type.as_deref(),
        Some("text/plain; charset=utf-8")
    );
    let moved = blobs
        .get("juice/ccr/fedcba9876543210")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(moved.meta.content_type, None);
    assert!(blobs.get("other/keep").await.unwrap().is_some());

    let again = registry
        .migrate(&scoped, &MigrateOptions::new())
        .await
        .unwrap();
    assert!(again.ran.is_empty());
}
