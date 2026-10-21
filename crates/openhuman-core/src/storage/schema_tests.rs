use super::*;

#[test]
fn every_linked_owner_collection_is_valid_and_unique() {
    let collections = owner_collections().expect("owner collection declarations compose");
    let mut names = std::collections::HashSet::new();
    for spec in &collections {
        spec.validate().expect("owner collection is valid");
        assert!(names.insert(spec.name.as_str()), "duplicate {}", spec.name);
    }
    for expected in [
        "harness.run_status",
        "conversations.threads",
        "approvals.requests",
        "devices.paired",
        "notifications.integration",
        "task_sources.sources",
        "artifacts.records",
        "cost.records",
        "config.documents",
        "profiles.registry",
        "threads.transcript_keys",
        "subagents.sessions",
    ] {
        assert!(names.contains(expected), "{expected} missing: {names:?}");
    }
}

#[test]
fn module_collections_are_limited_to_the_named_owner() {
    let mcp = module_collections("mcp");
    assert!(mcp.iter().any(|spec| spec.name == "mcp.servers"));
    assert!(module_collections("no-such-owner").is_empty());
}
