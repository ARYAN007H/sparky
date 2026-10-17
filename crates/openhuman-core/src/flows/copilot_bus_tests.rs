use super::*;
use serde_json::json;
use tinytools::Tool as _;

#[tokio::test]
async fn module_tool_descriptors_preserve_run_and_sandbox_safety_metadata() {
    let config = Arc::new(Config::default());
    let catalog = CopilotToolCatalog::load(config).expect("module tool catalog loads");

    let run = catalog.tool("run_flow").expect("run_flow is catalogued");
    assert_eq!(run.permission_level(), PermissionLevel::Write);
    assert!(run.external_effect());
    assert_eq!(
        run.parameters_schema()["required"],
        json!(["flow_id"])
    );

    let dry_run = catalog
        .tool("dry_run_workflow")
        .expect("dry_run_workflow is catalogued");
    assert_eq!(dry_run.permission_level(), PermissionLevel::None);
    assert!(!dry_run.external_effect());
}

#[tokio::test]
async fn model_arguments_cannot_replace_the_scoped_host_callback_address() {
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let mut config = Config::default();
    config.workspace_dir = workspace.path().to_path_buf();
    config.config_path = workspace.path().join("config.toml");
    crate::agent::harness::host_definition::AgentDefinitionRegistry::init_global(
        &config.workspace_dir,
    )
    .expect("agent registry initializes");

    let catalog = CopilotToolCatalog::load(Arc::new(config)).expect("module tool catalog loads");
    let tool = catalog
        .tool("list_agent_definitions")
        .expect("agent listing tool is catalogued");
    let result = tool
        .execute(json!({
            "host": {
                "bus_name": "ai.tinyhumans.attacker",
                "object_path": "/attacker/copilot-host"
            }
        }))
        .await
        .expect("the outer invocation supplies its own host callback");
    assert!(
        !result.is_error,
        "model arguments must not redirect the scoped callback: {}",
        result.output()
    );
    let output: Value = serde_json::from_str(&result.output()).expect("tool JSON result");
    assert!(output.get("agent_definitions").is_some(), "{output}");
}
