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
fn tiny_artifacts_blob_namespace_composes_with_host_artifact_collections() {
    let registry = registry().expect("the owners compose");
    assert_eq!(registry.owners().filter(|owner| *owner == "artifacts").count(), 1);

    let artifacts = owner_schemas()
        .into_iter()
        .find(|schema| schema.owner.as_str() == "artifacts")
        .expect("the artifact owners are composed");
    assert!(artifacts.items.iter().any(|item| matches!(
        item,
        Item::Collection { name, .. } if name == "artifacts.records"
    )));
    assert!(artifacts.items.iter().any(|item| matches!(
        item,
        Item::Blobs {
            prefix,
            scope: ScopeClass::Tenant,
        } if prefix == "artifacts/files/"
    )));
    assert!(!module_items("artifacts")
        .blobs
        .iter()
        .any(|prefix| prefix == "artifacts/files/"));
}
