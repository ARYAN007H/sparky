//! `[components]` over HTTP JSON-RPC: a component the config switched off
//! answers `COMPONENT_DISABLED:<name>` instead of "unknown method", while a
//! component left on keeps serving.
//!
//! Its own binary because it boots a core, which sets the process default
//! context for everything else in the same process.

use std::sync::Arc;

use openhuman_core::config::Config;
use openhuman_core::core::runtime::{CoreBuilder, DomainSet, ServiceSet, TokenSource};
use openhuman_core::HostKind;
use openhuman_rpc::server::build_core_http_router;
use serde_json::{json, Value};

#[path = "support/tinyhumans_boot.rs"]
mod tinyhumans_boot;

const TOKEN: &str = "components-e2e-token";

async fn rpc(base: &str, method: &str) -> Value {
    reqwest::Client::new()
        .post(format!("{base}/rpc"))
        .bearer_auth(TOKEN)
        .json(&json!({"jsonrpc":"2.0","id":1,"method":method,"params":{}}))
        .send()
        .await
        .expect("post")
        .json::<Value>()
        .await
        .expect("json")
}

#[test]
fn disabled_component_answers_component_disabled_over_json_rpc() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .thread_stack_size(16 * 1024 * 1024)
                .build()
                .unwrap()
                .block_on(async {
                    let dir = tempfile::tempdir().unwrap();
                    let mut config = Config {
                        workspace_dir: dir.path().join("workspace"),
                        config_path: dir.path().join("config.toml"),
                        ..Config::default()
                    };
                    config.components.mcp = false;
                    tinyhumans_boot::boot();
                    let _runtime = CoreBuilder::new(HostKind::Library)
                        .config(config)
                        .token(TokenSource::Fixed(Arc::new(TOKEN.to_string())))
                        .services(ServiceSet::none())
                        .domains(DomainSet::full())
                        .build()
                        .await
                        .unwrap();

                    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                    let base = format!("http://{}", listener.local_addr().unwrap());
                    let app = build_core_http_router(false);
                    let server = tokio::spawn(async move { axum::serve(listener, app).await });

                    let off = rpc(&base, "openhuman.mcp_clients_status").await;
                    let err = off
                        .get("error")
                        .unwrap_or_else(|| panic!("want error: {off}"));
                    assert_eq!(
                        err.get("message").and_then(Value::as_str),
                        Some("COMPONENT_DISABLED:mcp"),
                        "{off}"
                    );
                    assert_eq!(
                        err.pointer("/data/kind").and_then(Value::as_str),
                        Some("component_disabled"),
                        "{off}"
                    );
                    assert_eq!(
                        err.pointer("/data/component").and_then(Value::as_str),
                        Some("mcp")
                    );

                    // A component left on is untouched.
                    let on = rpc(&base, "openhuman.config_get").await;
                    let msg = on
                        .pointer("/error/message")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    assert!(!msg.starts_with("COMPONENT_DISABLED"), "{on}");
                    assert!(!msg.starts_with("unknown method"), "{on}");

                    server.abort();
                });
        })
        .unwrap()
        .join()
        .unwrap();
}
