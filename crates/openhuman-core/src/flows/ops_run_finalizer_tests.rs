use super::*;

#[tokio::test]
async fn dropping_finalizer_for_terminal_run_does_not_rewrite_flow_summary() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let flow = store::create_flow(
        &config,
        "terminal-finalizer-race".to_string(),
        structurally_valid_graph(trigger_only_graph()),
        false,
        true,
    )
    .unwrap();
    let run_id = "terminal-finalizer-race-run";
    let now = Utc::now().to_rfc3339();
    store::insert_flow_run(&config, run_id, &flow.id, run_id, &now).unwrap();
    assert!(
        store::finish_flow_run(&config, run_id, "completed", &now, &[], &[], None, None,).unwrap()
    );

    let config = Arc::new(config);
    drop(RunRowFinalizer::new(config.clone(), run_id, &flow.id));

    let row = store::get_flow_run(&config, run_id).unwrap().unwrap();
    assert_eq!(row.status, "completed");
    let flow = store::get_flow(&config, &flow.id).unwrap().unwrap();
    assert!(flow.last_status.is_none());
}
