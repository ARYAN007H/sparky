use super::*;

#[tokio::test]
async fn old_rows_without_checkpoint_identity_remain_readable_without_fabricated_keys() {
    let workspace = tempfile::tempdir().unwrap();
    let store = SubagentSessionStore::new(workspace.path().to_path_buf());
    let row: DurableSubagentSession = serde_json::from_value(serde_json::json!({
        "subagentSessionId": "legacy-session",
        "parentSession": "owner",
        "parentThreadId": "thread",
        "workerThreadId": null,
        "agentId": "researcher",
        "displayName": null,
        "model": null,
        "sandboxMode": "workspace",
        "actionRoot": null,
        "taskKey": "analysis",
        "taskTitle": "Analyze",
        "currentTaskId": "legacy-task",
        "status": "idle",
        "reusable": true,
        "latestHistory": null,
        "latestError": null,
        "createdAt": "2026-01-01T00:00:00Z",
        "updatedAt": "2026-01-01T00:00:00Z",
        "lastUsedAt": "2026-01-01T00:00:00Z"
    }))
    .unwrap();
    assert!(row.lifecycle_key.is_none());
    store.save(&[row]).unwrap();

    let port = OpenHumanTinySubagentSessionStore::new(
        workspace.path().to_path_buf(),
        "owner",
        Some("thread".into()),
    );
    let sessions = port.list_for_parent("owner", Some("thread")).await.unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].subagent_session_id, "legacy-session");
    assert_eq!(sessions[0].current_task_id.as_deref(), Some("legacy-task"));
    assert!(sessions[0].lifecycle_key.is_none());
    assert!(port
        .list_for_parent("foreign-owner", Some("thread"))
        .await
        .unwrap()
        .is_empty());
    assert!(port
        .list_for_parent("owner", Some("other-thread"))
        .await
        .unwrap()
        .is_empty());
    assert_eq!(port.list_for_parent("owner", None).await.unwrap().len(), 1);
    assert!(port.get("legacy-session").await.unwrap().is_some());
    let other_thread = OpenHumanTinySubagentSessionStore::new(
        workspace.path().to_path_buf(),
        "owner",
        Some("other-thread".into()),
    );
    assert!(other_thread.get("legacy-session").await.unwrap().is_none());
}

#[tokio::test]
async fn unthreaded_port_lists_only_unthreaded_rows() {
    let workspace = tempfile::tempdir().unwrap();
    let store = SubagentSessionStore::new(workspace.path().to_path_buf());
    let unthreaded: DurableSubagentSession = serde_json::from_value(serde_json::json!({
        "subagentSessionId": "unthreaded-session",
        "parentSession": "owner",
        "parentThreadId": null,
        "workerThreadId": null,
        "agentId": "researcher",
        "displayName": null,
        "model": null,
        "sandboxMode": "workspace",
        "actionRoot": null,
        "taskKey": "analysis",
        "taskTitle": "No thread",
        "currentTaskId": "task-unthreaded",
        "status": "idle",
        "reusable": true,
        "latestHistory": null,
        "latestError": null,
        "createdAt": "2026-01-01T00:00:00Z",
        "updatedAt": "2026-01-01T00:00:00Z",
        "lastUsedAt": "2026-01-01T00:00:00Z"
    }))
    .unwrap();
    let mut threaded = unthreaded.clone();
    threaded.subagent_session_id = "threaded-session".into();
    threaded.parent_thread_id = Some("thread-a".into());
    store.save(&[unthreaded, threaded]).unwrap();

    let port =
        OpenHumanTinySubagentSessionStore::new(workspace.path().to_path_buf(), "owner", None);
    let sessions = port.list_for_parent("owner", None).await.unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].subagent_session_id, "unthreaded-session");
    assert!(port
        .list_for_parent("owner", Some("thread-a"))
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn upsert_rejects_a_selector_or_lifecycle_key_outside_bound_scope() {
    let workspace = tempfile::tempdir().unwrap();
    let store = SubagentSessionStore::new(workspace.path().to_path_buf());
    let port = OpenHumanTinySubagentSessionStore::new(
        workspace.path().to_path_buf(),
        "owner",
        Some("thread-a".into()),
    );
    let key = |thread_id: Option<&str>, task_id: &str| {
        tinyagents_orchestration::subagent::SubagentTaskKey {
            root_run_id: "root".into(),
            parent_run_id: "parent".into(),
            thread_id: thread_id.map(str::to_owned),
            task_id: task_id.into(),
        }
    };
    let upsert =
        |parent_session: &str,
         parent_thread_id: Option<&str>,
         lifecycle_key: tinyagents_orchestration::subagent::SubagentTaskKey| {
            TinyUpsert {
                selector: TinySelector {
                    parent_session: parent_session.into(),
                    parent_thread_id: parent_thread_id.map(str::to_owned),
                    agent_id: "researcher".into(),
                    model: None,
                    sandbox_mode: "workspace".into(),
                    action_root: None,
                    task_key: "analysis".into(),
                },
                lifecycle_key,
                display_name: None,
                task_title: "Task".into(),
                worker_thread_id: None,
                task_id: "task-1".into(),
                force_fresh: true,
            }
        };

    assert!(port
        .upsert_running(upsert(
            "foreign",
            Some("thread-a"),
            key(Some("thread-a"), "task-1")
        ))
        .await
        .is_err());
    assert!(port
        .upsert_running(upsert(
            "owner",
            Some("thread-b"),
            key(Some("thread-b"), "task-1")
        ))
        .await
        .is_err());
    assert!(port
        .upsert_running(upsert(
            "owner",
            Some("thread-a"),
            key(Some("thread-b"), "task-1")
        ))
        .await
        .is_err());
    assert!(port
        .upsert_running(upsert(
            "owner",
            Some("thread-a"),
            key(Some("thread-a"), "other-task")
        ))
        .await
        .is_err());
    assert!(store.load().unwrap().is_empty());
}
