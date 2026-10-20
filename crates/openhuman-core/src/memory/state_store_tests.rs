use super::*;
use tinymemory_api::{ConsolidateRequest, ItemKind, Namespace, Reach};
use tinymemory_tools::BackgroundJob;

#[tokio::test]
async fn host_memory_state_adapters_use_the_state_kv_port() {
    let tmp = tempfile::tempdir().unwrap();
    let config = crate::memory::test_fixtures::config_in(&tmp);
    let workspace = &config.workspace_dir;

    crate::memory::deletion::enqueue(
        workspace,
        crate::memory::deletion::PendingDeletion::Thread {
            thread_id: "thread-1".into(),
        },
    );
    crate::memory::sources::state::update(workspace, "src-1", |_| {});
    crate::memory::layout_migration::state::save(workspace, &Default::default()).unwrap();
    crate::memory::lifecycle::jobs::enqueue(
        &config,
        &Namespace::ROOT,
        vec![BackgroundJob::BuildBeliefs {
            request: ConsolidateRequest::new(Reach::exact(Namespace::agent("a")))
                .kinds([ItemKind::Conversation]),
        }],
    )
    .await;

    for name in [
        "pending_deletions",
        "sources_state",
        "layout_migration",
        "jobs",
    ] {
        assert!(
            load::<serde_json::Value>(workspace, name)
                .unwrap()
                .is_some(),
            "memory state value {name} is stored through the KV port"
        );
    }
}

#[test]
fn workspace_scopes_keep_retry_queues_stable_and_isolated_on_shared_storage() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let backend = crate::storage::MemoryStorage::new();
    let first_scope = super::workspace_scope(first.path()).unwrap();
    let second_scope = super::workspace_scope(second.path()).unwrap();
    let first_storage = backend.for_scope(&first_scope).unwrap();
    let second_storage = backend.for_scope(&second_scope).unwrap();
    let first_state = crate::storage::state_kv::StateKv::over(
        &first_storage,
        "memory",
        "state",
    );

    first_state
        .put("pending_deletions", &vec!["thread-a".to_string()])
        .unwrap();

    let first_reopened = crate::storage::state_kv::StateKv::over(
        &first_storage,
        "memory",
        "state",
    );
    let second_state = crate::storage::state_kv::StateKv::over(
        &second_storage,
        "memory",
        "state",
    );
    assert_eq!(
        first_reopened
            .get::<Vec<String>>("pending_deletions")
            .unwrap(),
        Some(vec!["thread-a".to_string()])
    );
    assert!(
        second_state
            .get::<Vec<String>>("pending_deletions")
            .unwrap()
            .is_none(),
        "a different workspace cannot read the queue"
    );
}

#[test]
fn state_values_do_not_leak_between_workspaces() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let first_config = crate::memory::test_fixtures::config_in(&first);
    let second_config = crate::memory::test_fixtures::config_in(&second);

    save(&first_config.workspace_dir, "isolation", &"first workspace").unwrap();

    assert_eq!(
        load::<String>(&first_config.workspace_dir, "isolation").unwrap(),
        Some("first workspace".to_string())
    );
    assert!(
        load::<String>(&second_config.workspace_dir, "isolation")
            .unwrap()
            .is_none(),
        "a second workspace must not read the first workspace's state"
    );
}
