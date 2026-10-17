use super::*;
use tinyagents_session::port::TurnStates;
use tinyagents_session::turn_state::types::{TurnLifecycle, TurnState};

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

#[tokio::test]
async fn streamed_flow_finalizer_settles_the_bridge_workspace_turn_state() {
    let tmp_a = TempDir::new().unwrap();
    let tmp_b = TempDir::new().unwrap();
    let config_a = test_config(&tmp_a);
    let config_b = test_config(&tmp_b);
    let ctx = crate::core::runtime::DispatchCtx::new(
        crate::core::runtime::CoreContext::for_test_with_config(
            crate::core::runtime::DomainSet::full(),
            config_a.clone(),
        ),
    );

    crate::agent::harness::host_definition::AgentDefinitionRegistry::init_global(
        &config_b.workspace_dir,
    )
    .expect("agent registry init");
    let mut agent = crate::agent::OpenHumanSessionHost::from_config_for_agent(
        &ctx,
        &config_b,
        "workflow_builder",
    )
    .expect("build workflow_builder agent");
    let target = FlowStreamTarget {
        thread_id: "flow-stream-workspace-binding".to_string(),
        request_id: "flow-stream-request-binding".to_string(),
    };
    for config in [&config_a, &config_b] {
        crate::agent::session_store::turn_states_for(&config.workspace_dir)
            .put(&TurnState::started(
                target.thread_id.clone(),
                target.request_id.clone(),
                25,
                "2026-10-16T00:00:00Z",
            ))
            .expect("seed turn state");
    }

    attach_flow_progress_bridge(&ctx, &mut agent, &target, "flows_build", &config_b);
    finalize_flow_stream(
        &ctx,
        &target,
        &Err("fixture agent failure".to_string()),
        "fixture prompt",
    )
    .await;

    let state_a = crate::agent::session_store::turn_states_for(&config_a.workspace_dir)
        .get_turn(&target.thread_id, &target.request_id)
        .unwrap()
        .unwrap();
    let state_b = crate::agent::session_store::turn_states_for(&config_b.workspace_dir)
        .get_turn(&target.thread_id, &target.request_id)
        .unwrap()
        .unwrap();
    assert_eq!(state_a.lifecycle, TurnLifecycle::Started);
    assert_eq!(
        state_b.lifecycle,
        TurnLifecycle::Interrupted,
        "the finalizer must settle the exact store installed by the progress bridge"
    );
}
