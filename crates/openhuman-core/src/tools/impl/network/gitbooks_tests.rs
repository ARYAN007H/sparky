use super::*;

#[cfg(feature = "modules")]
use crate::modules::client::ModuleClient;

#[cfg(feature = "modules")]
#[derive(Clone, Default)]
struct MockProviderState {
    calls: std::sync::Arc<std::sync::Mutex<Vec<Value>>>,
}

#[cfg(feature = "modules")]
#[derive(Clone)]
struct MockTinyMcp {
    provider_state: MockProviderState,
}

#[cfg(feature = "modules")]
#[tinybus::interface(name = "ai.tinyhumans.tinymcp.Mcp")]
impl MockTinyMcp {
    #[tinybus(confidential)]
    async fn configured_tool_call(
        &self,
        request: tinymcp_bus::ConfiguredToolCallRequest,
    ) -> tinybus::Result<tinymcp_bus::ClassifiedStaticToolCall> {
        let tool = request.tool.clone();
        let server = request.server.name.clone();
        let client = crate::mcp::http_client::McpHttpClient::new(
            request.server.endpoint,
            request.server.timeout_secs,
        )
        .map_err(|error| tinybus::Error::failed(error.to_string()))?;
        let result = client
            .call_tool(&tool, request.arguments)
            .await
            .map_err(|error| tinybus::Error::failed(error.to_string()))?;
        self.provider_state
            .calls
            .lock()
            .expect("provider request log")
            .push(serde_json::json!({"tool": tool, "server": server}));
        Ok(tinymcp_bus::ClassifiedStaticToolCall {
            outcome: tinymcp_bus::McpCallOutcome::answered(&server, &tool),
            rendered: Some(result.rendered),
            envelope: None,
            error_message: None,
        })
    }
}

#[cfg(feature = "modules")]
async fn mock_module_client(
    state: MockProviderState,
) -> (ModuleClient, tinybus::Connection) {
    use tinybus::{broker::Broker, transport::memory::MemoryBus, Connection, ObjectPath};

    let bus = MemoryBus::new();
    Broker::new().spawn(bus.clone());
    let service = Connection::connect(bus.connect().await.expect("service transport"))
        .await
        .expect("service connection");
    let caller = Connection::connect(bus.connect().await.expect("caller transport"))
        .await
        .expect("caller connection");
    service
        .serve_at(
            ObjectPath::new(tinymcp_bus::names::OBJECT_PATH).expect("TinyMCP object path"),
            MockTinyMcp {
                provider_state: state,
            },
        )
        .await
        .expect("serve TinyMCP fixture");
    service
        .request_name(tinymcp_bus::names::INTERFACE)
        .await
        .expect("TinyMCP bus name");
    let proxy = caller
        .proxy(
            tinymcp_bus::names::INTERFACE,
            tinymcp_bus::names::OBJECT_PATH,
            tinymcp_bus::names::INTERFACE,
        )
        .expect("TinyMCP proxy");
    (ModuleClient::fixture(proxy), service)
}

