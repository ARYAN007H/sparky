use super::*;

use tinyagents_harness::store::InMemoryStore;

#[test]
fn only_root_orchestrator_turns_get_the_block() {
    assert!(applies(false, Some("orchestrator")));
    assert!(!applies(true, Some("orchestrator")), "a sub-agent run");
    assert!(!applies(false, Some("planner")));
    assert!(!applies(false, None));
}

#[test]
fn install_needs_the_scope_and_a_store() {
    let store: Arc<dyn Store> = Arc::new(InMemoryStore::default());
    let installed = |store: Option<Arc<dyn Store>>, is_subagent: bool, agent: Option<&str>| {
        let mut harness = AgentHarness::<(), ()>::new();
        let before = harness.middleware().len();
        install(
            &mut harness,
            store.map(|store| (store, "standalone".to_string())),
            is_subagent,
            agent,
        );
        harness.middleware().len() - before
    };
    assert_eq!(
        installed(Some(store.clone()), false, Some("orchestrator")),
        1
    );
    assert_eq!(installed(None, false, Some("orchestrator")), 0);
    assert_eq!(
        installed(Some(store.clone()), true, Some("orchestrator")),
        0
    );
    assert_eq!(installed(Some(store), false, Some("planner")), 0);
}
