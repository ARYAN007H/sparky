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
    assert_eq!(run.parameters_schema()["required"], json!(["flow_id"]));

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

fn tool_context(
    core: Arc<crate::core::runtime::CoreContext>,
    id: &str,
) -> tinyagents_harness::tool::ToolExecutionContext {
    let run = crate::agent::tinyagents::host::run_context::OpenHumanRunContext::new()
        .with_core(Some(core))
        .into_tinyagents(tinyagents_harness::context::RunConfig::new(id));
    tinyagents_harness::tool::ToolExecutionContext::from_run_context(
        &run,
        tinyagents_harness::ids::CallId::new(format!("{id}-call")),
    )
}

#[tokio::test]
async fn concurrent_tool_callbacks_keep_each_explicit_profile_scope() {
    use crate::core::runtime::{ContextOverlay, CoreContext, DispatchCtx, DomainSet, ToolGroups};

    let workspace = tempfile::tempdir().expect("temporary workspace");
    let mut config = Config::default();
    config.workspace_dir = workspace.path().join("workspace");
    config.action_dir = config.workspace_dir.clone();
    config.config_path = workspace.path().join("config.toml");
    std::fs::create_dir_all(&config.workspace_dir).expect("workspace directory");

    let root = CoreContext::for_test_with_config(DomainSet::full(), config.clone());
    let alice = root.derive_with(
        ContextOverlay::new(config.clone(), DomainSet::full(), ToolGroups::default())
            .profile("copilot-alice"),
    );
    let bob = root.derive_with(
        ContextOverlay::new(config.clone(), DomainSet::full(), ToolGroups::default())
            .profile("copilot-bob"),
    );
    let graph = json!({
        "nodes": [{"id":"trigger","kind":"trigger","name":"Trigger","config":{}}],
        "edges": []
    });
    for (context, name) in [(&alice, "Alice only"), (&bob, "Bob only")] {
        let context = Arc::clone(context);
        let config = config.clone();
        let graph = graph.clone();
        let name = name.to_string();
        CoreContext::scope(context.clone(), async move {
            crate::flows::ops::flows_create(
                &DispatchCtx::new(context),
                &config,
                name,
                graph,
                false,
            )
            .await
            .expect("profile flow creation");
        })
        .await;
    }

    let catalog = CopilotToolCatalog::load(Arc::new(config)).expect("module tool catalog loads");
    let tool = catalog.tool("list_flows").expect("list_flows is catalogued");
    let alice_call = tool_context(alice, "copilot-alice");
    let bob_call = tool_context(bob, "copilot-bob");
    let alice_future = tool.execute_with_context(
        json!({}),
        tinytools::ToolCallOptions::default(),
        Some(&alice_call),
    );
    let bob_future = tool.execute_with_context(
        json!({}),
        tinytools::ToolCallOptions::default(),
        Some(&bob_call),
    );
    let (alice_result, bob_result) = tokio::join!(alice_future, bob_future);
    let alice_result = alice_result.expect("Alice callback result");
    let bob_result = bob_result.expect("Bob callback result");
    assert!(!alice_result.is_error, "{}", alice_result.output());
    assert!(!bob_result.is_error, "{}", bob_result.output());
    assert!(alice_result.output().contains("Alice only"));
    assert!(!alice_result.output().contains("Bob only"));
    assert!(bob_result.output().contains("Bob only"));
    assert!(!bob_result.output().contains("Alice only"));
}
