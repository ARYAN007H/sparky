use super::*;

fn insert_parked_run(config: &Config, flow_id: &str, run_id: &str) {
    let now = Utc::now().to_rfc3339();
    store::insert_flow_run(config, run_id, flow_id, run_id, &now).unwrap();
    assert!(
        store::finish_flow_run(
            config,
            run_id,
            "pending_approval",
            &now,
            &[],
            &["gate".into()],
            None,
            Some("parked-graph"),
        )
        .unwrap()
    );
}

async fn park_approval_flow(config: &Config, name: &str) -> (String, String, Vec<String>) {
    let created = flows_create(config, name.to_string(), approval_gated_graph(), false)
        .await
        .unwrap();
    let parked = flows_run(
        &crate::core::runtime::DispatchCtx::ambient(),
        config,
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
    (created.value.id, run_id, pending)
}

async fn checkpoint_count(config: &Config, thread_id: &str) -> usize {
    let checkpointer = crate::flows::tinyflows::open_flow_checkpointer(config).unwrap();
    let thread_id = thread_id.to_string();
    crate::storage::block_on_anyhow(
        async move { Ok(checkpointer.get_thread(&thread_id).await?.len()) },
    )
    .unwrap()
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
    let checkpoint_thread = run_id.clone();
    let checkpoints = crate::storage::block_on_anyhow(async move {
        Ok(checkpointer.get_thread(&checkpoint_thread).await?.len())
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
async fn stale_graph_refusal_does_not_settle_a_resume_claimed_after_its_snapshot() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let (flow_id, run_id, pending) = park_approval_flow(&config, "stale-resume-race").await;
    let summary_before = store::get_flow(&config, &flow_id)
        .unwrap()
        .unwrap()
        .last_status;
    let mut changed_graph = approval_gated_graph();
    changed_graph["nodes"][2]["name"] = json!("Downstream changed after park");
    let flow = store::get_flow(&config, &flow_id).unwrap().unwrap();
    store::update_flow_graph(
        &config,
        &flow_id,
        flow.name,
        structurally_valid_graph(changed_graph),
        flow.require_approval,
        None,
        false,
        None,
    )
    .unwrap();

    let mut owner = None;
    let error = {
        let mut claim_resume = || {
            owner = Some(
                run_registry::try_register(&run_id)
                    .expect("the competing resume must reserve the parked run"),
            );
            assert!(store::mark_run_resuming(&config, &run_id).unwrap());
        };
        resume::flows_resume_with_refusal_hook(
            &crate::core::runtime::DispatchCtx::ambient(),
            &config,
            &flow_id,
            &run_id,
            pending,
            vec![],
            &mut claim_resume,
        )
        .await
        .unwrap_err()
    };
    assert!(error.contains(GRAPH_CHANGED_SINCE_PARK_ERROR), "{error}");

    let (token, guard) = owner.take().unwrap();
    assert!(!token.is_cancelled());
    assert!(run_registry::is_in_flight(&run_id));
    let row = store::get_flow_run(&config, &run_id).unwrap().unwrap();
    assert_eq!(row.status, "running");
    let flow = store::get_flow(&config, &flow_id).unwrap().unwrap();
    assert_eq!(flow.last_status, summary_before);
    assert!(checkpoint_count(&config, &run_id).await > 0);

    let cancel = flows_cancel_run(&config, &run_id).await.unwrap();
    assert_eq!(cancel.value["was_in_flight"], true);
    assert!(token.is_cancelled());
    assert_eq!(
        store::get_flow_run(&config, &run_id)
            .unwrap()
            .unwrap()
            .status,
        "running"
    );
    assert!(checkpoint_count(&config, &run_id).await > 0);
    drop(guard);
}

#[tokio::test]
async fn compatibility_refusal_does_not_settle_a_resume_claimed_after_its_snapshot() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let (flow_id, run_id, pending) = park_approval_flow(&config, "compatibility-resume-race").await;
    let flow = store::get_flow(&config, &flow_id).unwrap().unwrap();
    let incompatible_graph = structurally_valid_graph(nested_conditional_fan_in_graph());
    store::update_flow_graph(
        &config,
        &flow_id,
        flow.name,
        incompatible_graph.clone(),
        flow.require_approval,
        None,
        false,
        None,
    )
    .unwrap();
    let current_flow = store::get_flow(&config, &flow_id).unwrap().unwrap();
    let hash = compute_graph_hash(&incompatible_graph, current_flow.require_approval).unwrap();
    let row = store::get_flow_run(&config, &run_id).unwrap().unwrap();
    assert!(
        store::finish_flow_run(
            &config,
            &run_id,
            "pending_approval",
            row.finished_at.as_deref().unwrap_or(&row.started_at),
            &row.steps,
            &pending,
            None,
            Some(&hash),
        )
        .unwrap()
    );
    let summary_before = current_flow.last_status;

    let mut owner = None;
    let error = {
        let mut claim_resume = || {
            owner = Some(
                run_registry::try_register(&run_id)
                    .expect("the competing resume must reserve the parked run"),
            );
            assert!(store::mark_run_resuming(&config, &run_id).unwrap());
        };
        resume::flows_resume_with_refusal_hook(
            &crate::core::runtime::DispatchCtx::ambient(),
            &config,
            &flow_id,
            &run_id,
            pending,
            vec![],
            &mut claim_resume,
        )
        .await
        .unwrap_err()
    };
    assert!(
        error.contains(UNSUPPORTED_NESTED_CONDITIONAL_FAN_IN),
        "{error}"
    );

    let (token, guard) = owner.take().unwrap();
    assert!(!token.is_cancelled());
    assert!(run_registry::is_in_flight(&run_id));
    let row = store::get_flow_run(&config, &run_id).unwrap().unwrap();
    assert_eq!(row.status, "running");
    let flow = store::get_flow(&config, &flow_id).unwrap().unwrap();
    assert_eq!(flow.last_status, summary_before);
    assert!(checkpoint_count(&config, &run_id).await > 0);

    let cancel = flows_cancel_run(&config, &run_id).await.unwrap();
    assert_eq!(cancel.value["was_in_flight"], true);
    assert!(token.is_cancelled());
    assert_eq!(
        store::get_flow_run(&config, &run_id)
            .unwrap()
            .unwrap()
            .status,
        "running"
    );
    assert!(checkpoint_count(&config, &run_id).await > 0);
    drop(guard);
}

#[tokio::test]
async fn cancel_after_a_parked_snapshot_does_not_drop_a_resume_starting() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let (flow_id, run_id, _) = park_approval_flow(&config, "cancel-resume-start-race").await;
    let observed = store::get_flow_run(&config, &run_id).unwrap().unwrap();
    assert_eq!(observed.status, "pending_approval");
    let summary_before = store::get_flow(&config, &flow_id)
        .unwrap()
        .unwrap()
        .last_status;

    let mut owner = None;
    let error = {
        let mut claim_resume = || {
            owner = Some(
                run_registry::try_register(&run_id)
                    .expect("the starting resume must reserve the parked run"),
            );
            assert!(store::mark_run_resuming(&config, &run_id).unwrap());
        };
        run_management::flows_cancel_run_with_snapshot(
            &config,
            &run_id,
            observed,
            &mut claim_resume,
        )
        .await
        .unwrap_err()
    };
    assert!(
        error.contains("settled before it could be cancelled"),
        "{error}"
    );

    let (token, guard) = owner.take().unwrap();
    assert!(!token.is_cancelled());
    assert!(run_registry::is_in_flight(&run_id));
    let row = store::get_flow_run(&config, &run_id).unwrap().unwrap();
    assert_eq!(row.status, "running");
    let flow = store::get_flow(&config, &flow_id).unwrap().unwrap();
    assert_eq!(flow.last_status, summary_before);
    assert!(checkpoint_count(&config, &run_id).await > 0);

    let cancel = flows_cancel_run(&config, &run_id).await.unwrap();
    assert_eq!(cancel.value["was_in_flight"], true);
    assert!(token.is_cancelled());
    assert_eq!(
        store::get_flow_run(&config, &run_id)
            .unwrap()
            .unwrap()
            .status,
        "running"
    );
    assert!(checkpoint_count(&config, &run_id).await > 0);
    drop(guard);
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
