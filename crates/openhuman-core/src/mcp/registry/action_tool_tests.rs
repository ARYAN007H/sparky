use super::*;
use serde_json::json;
use tinytools::{PermissionLevel, ToolExposure};

fn presentation(
    server_id: &str,
    name: &str,
    legacy_name: &str,
) -> tinymcp_bus::McpToolPresentation {
    tinymcp_bus::McpToolPresentation {
        name: name.into(),
        legacy_name: legacy_name.into(),
        server_id: server_id.into(),
        remote_name: "readGoals".into(),
        family: "example/weather".into(),
        description: "MCP server Weather Service: Get the current weather forecast".into(),
        parameters: json!({
            "type": "object",
            "properties": { "city": { "type": "string" } },
            "required": ["city"]
        }),
        deferred: true,
        tags: vec![
            "mcp.server:example/weather".into(),
            format!("mcp.server_id:{server_id}"),
            "mcp.tool:readGoals".into(),
        ],
    }
}

#[test]
fn host_adapter_uses_the_module_prepared_name_schema_and_policy_tags() {
    let tools = deferred_connected_tools(
        Arc::new(Config::default()),
        &[presentation(
            "server-1",
            "mcp_weather_read_goals_abc123",
            "legacy-goals",
        )],
    );
    assert_eq!(tools.len(), 1);
    let tool = &tools[0];
    assert_eq!(tool.name(), "mcp_weather_read_goals_abc123");
    assert_eq!(tool.exposure(), ToolExposure::Deferred);
    assert_eq!(tool.permission_level(), PermissionLevel::Execute);
    assert!(tool.external_effect());
    assert_eq!(tool.family(), Some("example/weather"));
    assert_eq!(tool.tags()[0], "mcp.server:example/weather");
    assert_eq!(tool.parameters_schema()["required"], json!(["city"]));
}

#[test]
fn a_recorded_legacy_name_is_restored_as_an_alias() {
    let legacy = "mcp_read_goals_legacy".to_string();
    let recorded: HashSet<String> = [legacy.clone()].into_iter().collect();
    let tools = deferred_connected_tools_with_legacy(
        Arc::new(Config::default()),
        &[presentation(
            "server-1",
            "mcp_weather_read_goals_abc123",
            &legacy,
        )],
        &recorded,
    );
    let names: Vec<&str> = tools.iter().map(|tool| tool.name()).collect();
    assert_eq!(
        names,
        ["mcp_read_goals_legacy", "mcp_weather_read_goals_abc123"]
    );

    let none = deferred_connected_tools_with_legacy(
        Arc::new(Config::default()),
        &[presentation(
            "server-1",
            "mcp_weather_read_goals_abc123",
            &legacy,
        )],
        &HashSet::new(),
    );
    assert_eq!(none.len(), 1);
}

#[test]
fn same_remote_name_on_distinct_servers_keeps_module_names_distinct() {
    let tools = deferred_connected_tools(
        Arc::new(Config::default()),
        &[
            presentation("server-1", "mcp_weather_read_goals_abc123", "legacy-1"),
            presentation("server-2", "mcp_weather_read_goals_def456", "legacy-2"),
        ],
    );
    assert_eq!(tools.len(), 2);
    assert_ne!(tools[0].name(), tools[1].name());
}

#[tokio::test]
async fn unavailable_module_returns_a_tool_error_without_a_host_fallback() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let config = Config {
        workspace_dir: workspace.path().join("workspace"),
        action_dir: workspace.path().join("workspace"),
        config_path: workspace.path().join("config.toml"),
        ..Default::default()
    };
    let tools = deferred_connected_tools(
        Arc::new(config),
        &[presentation(
            "server-1",
            "mcp_weather_read_goals_abc123",
            "legacy-goals",
        )],
    );
    let result = tools[0].execute(json!({ "city": "London" })).await.unwrap();
    assert!(result.is_error);
    assert!(!result.text().is_empty());
}
