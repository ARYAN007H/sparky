use super::*;
use serde_json::json;

fn port() -> (tempfile::TempDir, HostUiPort) {
    let workspace = tempfile::tempdir().expect("workspace");
    let mut config = Config::default();
    config.workspace_dir = workspace.path().to_path_buf();
    (workspace, HostUiPort::new(Arc::new(config)))
}

#[tokio::test]
async fn an_unknown_server_is_unavailable() {
    let (_workspace, port) = port();
    assert!(!port.is_available("nowhere").await);
}

#[tokio::test]
async fn an_unknown_server_has_no_tool_descriptor() {
    let (_workspace, port) = port();
    assert!(port.tool_descriptor("nowhere", "order").await.is_none());
}

#[tokio::test]
async fn reading_from_an_unknown_server_fails() {
    let (_workspace, port) = port();
    assert!(port.read_resource("nowhere", "ui://x").await.is_err());
}

#[tokio::test]
async fn calling_a_tool_on_an_unknown_server_fails() {
    let (_workspace, port) = port();
    assert!(port.call_tool("nowhere", "order", json!({})).await.is_err());
}
