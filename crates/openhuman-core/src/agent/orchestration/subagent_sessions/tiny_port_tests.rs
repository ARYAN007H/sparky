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
