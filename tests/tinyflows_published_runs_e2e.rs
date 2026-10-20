//! Runs the public TinyFlows module against the host's real Runs and CatalogStorage adapters.

#[path = "support/env_guard.rs"]
mod env_guard;
#[path = "support/published_tinyflows.rs"]
mod published_tinyflows;
#[path = "support/tinyhumans_boot.rs"]
mod tinyhumans_boot;

use std::path::Path;
use std::time::Duration;

use axum::http::header::AUTHORIZATION;
use env_guard::EnvVarGuard;
use serde_json::{json, Value};

#[path = "support/storage_drivers.rs"]
#[macro_use]
mod storage_drivers;

use storage_drivers::{Case, Driver};

fn write_config(home: &Path, module: &Path) {
    let quoted_module = serde_json::to_string(
        module
            .to_str()
            .expect("staged published module path is UTF-8"),
    )
    .expect("quoted module path");
    let config = format!(
        "api_url = \"http://127.0.0.1:1\"\n\n[components]\ntinyflows = true\n\n[modules]\nenabled = true\n\n[[modules.overrides]]\nid = \"tinyflows\"\npath = {quoted_module}\n\n[autonomy]\nenabled = true\nlevel = \"read_only\"\n"
    );
    for dir in [home.join(".openhuman"), home.join(".openhuman/users/local")] {
        std::fs::create_dir_all(&dir).expect("create isolated config directory");
        std::fs::write(dir.join("config.toml"), &config).expect("write isolated config");
    }
}

async fn rpc(addr: std::net::SocketAddr, token: &str, id: u64, method: &str, params: Value) -> Value {
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("build RPC client")
        .post(format!("http://{addr}/rpc"))
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .json(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))
        .send()
        .await
        .unwrap_or_else(|error| panic!("send {method}: {error}"));
    assert!(response.status().is_success(), "{method} transport status");
    response.json().await.expect("decode RPC response")
}

async fn ok(addr: std::net::SocketAddr, token: &str, id: u64, method: &str, params: Value) -> Value {
    let response = rpc(addr, token, id, method, params).await;
    assert!(response.get("error").is_none(), "{method}: {response}");
    let result = &response["result"];
    match (result.get("logs"), result.get("result")) {
        (Some(_), Some(value)) => value.clone(),
        _ => result.clone(),
    }
}

fn approval_gated_http_graph() -> Value {
    json!({
        "name": "Published HTTP capability",
        "nodes": [
            { "id": "trigger", "kind": "trigger", "name": "Manual" },
            { "id": "request", "kind": "http_request", "name": "Blocked request",
              "config": { "method": "POST", "url": "http://127.0.0.1:1/must-not-connect" } },
            { "id": "after", "kind": "output_parser", "name": "After request" }
        ],
        "edges": [
            { "from_node": "trigger", "to_node": "request" },
            { "from_node": "request", "to_node": "after" }
        ]
    })
}

fn approval_gated_parser_graph() -> Value {
    json!({
        "name": "Published approval lifecycle",
        "nodes": [
            { "id": "trigger", "kind": "trigger", "name": "Manual" },
            { "id": "gate", "kind": "output_parser", "name": "Approval", "config": { "requires_approval": true } },
            { "id": "after", "kind": "output_parser", "name": "After approval" }
        ],
        "edges": [
            { "from_node": "trigger", "to_node": "gate" },
            { "from_node": "gate", "to_node": "after" }
        ]
    })
}

