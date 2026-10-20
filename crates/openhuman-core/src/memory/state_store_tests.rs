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