#[cfg(feature = "modules")]
async fn mock_gitbooks_provider() -> (String, std::sync::Arc<std::sync::Mutex<Vec<Value>>>) {
    use axum::{extract::State, response::IntoResponse, routing::post, Json, Router};

    let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let app = Router::new()
        .route(
            "/mcp",
            post(
                |State(calls): State<std::sync::Arc<std::sync::Mutex<Vec<Value>>>>,
                 Json(body): Json<Value>| async move {
                    let method = body["method"].as_str().unwrap_or_default();
                    let id = body["id"].clone();
                    let result = match method {
                        "initialize" => serde_json::json!({
                            "protocolVersion": tinymcp_bus::LATEST_PROTOCOL_VERSION,
                            "capabilities": {"tools": {}},
                            "serverInfo": {"name": "gitbooks-fixture", "version": "1"}
                        }),
                        "notifications/initialized" => {
                            return axum::http::StatusCode::NO_CONTENT.into_response();
                        }
                        "tools/list" => serde_json::json!({"tools": [
                            {"name": "searchDocumentation", "inputSchema": {"type": "object"}},
                            {"name": "getPage", "inputSchema": {"type": "object"}}
                        ]}),
                        "tools/call" => {
                            calls.lock().expect("provider request log").push(body["params"].clone());
                            let name = body["params"]["name"].as_str().unwrap_or_default();
                            let text = match name {
                                "searchDocumentation" => "search fixture result",
                                "getPage" => "page fixture markdown",
                                _ => "unexpected tool",
                            };
                            serde_json::json!({"content": [{"type": "text", "text": text}]})
                        }
                        _ => serde_json::json!({}),
                    };
                    axum::Json(serde_json::json!({"jsonrpc": "2.0", "id": id, "result": result}))
                        .into_response()
                },
            ),
        )
        .with_state(std::sync::Arc::clone(&calls));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback provider");
    let address = listener.local_addr().expect("loopback address");
    tokio::spawn(async move { axum::serve(listener, app).await.expect("serve provider") });
    (format!("http://{address}/mcp"), calls)
}

#[cfg(feature = "modules")]
#[tokio::test]
async fn gitbooks_tools_use_confidential_one_shot_module_calls() {
    let (endpoint, provider_calls) = mock_gitbooks_provider().await;
    let (client, _service) = mock_module_client(MockProviderState::default()).await;
    let search = GitbooksSearchTool::with_client(endpoint.clone(), 5, client.clone());
    let page = GitbooksGetPageTool::with_client(endpoint, 5, client);

    let search_result = search
        .execute(serde_json::json!({"query": "module migration"}))
        .await
        .expect("search executes");
    let page_result = page
        .execute(serde_json::json!({"url": "https://docs.example.test/guide"}))
        .await
        .expect("page fetch executes");

    assert!(!search_result.is_error, "{}", search_result.output());
    assert!(search_result.output().contains("search fixture result"));
    assert!(!page_result.is_error, "{}", page_result.output());
    assert!(page_result.output().contains("page fixture markdown"));
    assert_eq!(
        provider_calls.lock().expect("provider request log").as_slice(),
        [
            serde_json::json!({"name": "searchDocumentation", "arguments": {"query": "module migration"}}),
            serde_json::json!({"name": "getPage", "arguments": {"url": "https://docs.example.test/guide"}}),
        ]
    );
}

#[test]
fn redact_endpoint_keeps_only_origin() {
    assert_eq!(
        redact_endpoint("https://tinyhumans.gitbook.io/openhuman/~gitbook/mcp"),
        "https://tinyhumans.gitbook.io"
    );
    assert_eq!(
        redact_endpoint("http://example.com:8080/path?token=secret"),
        "http://example.com:8080"
    );
}

#[tokio::test]
async fn search_rejects_empty_query() {
    let t = GitbooksSearchTool::new("https://example.com/mcp".into(), 5).expect("a client builds");
    let result = t.execute(json!({"query": "   "})).await.unwrap();
    assert!(result.is_error);
    assert!(result.output().contains("empty"));
}

#[tokio::test]
async fn get_page_rejects_empty_url() {
    let t = GitbooksGetPageTool::new("https://example.com/mcp".into(), 5).expect("a client builds");
    let result = t.execute(json!({"url": ""})).await.unwrap();
    assert!(result.is_error);
}

#[tokio::test]
async fn live_search_smoke() {
    if std::env::var("OPENHUMAN_GITBOOKS_LIVE_TEST")
        .ok()
        .as_deref()
        != Some("1")
    {
        return;
    }
    let t = GitbooksSearchTool::new(
        "https://tinyhumans.gitbook.io/openhuman/~gitbook/mcp".into(),
        30,
    )
    .expect("a client builds");
    let result = t
        .execute(json!({"query": "what is openhuman"}))
        .await
        .unwrap();
    assert!(
        !result.is_error,
        "live search returned error: {}",
        result.output()
    );
    assert!(!result.output().is_empty());
}
