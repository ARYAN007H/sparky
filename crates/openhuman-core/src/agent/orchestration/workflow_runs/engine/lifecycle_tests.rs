use super::*;
use serde_json::json;

#[tokio::test]
async fn stop_fences_the_driver_and_makes_a_running_phase_resumable() {
    let workspace = tempfile::tempdir().expect("workspace");
    let config = Config {
        workspace_dir: workspace.path().to_path_buf(),
        ..Config::default()
    };
    let run = crate::storage::sessions::with_ledger(&config.workspace_dir, |ledger| {
        let ledger_arg_0 = WorkflowRunUpsert {
            id: "stop-resume-lifecycle".into(),
            definition_id: "parallel_research_cross_check".into(),
            parent_thread_id: None,
            input: json!({"question":"q"}),
            phase_states: json!({
                "decompose": {"status":"completed", "outputs": []},
                "research": {"status":"running", "outputs": []}
            }),
            child_run_ids: vec!["live-child".into()],
            status: WorkflowRunStatus::Running,
            summary: None,
            started_at: None,
            completed_at: None,
        };
        async move { ledger.upsert_workflow_run(ledger_arg_0).await }
    })
    .expect("seed run");

    let stopped = stop_workflow_run(&config, &run.id)
        .await
        .expect("stop succeeds")
        .expect("row exists");
    assert_eq!(stopped.status, WorkflowRunStatus::Interrupted);
    assert_eq!(stopped.phase_states["research"]["status"], json!("pending"));
    assert!(stopped.lease_owner.is_none());
    assert!(
        stopped.revision > run.revision,
        "stop must fence the old driver"
    );
}

#[tokio::test]
async fn stale_workflow_generation_cannot_fail_its_replacement() {
    let workspace = tempfile::tempdir().expect("workspace");
    let config = Config {
        workspace_dir: workspace.path().to_path_buf(),
        ..Config::default()
    };
    let run_id = format!("stale-generation-{}", uuid::Uuid::new_v4());
    let seeded = crate::storage::sessions::with_ledger(&config.workspace_dir, |ledger| {
        let row = WorkflowRunUpsert {
            id: run_id.clone(),
            definition_id: "parallel_research_cross_check".into(),
            parent_thread_id: None,
            input: json!({"question":"q"}),
            phase_states: json!({"research":{"status":"pending","outputs":[]}}),
            child_run_ids: Vec::new(),
            status: WorkflowRunStatus::Running,
            summary: None,
            started_at: None,
            completed_at: None,
        };
        async move { ledger.upsert_workflow_run(row).await }
    })
    .expect("seed run");

    let supervisor = workflow_supervisor().clone();
    let old = supervisor.start_scoped_with_handle(
        run_id.clone(),
        |token| async move { token.cancelled().await },
        |future| future,
        |future| future,
        |_handle, future| {
            tokio::spawn(future);
        },
    );
    let current = supervisor.start_scoped_with_handle(
        run_id.clone(),
        |token| async move { token.cancelled().await },
        |future| future,
        |future| future,
        |_handle, future| {
            tokio::spawn(future);
        },
    );
    assert!(!supervisor.is_current(&old));
    assert!(supervisor.is_current(&current));

    record_failed_if_current(
        &config,
        &run_id,
        &old,
        seeded.revision,
        "stale sentinel".into(),
    );
    let unchanged = crate::storage::sessions::with_ledger(&config.workspace_dir, |ledger| {
        let id = run_id.clone();
        async move { ledger.get_workflow_run(&id).await }
    })
    .expect("read row")
    .expect("run exists");
    assert_eq!(unchanged.revision, seeded.revision);
    assert_ne!(unchanged.summary.as_deref(), Some("stale sentinel"));

    let stopped = stop_workflow_run(&config, &run_id)
        .await
        .expect("stop succeeds")
        .expect("run exists");
    assert_eq!(stopped.status, WorkflowRunStatus::Interrupted);
    assert!(current.cancellation_token().is_cancelled());
}
