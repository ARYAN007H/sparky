use super::*;

fn insert_parked_run(config: &Config, flow_id: &str, run_id: &str) {
    let now = Utc::now().to_rfc3339();
    store::insert_flow_run(config, run_id, flow_id, run_id, &now).unwrap();
    assert!(store::finish_flow_run(
        config,
        run_id,
        "pending_approval",
        &now,
        &[],
        &["gate".into()],
        None,
        Some("parked-graph"),
    )
    .unwrap());
}

#[tokio::test]
async fn duplicate_resume_preserves_the_registered_run_and_parked_checkpoint() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let created = flows_create(
        &config,
        "duplicate-resume".to_string(),
        approval_gated_graph(),
        false,
    )
    .await
    .unwrap();
    let parked = flows_run(
        &crate::core::runtime::DispatchCtx::ambient(),
        &config,
        &created.value.id,
        json!({ "x": 1 }),
        serde_json::Map::new(),
        FlowRunTrigger::Rpc,
    )
    .await
    .unwrap();
    let run_id = parked.value["thread_id"].as_str().unwrap().to_string();
    let pending: Vec<String> =
        serde_json::from_value(parked.value["pending_approvals"].clone()).unwrap();
    assert!(!pending.is_empty());

    // Resume A has registered its cancellation token but has not yet flipped
    // the durable row from pending_approval to running. Resume B must refuse
    // to replace that token or claim the checkpoint out from under A.
    let (active_token, active_guard) = run_registry::register(&run_id);
    let duplicate = flows_resume(
        &crate::core::runtime::DispatchCtx::ambient(),
        &config,
        &created.value.id,
        &run_id,
        pending.clone(),
        vec![],
    )
    .await
    .expect_err("a second resume must refuse an already registered run");
    assert!(duplicate.contains("in flight"), "{duplicate}");
    assert!(!active_token.is_cancelled());
    assert!(run_registry::is_in_flight(&run_id));

    let row = store::get_flow_run(&config, &run_id).unwrap().unwrap();
    assert_eq!(row.status, "pending_approval");
    assert_eq!(row.pending_approvals, pending);
    let checkpointer = crate::flows::tinyflows::open_flow_checkpointer(&config).unwrap();
    let checkpoints = crate::storage::block_on_anyhow(async move {
        Ok(checkpointer.get_thread(&run_id).await?.len())
    })
    .unwrap();
    assert!(
        checkpoints > 0,
        "the active resume's checkpoint must remain stored"
    );

    drop(active_guard);
    let resumed = flows_resume(
        &crate::core::runtime::DispatchCtx::ambient(),
        &config,
        &created.value.id,
        &run_id,
        pending,
        vec![],
    )
    .await
    .expect("the preserved checkpoint must still resume after the active owner exits");
    assert_eq!(resumed.value["pending_approvals"], json!([]));
    assert_eq!(
        store::get_flow_run(&config, &run_id)
            .unwrap()
            .unwrap()
            .status,
        "completed"
    );
}

#[tokio::test]
async fn compatibility_refusal_does_not_label_a_resume_claimed_run_failed() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let flow = store::create_flow(
        &config,
        "compatibility-race".to_string(),
        structurally_valid_graph(trigger_only_graph()),
        false,
        true,
    )
    .unwrap();
    let run_id = "compatibility-race-run";
    insert_parked_run(&config, &flow.id, run_id);
    assert!(store::mark_run_resuming(&config, run_id).unwrap());

    assert!(
        !run_management::settle_parked_run(
            &config,
            run_id,
            &flow.id,
            "failed",
            "unsupported graph topology",
        )
        .await
    );

    let row = store::get_flow_run(&config, run_id).unwrap().unwrap();
    assert_eq!(row.status, "running");
    let flow = store::get_flow(&config, &flow.id).unwrap().unwrap();
    assert!(flow.last_status.is_none());
}
