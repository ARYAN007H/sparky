//! JSON-RPC E2E coverage for the agent-orchestration controllers that no e2e
//! target reached: detached sub-agent control, the command
//! center's `agent_work_control`, the
//! detached sub-agent controls (`subagent_cancel` / `subagent_steer`).
//!
//! Every case boots the real Axum JSON-RPC router over HTTP against an
//! isolated `HOME` and asserts on the **content** of the response. Nothing
//! here needs a model: the durable ledger writes, the control-verb validation
//! and the sub-agent registry lookups are all reachable offline, and `api_url`
//! points at a closed port so a spawned engine loop fails fast instead of
//! reaching the network.
//!
//! Aggregated into `tests/raw_coverage_all.rs` by `build.rs`. Run with:
//! `cargo test --test raw_coverage_all --features "$(bash scripts/ci/product-features.sh)" agent_orchestration_e2e`

use crate::env_guard::EnvVarGuard;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

use axum::http::header::AUTHORIZATION;
use reqwest::StatusCode;
use serde_json::{json, Value};
use tempfile::{tempdir, TempDir};

use openhuman_core::core::auth::{get_rpc_token, init_rpc_token, CORE_TOKEN_ENV_VAR};
use openhuman_rpc::server::build_core_http_router;

/// Seeded only if this suite is the first in the aggregated binary to
/// initialise the token; the bearer actually sent is always read back from
/// `get_rpc_token()`, because `RPC_TOKEN` is a process-global `OnceLock` and
/// whichever aggregated suite calls `init_rpc_token` first wins it for all.
const TEST_RPC_TOKEN: &str = "agent-orchestration-e2e-token";

/// The one builtin workflow definition (`ops::PARALLEL_RESEARCH_ID`).
const BUILTIN_WORKFLOW_ID: &str = "parallel_research_cross_check";

static AUTH_INIT: OnceLock<()> = OnceLock::new();

/// The crate-wide env lock, not a private one. Every aggregated suite in
/// `raw_coverage_all` shares one process, so libtest runs them concurrently
/// and a lock local to this file would isolate nothing.
static ENV_LOCK: &OnceLock<tokio::sync::Mutex<()>> = &crate::SHARED_ENV_LOCK;

fn env_lock() -> tokio::sync::MutexGuard<'static, ()> {
    ENV_LOCK
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .blocking_lock()
}

async fn env_lock_async() -> tokio::sync::MutexGuard<'static, ()> {
    ENV_LOCK
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock().await
}

/// Initialise the process RPC token (idempotent) and return the bearer the
/// router will actually accept.
fn ensure_rpc_auth() -> &'static str {
    crate::tinyhumans_boot::boot();
    AUTH_INIT.get_or_init(|| {
        if get_rpc_token().is_none() {
            std::env::set_var(CORE_TOKEN_ENV_VAR, TEST_RPC_TOKEN);
        }
        let token_dir = std::env::temp_dir().join("openhuman-agent-orchestration-e2e-auth");
        init_rpc_token(&token_dir).expect("init rpc auth token");
    });
    get_rpc_token().expect("rpc token initialized")
}

async fn serve_rpc() -> (
    SocketAddr,
    &'static str,
    tokio::task::JoinHandle<Result<(), std::io::Error>>,
) {
    let token = ensure_rpc_auth();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind rpc listener");
    let addr = listener.local_addr().expect("rpc listener addr");
    let join =
        tokio::spawn(async move { axum::serve(listener, build_core_http_router(false)).await });
    (addr, token, join)
}

/// `api_url` points at a closed port on purpose: a workflow run's spawned
/// engine loop must fail fast rather than reach the network.
fn write_min_config(openhuman_dir: &Path) {
    let cfg = r#"api_url = "http://127.0.0.1:9"
default_model = "orchestration-e2e-model"
default_temperature = 0.2
chat_onboarding_completed = true

[secrets]
encrypt = false

[local_ai]
enabled = false

[node]
enabled = false

[runtime_python]
enabled = false

[memory_tree]
embedding_strict = false
spacy_enabled = false
"#;
    let write = |dir: &Path| {
        std::fs::create_dir_all(dir).expect("create config dir");
        std::fs::write(dir.join("config.toml"), cfg).expect("write config.toml");
    };
    write(openhuman_dir);
    // Runtime config resolution is user-scoped before login, so the pre-login
    // `users/local` layer needs the same file or the RPC handlers load defaults.
    write(&openhuman_dir.join("users").join("local"));
    let _: openhuman_core::config::Config =
        toml::from_str(cfg).expect("test config must match the Config schema");
}

struct Harness {
    _tmp: TempDir,
    _guards: Vec<EnvVarGuard>,
    rpc_base: String,
    token: &'static str,
    join: tokio::task::JoinHandle<Result<(), std::io::Error>>,
}

impl Harness {
    async fn rpc(&self, id: i64, method: &str, params: Value) -> Value {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("build rpc client");
        let url = format!("{}/rpc", self.rpc_base);
        let response = client
            .post(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .json(&json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": params,
            }))
            .send()
            .await
            .unwrap_or_else(|err| panic!("POST {url} {method}: {err}"));
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "HTTP transport should accept {method}"
        );
        response
            .json::<Value>()
            .await
            .unwrap_or_else(|err| panic!("json for {method}: {err}"))
    }

    async fn ok(&self, id: i64, method: &str, params: Value) -> Value {
        let response = self.rpc(id, method, params).await;
        if let Some(error) = response.get("error") {
            panic!("{method}: unexpected JSON-RPC error: {error}");
        }
        let result = response
            .get("result")
            .unwrap_or_else(|| panic!("{method}: missing result: {response}"));
        match result.get("result") {
            Some(inner) if result.get("logs").is_some() => inner.clone(),
            _ => result.clone(),
        }
    }

    async fn err(&self, id: i64, method: &str, params: Value) -> String {
        let response = self.rpc(id, method, params).await;
        let error = response
            .get("error")
            .unwrap_or_else(|| panic!("{method}: expected a JSON-RPC error, got: {response}"));
        error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("{method}: error carries no message: {error}"))
            .to_string()
    }
}

async fn setup() -> Harness {
    crate::tinyhumans_boot::boot();
    let tmp = tempdir().expect("tempdir");
    let home = tmp.path().to_path_buf();
    write_min_config(&home.join(".openhuman"));

    // Left at its default, `action_dir` is `~/OpenHuman/projects` — the
    // developer's real directory. Pin it inside the tempdir.
    let action_dir = home.join("actions");
    std::fs::create_dir_all(&action_dir).expect("create action dir");

    let guards = vec![
        EnvVarGuard::set_to_path("HOME", &home),
        EnvVarGuard::set_to_path("OPENHUMAN_ACTION_DIR", &action_dir),
        EnvVarGuard::unset("OPENHUMAN_WORKSPACE"),
        EnvVarGuard::unset("BACKEND_URL"),
        EnvVarGuard::unset("VITE_BACKEND_URL"),
        EnvVarGuard::unset("OPENHUMAN_API_URL"),
        EnvVarGuard::set("OPENHUMAN_KEYRING_BACKEND", "file"),
        EnvVarGuard::set("OPENHUMAN_MEMORY_EMBED_STRICT", "false"),
    ];

    let (addr, token, join) = serve_rpc().await;
    Harness {
        _tmp: tmp,
        _guards: guards,
        rpc_base: format!("http://{addr}"),
        token,
        join,
    }
}

fn str_at<'a>(value: &'a Value, pointer: &str) -> &'a str {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("expected a string at {pointer} in {value}"))
}

