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
