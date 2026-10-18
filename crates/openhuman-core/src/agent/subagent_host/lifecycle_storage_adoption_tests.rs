use super::*;
use crate::core::runtime::{ContextOverlay, CoreContext, DomainSet};
use tinyagents_orchestration::subagent::{
    PersistedSubagentPause, SubagentOutcomeKind, SubagentPause, SubagentPersistence,
    SubagentResume, SubagentTerminalPersistenceDisposition,
};

fn task() -> SubagentTaskKey {
    SubagentTaskKey {
        root_run_id: "storage-root".into(),
        parent_run_id: "storage-parent".into(),
        thread_id: Some("same-thread".into()),
        task_id: "same-child".into(),
    }
}

fn paused(key: &SubagentTaskKey) -> PersistedSubagentPause {
    let mut outcome = SubagentOutcome::cancelled(&key.task_id);
    outcome.output = "durable partial".into();
    outcome.status = SubagentOutcomeKind::AwaitingInput(SubagentPause {
        reason: "which source?".into(),
        resume: SubagentResume::default(),
    });
    PersistedSubagentPause {
        key: key.clone(),
        outcome,
        replaces: None,
    }
}

fn actor(workspace: &Path, profile: &str) -> std::sync::Arc<CoreContext> {
    let mut config = crate::config::Config::default();
    config.workspace_dir = workspace.to_path_buf();
    config.config_path = workspace.join("config.toml");
    CoreContext::for_test(DomainSet::full(), None).derive_with(
        ContextOverlay::new(config, DomainSet::full(), Default::default())
            .profile(profile)
            .session_agent("same-agent"),
    )
}

#[tokio::test]
async fn production_lifecycle_persistence_follows_the_explicit_actor_scope_on_reopen() {
    let workspace = tempfile::tempdir().unwrap();
    let checkpoints = workspace.path().join(".openhuman/subagent_checkpoints");
    let key = task();
    let alice = actor(workspace.path(), "lifecycle-alice");
    let bob = actor(workspace.path(), "lifecycle-bob");
    CoreContext::scope(alice.clone(), async {
        let adapter = super::super::persistence::OpenHumanPersistence::new(checkpoints.clone());
        adapter.save_pause(paused(&key)).await.unwrap();
        assert_eq!(
            adapter.checkpoint_for_task(&key.task_id).unwrap().output,
            "durable partial"
        );
    })
    .await;
    CoreContext::scope(bob, async {
        let adapter = super::super::persistence::OpenHumanPersistence::new(checkpoints.clone());
        assert!(adapter.load_pause(&key).await.unwrap().is_none());
        assert!(adapter.checkpoint_for_task(&key.task_id).is_err());
    })
    .await;
    CoreContext::scope(alice, async {
        let reopened = super::super::persistence::OpenHumanPersistence::new(checkpoints.clone());
        let durable = reopened.load_pause(&key).await.unwrap().unwrap();
        assert_eq!(durable.output, "durable partial");
        let SubagentOutcomeKind::AwaitingInput(pause) = durable.status else {
            panic!("durable pause")
        };
        let mut terminal = SubagentOutcome::cancelled(&key.task_id);
        terminal.output = "terminal winner".into();
        terminal
            .extensions
            .insert("agent_id".into(), serde_json::json!("researcher"));
        terminal
            .extensions
            .insert("elapsed_ms".into(), serde_json::json!(97));
        assert_eq!(
            reopened
                .record_terminal(&key, &terminal, Some(&pause.resume))
                .await
                .unwrap(),
            SubagentTerminalPersistenceDisposition::Inserted
        );
        let restarted = super::super::persistence::OpenHumanPersistence::new(checkpoints.clone());
        assert!(restarted.load_pause(&key).await.unwrap().is_none());
        // A stale legacy copy may still exist after read-only migration.
        let stale_path = restarted.legacy_pause_path(&key);
        std::fs::create_dir_all(stale_path.parent().unwrap()).unwrap();
        let stale =
            super::super::persistence::OpenHumanPersistence::checkpoint_from_persisted_pause(
                paused(&key),
            )
            .unwrap();
        std::fs::write(&stale_path, serde_json::to_vec(&stale).unwrap()).unwrap();
        assert!(restarted.checkpoint_for_task(&key.task_id).is_err());
        assert!(super::super::load_subagent_checkpoint(&checkpoints, &key.task_id).is_err());
        let terminal = restarted.load_terminal(&key).await.unwrap().unwrap();
        assert_eq!(terminal.output, "terminal winner");
        assert_eq!(terminal.extensions["agent_id"], "researcher");
        assert_eq!(terminal.extensions["elapsed_ms"], 97);
    })
    .await;
}

#[test]
fn legacy_host_kv_pause_is_converted_without_rewriting_the_source_record() {
    let workspace = tempfile::tempdir().unwrap();
    let store = LifecycleStore::open(workspace.path()).unwrap();
    let key = task();
    let mut checkpoint =
        super::super::persistence::OpenHumanPersistence::checkpoint_from_persisted_pause(paused(
            &key,
        ))
        .unwrap();
    checkpoint.agent_id = "researcher".into();
    checkpoint.elapsed_ms = 73;
    let record = LifecycleRecord {
        pause: Some(checkpoint),
        terminal: None,
    };
    store
        .kv
        .put(&LifecycleStore::record_name(&key), &record)
        .unwrap();
    let before = serde_json::to_value(store.record(&key).unwrap()).unwrap();
    let owner = store.neutral_store().unwrap();
    let durable = owner.pause_outcome(&key).unwrap().unwrap();
    assert_eq!(
        durable.extensions["openhuman_checkpoint"]["agent_id"],
        "researcher"
    );
    assert_eq!(durable.extensions["openhuman_checkpoint"]["elapsed_ms"], 73);
    assert_eq!(
        owner.keys_for_task(&key.task_id).unwrap(),
        vec![key.clone()]
    );
    assert_eq!(
        serde_json::to_value(store.record(&key).unwrap()).unwrap(),
        before
    );
}

#[test]
fn owner_file_pause_is_imported_and_the_original_bytes_stay_unchanged() {
    let workspace = tempfile::tempdir().unwrap();
    let key = task();
    let file_store =
        tinyagents_orchestration::subagent::FileSubagentPersistence::new(workspace.path());
    let file = file_store.pause_path(&key).unwrap();
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let bytes = serde_json::to_vec(&paused(&key)).unwrap();
    std::fs::write(&file, &bytes).unwrap();
    let store = LifecycleStore::open(workspace.path()).unwrap();
    let owner = store.neutral_store().unwrap();
    assert_eq!(
        owner.pause_outcome(&key).unwrap().unwrap().output,
        "durable partial"
    );
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}
