use super::*;
use serde_json::json;

#[test]
fn meta_keys_include_ui_members() {
    let keys = meta_keys(&json!({"ui": {"resourceUri": "ui://a", "visibility": ["app"]}, "x": 1}));
    assert!(keys.contains(&"ui".to_string()));
    assert!(keys.contains(&"ui.resourceUri".to_string()));
    assert!(keys.contains(&"x".to_string()));
}

fn remote_tool(value: serde_json::Value) -> tinymcp_bus::McpRemoteTool {
    serde_json::from_value(value).expect("remote tool")
}

#[test]
fn meta_keys_of_a_non_object_are_empty() {
    assert!(meta_keys(&json!("text")).is_empty());
    assert!(meta_keys(&json!(null)).is_empty());
}

#[test]
fn logging_a_tool_and_resources_does_not_expose_values() {
    log_tool(
        "shop",
        "order",
        &json!({"ui": {"resourceUri": "ui://shop/cart"}}),
    );
    log_resource("shop", "ui://shop/cart", Some("text/html"));
    log_resource("shop", "file:///secret", None);
}

#[test]
fn configured_server_logs_every_tool_with_meta() {
    let tools = vec![
        remote_tool(json!({
            "name": "order",
            "inputSchema": {"type": "object"},
            "_meta": {"ui": {"resourceUri": "ui://shop/cart"}}
        })),
        remote_tool(json!({"name": "plain", "inputSchema": {"type": "object"}})),
    ];
    log_configured_server("shop", &tools);
}

#[tokio::test]
async fn installed_server_with_no_connection_logs_nothing_to_list() {
    let workspace = tempfile::tempdir().unwrap();
    let mut config = crate::config::Config::default();
    config.workspace_dir = workspace.path().to_path_buf();
    let host = host::for_config(&config).expect("host");
    log_installed_server(&host, "missing").await;
    log_all_installed(Arc::clone(&host)).await;
}

#[tokio::test]
async fn spawning_a_log_without_a_runtime_host_is_a_no_op() {
    spawn_log_installed("missing".to_string());
}
