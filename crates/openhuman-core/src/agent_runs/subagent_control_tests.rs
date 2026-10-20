use super::*;
use crate::core::runtime::DispatchCtx;

#[test]
fn registered_controllers_match_schemas() {
    let schemas = all_controller_schemas();
    let registered = all_registered_controllers();
    assert_eq!(schemas.len(), registered.len());
    assert_eq!(schemas.len(), 1);
    assert_eq!(schema_for("subagent_cancel").namespace, "subagent");
    assert_eq!(schema_for("subagent_cancel").function, "cancel");
}

#[test]
fn require_str_rejects_blank_and_missing() {
    let mut params = Map::new();
    assert!(require_str(&params, "taskId").is_err());
    params.insert("taskId".into(), json!("   "));
    assert!(require_str(&params, "taskId").is_err());
    params.insert("taskId".into(), json!("sub-1"));
    assert_eq!(require_str(&params, "taskId").unwrap(), "sub-1");
    // Whitespace-padded ids are trimmed so they match the registry key.
    params.insert("taskId".into(), json!("  sub-1  "));
    assert_eq!(require_str(&params, "taskId").unwrap(), "sub-1");
}

#[tokio::test]
async fn cancel_unknown_task_is_a_noop_false() {
    let _lock = crate::config::TEST_ENV_LOCK.lock().await;
    let mut params = Map::new();
    params.insert("taskId".into(), json!("sub-does-not-exist"));
    let out = handle_subagent_cancel(DispatchCtx::process_default(), params)
        .await
        .expect("handler ok");
    // Outcome wraps the payload under `data`.
    let cancelled = out
        .get("data")
        .and_then(|d| d.get("cancelled"))
        .or_else(|| out.get("cancelled"))
        .and_then(Value::as_bool);
    assert_eq!(cancelled, Some(false));
    // Nothing known about the task: the card must not claim it succeeded.
    assert_eq!(field(&out, "outcome"), Some(json!("unknown")));
}

fn field(out: &Value, name: &str) -> Option<Value> {
    out.get("data")
        .and_then(|d| d.get(name))
        .or_else(|| out.get(name))
        .cloned()
}

/// A late Cancel on a legacy failed run reports its outcome and leaves the
/// durable session alone, instead of rewriting it as "cancelled by user".
#[tokio::test]
async fn cancel_of_a_legacy_failed_run_reports_its_outcome_and_rewrites_nothing() {
    use crate::agent::orchestration::subagent_sessions::{
        mark_failed, mark_finished, SubagentSessionStore,
    };
    use crate::core::runtime::{CoreContext, DispatchCtx, DomainSet};
    use std::sync::Arc;

    let _lock = crate::config::TEST_ENV_LOCK.lock().await;
    let workspace = tempfile::tempdir().expect("tempdir");
    // Snapshot the durable session store before and after: a real cancel
    // would `mark_failed` the session and rewrite it.
    let store = SubagentSessionStore::new(workspace.path().to_path_buf());
    let seeded: Vec<crate::agent::orchestration::subagent_sessions::DurableSubagentSession> =
        serde_json::from_value(json!([{
            "subagentSessionId": "subsess-sub-rpc-done",
            "parentSession": "session-rpc",
            "parentThreadId": "thread-rpc",
            "workerThreadId": null,
            "agentId": "agent_memory",
            "displayName": null,
            "model": null,
            "sandboxMode": "workspace",
            "actionRoot": null,
            "taskKey": "task",
            "taskTitle": "Task",
            "currentTaskId": "sub-rpc-done",
            "status": "running",
            "reusable": true,
            "latestHistory": null,
            "latestError": null,
            "createdAt": "2026-01-01T00:00:00Z",
            "updatedAt": "2026-01-01T00:00:00Z",
            "lastUsedAt": "2026-01-01T00:00:00Z"
        }]))
        .expect("session fixture");
    store.save(&seeded).expect("seed store");
    mark_failed(
        &store,
        "subsess-sub-rpc-failed",
        "sub-rpc-failed",
        "boom".into(),
    )
    .expect("failed driver row");
    let before = serde_json::to_string(&store.load().expect("load seeded")).expect("encode");
    let mut config = crate::config::Config::default();
    config.workspace_dir = workspace.path().to_path_buf();
    let core = CoreContext::for_test_with_config(DomainSet::full(), config);
    let ctx = DispatchCtx::new(Arc::clone(&core));
    let mut params = Map::new();
    params.insert("taskId".into(), json!("sub-rpc-failed"));
    let out = handle_subagent_cancel(ctx, params)
        .await
        .expect("handler ok");
    assert_eq!(field(&out, "cancelled"), Some(json!(false)));
    assert_eq!(field(&out, "outcome"), Some(json!("failed")));
    assert_eq!(
        serde_json::to_string(&store.load().expect("store still readable")).expect("encode"),
        before,
        "a finished run's session must not be rewritten as cancelled"
    );
}

