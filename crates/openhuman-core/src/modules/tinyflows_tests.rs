use super::*;

#[test]
fn the_disabled_message_names_the_component_and_the_switch() {
    let message = disabled_message();
    assert!(
        message.starts_with("COMPONENT_DISABLED:tinyflows"),
        "{message}"
    );
    assert!(message.contains("[components] tinyflows = true"));
}

#[test]
fn the_module_is_available_only_while_its_component_is_on() {
    let mut config = Config::default();
    assert_eq!(available(&config), cfg!(feature = "modules"));
    config.components.tinyflows = false;
    assert!(!available(&config));
}

#[tokio::test]
async fn a_switched_off_component_never_loads_the_module() {
    let mut config = Config::default();
    config.components.tinyflows = false;
    let error = proxy(&config).await.unwrap_err();
    assert!(error.starts_with("COMPONENT_DISABLED:tinyflows"), "{error}");
    let error = run_proxy(&config).await.unwrap_err();
    assert!(error.starts_with("COMPONENT_DISABLED:tinyflows"), "{error}");
    let error = copilot_proxy(&config).await.unwrap_err();
    assert!(error.starts_with("COMPONENT_DISABLED:tinyflows"), "{error}");
}

#[tokio::test]
async fn the_schedule_object_answers_on_the_contracts_names() {
    let config = Config::default();
    let proxy = proxy(&config).await.expect("module attaches");
    let millis: i64 = proxy
        .call(
            tinyflows_bus::schedule::members::PARSE_HUMAN_DELAY,
            ("30m",),
        )
        .await
        .unwrap();
    assert_eq!(millis, 30 * 60 * 1000);
}

#[tokio::test]
async fn the_copilot_object_answers_tool_catalog_over_its_bus_contract() {
    let config = Config::default();
    let proxy = copilot_proxy(&config)
        .await
        .expect("module copilot interface attaches");
    let tools: Vec<tinyflows_bus::copilot::ToolDescriptor> = proxy
        .call(
            "ToolCatalog",
            (tinyflows_bus::copilot::ToolCatalogRequest::default(),),
        )
        .await
        .expect("copilot catalogue call succeeds");
    let names: Vec<_> = tools.iter().map(|tool| tool.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "propose_workflow",
            "revise_workflow",
            "edit_workflow",
            "validate_workflow",
            "get_flow_history",
            "list_flow_runs",
            "resume_flow_run",
            "cancel_flow_run",
            "create_workflow",
            "duplicate_flow",
            "list_flows",
            "get_flow",
            "get_flow_run",
            "list_agent_definitions",
            "list_node_kinds",
            "get_node_kind_contract",
            "dry_run_workflow",
            "run_flow",
            "save_workflow",
            "suggest_workflows",
        ]
    );
    let run_flow = tools.iter().find(|tool| tool.name == "run_flow").unwrap();
    assert_eq!(run_flow.permission, "Write");
    assert!(run_flow.external_effect);
}
