use super::*;

fn pending(home: std::path::PathBuf) -> PendingCleanup {
    PendingCleanup {
        cleanup: None,
        instance: Weak::new(),
        home,
        purge: true,
        retry_running: Default::default(),
    }
}

#[test]
fn confirmed_cleanup_purges_before_releasing_a_dead_agent_reservation() {
    let scratch = tempfile::tempdir().unwrap();
    let home = scratch.path().join("worker");
    std::fs::create_dir(&home).unwrap();
    std::fs::write(home.join("kept-until-cleanup"), "fixture").unwrap();
    let pending = pending(home.clone());
    let registry = crate::runtime::AgentMap::default();
    {
        let mut agents = registry.lock().unwrap();
        agents.insert("worker".into(), pending.instance.clone());
        agents.pending.insert("worker".into(), pending.clone());
    }
    finish_removal(&registry, "worker", &pending).unwrap();
    assert!(!home.exists());
    let agents = registry.lock().unwrap();
    assert!(!agents.contains_key("worker"));
    assert!(!agents.pending.contains_key("worker"));
}

#[test]
fn a_failed_purge_keeps_the_dead_agent_reservation_and_purge_intent() {
    let scratch = tempfile::tempdir().unwrap();
    let home = scratch.path().join("worker");
    std::fs::write(&home, "a file cannot be purged as a directory").unwrap();
    let pending = pending(home.clone());
    let registry = crate::runtime::AgentMap::default();
    {
        let mut agents = registry.lock().unwrap();
        agents.insert("worker".into(), pending.instance.clone());
        agents.pending.insert("worker".into(), pending.clone());
    }
    assert!(matches!(
        finish_removal(&registry, "worker", &pending),
        Err(AgentError::Workspace { .. })
    ));
    assert!(home.exists());
    let agents = registry.lock().unwrap();
    assert!(agents.contains_key("worker"));
    assert!(agents.pending.get("worker").unwrap().purge);
}

#[test]
fn an_already_released_reservation_does_not_purge_a_home() {
    let scratch = tempfile::tempdir().unwrap();
    let home = scratch.path().join("worker");
    std::fs::create_dir(&home).unwrap();
    finish_removal(
        &crate::runtime::AgentMap::default(),
        "worker",
        &pending(home.clone()),
    )
    .unwrap();
    assert!(home.exists());
}