#[tokio::test]
async fn cancelling_a_paused_driver_queues_a_failed_delivery_with_reason() {
    use crate::agent::orchestration::background_completions::{
        forget_workspace_for_test, pending_for,
    };
    use crate::agent::orchestration::subagent_sessions::{
        mark_finished, upsert_running, DriverTaskCancellation, DurableSubagentStatus,
        SubagentSessionSelector, SubagentSessionStore, SubagentSessionUpsert,
    };
    use tinyagents_orchestration::subagent::SubagentTaskKey;
    use tinyagents_tasks::CompletionStatus;

    let _lock = crate::config::TEST_ENV_LOCK.lock().await;
    let workspace = tempfile::tempdir().expect("workspace");
    let workspace_dir = workspace.path().to_path_buf();
    let parent_session = "paused-cancel-parent";
    let parent_thread = Some("paused-cancel-thread".to_string());
    let task_id = "paused-cancel-public-id";
    let key = SubagentTaskKey {
        root_run_id: "paused-cancel-root".into(),
        parent_run_id: parent_session.into(),
        thread_id: parent_thread.clone(),
        task_id: task_id.into(),
    };
    let store = SubagentSessionStore::new(workspace_dir.clone());
    let session = upsert_running(
        &store,
        SubagentSessionUpsert {
            selector: SubagentSessionSelector {
                parent_session: parent_session.into(),
                parent_thread_id: parent_thread.clone(),
                agent_id: "researcher".into(),
                model: None,
                sandbox_mode: "read_only".into(),
                action_root: None,
                task_key: "paused cancellation".into(),
            },
            display_name: None,
            task_title: "paused cancellation".into(),
            worker_thread_id: None,
            task_id: task_id.into(),
            lifecycle_key: Some(key.clone()),
            parent_epoch: Some(1),
        },
        None,
    )
    .expect("session inserted");
    mark_finished(
        &store,
        &session.subagent_session_id,
        task_id,
        &crate::agent::subagent_host::SubagentRunStatus::AwaitingUser {
            question: "Need input".into(),
            options: None,
            checkpoint: None,
        },
        Vec::new(),
    )
    .expect("session paused");
    let observed = DriverTaskCancellation {
        task_id: task_id.into(),
        agent_id: "researcher".into(),
        parent_session: parent_session.into(),
        subagent_session_id: session.subagent_session_id.clone(),
        workspace_dir: workspace_dir.clone(),
        parent_thread_id: parent_thread.clone(),
        parent_epoch: Some(1),
        already_finished: None,
        was_paused: true,
        lifecycle_key: Some(key),
    };

    forget_workspace_for_test(&workspace_dir);
    assert!(settle_paused_driver_cancel(
        &DispatchCtx::process_default(),
        &observed,
        observed.parent_epoch,
        Some("user requested stop")
    )
    .await
    .expect("paused cancellation settles"));

    let row = store.load().unwrap().remove(0);
    assert_eq!(row.status, DurableSubagentStatus::Failed);
    let delivery = pending_for(&workspace_dir, parent_thread.as_deref().unwrap());
    assert_eq!(delivery.len(), 1);
    assert_eq!(delivery[0].status, CompletionStatus::Failed);
    assert!(delivery[0].result.text.contains("[SUBAGENT_FAILED]"));
    assert!(delivery[0].result.text.contains("user requested stop"));
}