#[tokio::test]
async fn published_runs_use_scoped_host_capabilities_and_persist_lifecycle() {
    let _env_lock = env_guard::env_lock_async().await;
    let case = Case::open(Driver::Memory).expect("memory storage driver is always available");
    case.install();

    let temp = tempfile::tempdir().expect("isolated home/workspace");
    let _home = EnvVarGuard::set_to_path("HOME", temp.path());
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace directory");
    let _workspace = EnvVarGuard::set_to_path("OPENHUMAN_WORKSPACE", &workspace);
    let _storage = EnvVarGuard::unset("OPENHUMAN_STORAGE_URL");
    let _backend = EnvVarGuard::unset("BACKEND_URL");
    let _vite_backend = EnvVarGuard::unset("VITE_BACKEND_URL");
    let _api_key = EnvVarGuard::unset("OPENHUMAN_BACKEND_API_KEY");
    let _session = EnvVarGuard::unset("OPENHUMAN_BACKEND_SESSION_TOKEN");
    let _module_source = EnvVarGuard::unset("OPENHUMAN_MODULE_SOURCE");
    let _module_source_dir = EnvVarGuard::unset("OPENHUMAN_MODULE_SOURCE_DIR");
    let _tinyflows_source = EnvVarGuard::unset("OPENHUMAN_MODULE_SOURCE_TINYFLOWS");
    let (module, digest) = published_tinyflows::staged_v0_8_27_module();
    write_config(temp.path(), &module);
    tinyhumans_boot::boot();

    const TEST_TOKEN: &str = "tinyflows-published-runs-e2e-token";
    openhuman_core::core::auth::init_rpc_token_with_value(TEST_TOKEN)
        .expect("initialize isolated RPC token");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback RPC listener");
    let addr = listener.local_addr().expect("RPC listener address");
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            openhuman_rpc::server::build_core_http_router(false),
        )
        .await
    });

    let created = ok(
        addr,
        TEST_TOKEN,
        1,
        "openhuman.flows_create",
        json!({ "name": "Published capability flow", "graph": approval_gated_http_graph() }),
    )
    .await;
    let flow_id = created["id"].as_str().expect("saved flow id").to_owned();
    let approval_flow = ok(
        addr,
        TEST_TOKEN,
        8,
        "openhuman.flows_create",
        json!({ "name": "Published approval flow", "graph": approval_gated_parser_graph() }),
    )
    .await;
    let approval_flow_id = approval_flow["id"].as_str().expect("approval flow id").to_owned();
    let approval_parked = ok(
        addr,
        TEST_TOKEN,
        9,
        "openhuman.flows_run",
        json!({ "id": approval_flow_id, "input": { "item": "approved" } }),
    )
    .await;
    let approval_run_id = approval_parked["thread_id"].as_str().expect("approval run id");
    assert!(approval_parked["pending_approvals"].as_array().unwrap().iter().any(|v| v == "gate"));
    let approved = ok(
        addr,
        TEST_TOKEN,
        10,
        "openhuman.flows_resume",
        json!({ "id": approval_flow_id, "thread_id": approval_run_id, "approvals": ["gate"] }),
    )
    .await;
    assert!(approved["pending_approvals"].as_array().unwrap().is_empty());
    let approved_row = ok(
        addr,
        TEST_TOKEN,
        11,
        "openhuman.flows_get_run",
        json!({ "run_id": approval_run_id }),
    )
    .await;
    assert_eq!(approved_row["status"], "completed");

    let parked = ok(
        addr,
        TEST_TOKEN,
        2,
        "openhuman.flows_run",
        json!({ "id": flow_id, "input": { "item": "scoped" } }),
    )
    .await;
    let run_id = parked["thread_id"].as_str().expect("parked run id").to_owned();
    assert!(parked["pending_approvals"].as_array().unwrap().iter().any(|v| v == "request"));

    // Resume reaches the real host CapsHost callback. The captured read-only
    // policy denies Network before the deliberately closed loopback endpoint is
    // contacted; TinyFlows then persists the failed terminal run through the
    // owner-scoped CatalogStorage callback.
    let resumed = ok(
        addr,
        TEST_TOKEN,
        3,
        "openhuman.flows_resume",
        json!({ "id": flow_id, "thread_id": run_id, "approvals": ["request"] }),
    )
    .await;
    assert!(resumed["pending_approvals"].as_array().unwrap().is_empty());
    let terminal = ok(
        addr,
        TEST_TOKEN,
        4,
        "openhuman.flows_get_run",
        json!({ "run_id": run_id }),
    )
    .await;
    assert_eq!(terminal["status"], "failed", "host policy must stop the request: {terminal}");
    let failure = terminal["error"].as_str().unwrap_or_default().to_lowercase();
    assert!(failure.contains("read-only") || failure.contains("network"), "{terminal}");
    assert!(terminal["steps"].as_array().is_some(), "module persisted run steps: {terminal}");
    assert_eq!(
        published_tinyflows::verify_v0_8_27_module(&module),
        digest,
        "the run used the checksum-verified published module"
    );
    assert!(
        !workspace.join("flows/flows.db").exists() && !workspace.join("flows/catalog.db").exists(),
        "module run/catalog state must use the installed CatalogStorage backend"
    );

    let parked_again = ok(
        addr,
        TEST_TOKEN,
        5,
        "openhuman.flows_run",
        json!({ "id": approval_flow_id, "input": { "item": "cancel" } }),
    )
    .await;
    let cancelled_id = parked_again["thread_id"]
        .as_str()
        .expect("second parked run id")
        .to_owned();
    let cancelled = ok(
        addr,
        TEST_TOKEN,
        6,
        "openhuman.flows_cancel_run",
        json!({ "run_id": cancelled_id }),
    )
    .await;
    assert_eq!(cancelled["cancelled"], true);
    let cancelled_row = ok(
        addr,
        TEST_TOKEN,
        7,
        "openhuman.flows_get_run",
        json!({ "run_id": cancelled_id }),
    )
    .await;
    assert_eq!(cancelled_row["status"], "cancelled");

    server.abort();
}
