use super::*;

#[test]
fn every_linked_crate_exposes_its_valid_collection_declarations() {
    let owners = collection_owners().expect("owner collection declarations are valid");
    let mut crates = std::collections::HashSet::new();
    let mut names = std::collections::HashSet::new();
    for owner in &owners {
        assert!(
            crates.insert(owner.crate_name),
            "duplicate crate {}",
            owner.crate_name
        );
        for spec in &owner.collections {
            spec.validate().expect("owner collection is valid");
            assert!(
                names.insert((owner.crate_name, spec.name.as_str())),
                "duplicate {} collection in {}",
                spec.name,
                owner.crate_name
            );
        }
    }
    for expected in [
        ("tinyagents-harness", "harness.run_status"),
        ("tinyagents-session", "conversations.threads"),
        ("openhuman-core", "approvals.requests"),
        ("openhuman-core", "devices.paired"),
        ("openhuman-core", "notifications.integration"),
        ("openhuman-core", "task_sources.sources"),
        ("openhuman-core", "artifacts.records"),
        ("openhuman-core", "cost.records"),
        ("openhuman-core", "config.documents"),
        ("openhuman-core", "profiles.registry"),
        ("openhuman-core", "threads.transcript_keys"),
        ("openhuman-core", "subagents.sessions"),
    ] {
        assert!(names.contains(&expected), "{expected:?} missing: {names:?}");
    }
}

#[test]
fn module_collections_are_limited_to_the_named_owner() {
    let mcp = module_collections("mcp");
    assert!(mcp.iter().any(|spec| spec.name == "mcp.servers"));
    assert!(module_collections("no-such-owner").is_empty());
}

#[test]
fn tinyjuice_ccr_blobs_stay_inside_the_juice_owner_namespace() {
    assert_eq!(
        module_collections("juice")
            .iter()
            .map(|collection| collection.name.as_str())
            .collect::<Vec<_>>(),
        [tinyjuice_bus::storage::SAVINGS_COLLECTION]
    );
}

#[test]
fn tiny_artifacts_blob_namespace_composes_with_host_artifact_collections() {
    let registry = registry().expect("the owners compose");
    assert_eq!(
        registry
            .owners()
            .filter(|owner| *owner == "artifacts")
            .count(),
        1
    );

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
