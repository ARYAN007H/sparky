//! Durable native-channel history across actual CLI process restarts.
//!
//! The first process receives a Telegram update and calls a loopback model.
//! After that process exits, a second `openhuman-core serve` process receives a
//! new update for the same Telegram conversation. The second model request
//! must contain the first user's message from the session transcript.
//!
//! Both Telegram and inference are loopback mocks. This target needs the
//! linked TinyChannels module implementation so it exercises the native
//! Telegram listener rather than a host-side REST adapter.

use std::collections::VecDeque;
use std::fs::File;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use tokio::task::JoinHandle;
use tokio::time::sleep;

const BOT_TOKEN: &str = "session-restart-token";
const MOCK_API_KEY: &str = "loopback-session-restart-key";
const RPC_TOKEN: &str = "native-session-restart-rpc-token";
const FIRST_TEXT: &str = "remember this after the core restarts";
const SECOND_TEXT: &str = "what did I ask before the restart?";

#[derive(Default)]
struct MockState {
    updates: Mutex<VecDeque<Value>>,
    update_offsets: Mutex<Vec<i64>>,
    completions: Mutex<Vec<Value>>,
    sent_messages: Mutex<Vec<Value>>,
}

struct MockServers {
    telegram: String,
    inference: String,
    _telegram_task: JoinHandle<()>,
    _inference_task: JoinHandle<()>,
    state: Arc<MockState>,
}

async fn start_mock_servers() -> MockServers {
    let state = Arc::new(MockState::default());
    let telegram_app = Router::new()
        .route("/botsession-restart-token/getUpdates", post(get_updates))
        .route("/botsession-restart-token/sendMessage", post(send_message))
        .route(
            "/botsession-restart-token/editMessageText",
            post(send_message),
        )
        .route("/botsession-restart-token/sendChatAction", post(ok))
        .route("/botsession-restart-token/deleteWebhook", post(ok))
        .route("/botsession-restart-token/getMe", get(get_me))
        .with_state(Arc::clone(&state));
    let inference_app = Router::new()
        .route("/v1/chat/completions", post(chat_completion))
        .route("/v1/models", get(models))
        .route("/auth/me", get(auth_me))
        .route("/settings", get(auth_me))
        .with_state(Arc::clone(&state));
    let (telegram, telegram_task) = serve(telegram_app).await;
    let (inference, inference_task) = serve(inference_app).await;
    MockServers {
        telegram,
        inference,
        _telegram_task: telegram_task,
        _inference_task: inference_task,
        state,
    }
}

async fn serve(app: Router) -> (String, JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback mock");
    let address = listener.local_addr().expect("mock address");
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("serve loopback mock");
    });
    (format!("http://{address}"), task)
}

async fn get_updates(State(state): State<Arc<MockState>>, Json(body): Json<Value>) -> Json<Value> {
    sleep(Duration::from_millis(25)).await;
    let offset = body
        .get("offset")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    state
        .update_offsets
        .lock()
        .expect("Telegram offsets")
        .push(offset);
    let mut updates = state.updates.lock().expect("Telegram updates");
    updates.retain(|update| {
        update
            .get("update_id")
            .and_then(Value::as_i64)
            .is_some_and(|id| id >= offset)
    });
    Json(json!({ "ok": true, "result": updates.iter().cloned().collect::<Vec<_>>() }))
}

async fn send_message(State(state): State<Arc<MockState>>, Json(body): Json<Value>) -> Json<Value> {
    state
        .sent_messages
        .lock()
        .expect("Telegram sent messages")
        .push(body);
    let id = state
        .sent_messages
        .lock()
        .expect("Telegram sent messages")
        .len() as i64;
    Json(json!({ "ok": true, "result": { "message_id": id, "chat": { "id": 42 } } }))
}

async fn ok() -> Json<Value> {
    Json(json!({ "ok": true, "result": true }))
}

async fn models() -> Json<Value> {
    Json(json!({ "data": [{ "id": "session-model", "object": "model" }] }))
}

async fn auth_me() -> Json<Value> {
    Json(json!({ "id": "loopback-user", "email": "loopback@example.test" }))
}

async fn get_me() -> Json<Value> {
    Json(json!({ "ok": true, "result": { "id": 9, "is_bot": true, "username": "test_bot" } }))
}

async fn chat_completion(State(state): State<Arc<MockState>>, Json(body): Json<Value>) -> Response {
    let index = {
        let mut completions = state.completions.lock().expect("model requests");
        completions.push(body.clone());
        completions.len()
    };
    let message = json!({
        "role": "assistant",
        "content": format!("mock reply {index}")
    });
    if body.get("stream").and_then(Value::as_bool).unwrap_or(false) {
        let mut delta = json!({ "role": "assistant" });
        delta["content"] = message["content"].clone();
        let stream = format!(
            "data: {}\n\ndata: {}\n\ndata: [DONE]\n\n",
            json!({ "choices": [{ "index": 0, "delta": delta }] }),
            json!({ "choices": [{ "index": 0, "delta": {}, "finish_reason": "stop" }] })
        );
        return (
            [(axum::http::header::CONTENT_TYPE, "text/event-stream")],
            stream,
        )
            .into_response();
    }
    let response = json!({
        "choices": [{
            "index": 0,
            "message": message,
            "finish_reason": "stop"
        }]
    });
    (StatusCode::OK, Json(response)).into_response()
}

fn message_update(update_id: i64, message_id: i64, text: &str) -> Value {
    json!({
        "update_id": update_id,
        "message": {
            "message_id": message_id,
            "date": 1_800_000_000,
            "from": { "id": 42, "is_bot": false, "first_name": "Ada" },
            "chat": { "id": 42, "type": "private", "first_name": "Ada" },
            "text": text
        }
    })
}

fn push_update(state: &MockState, update_id: i64, message_id: i64, text: &str) {
    state
        .updates
        .lock()
        .expect("Telegram updates")
        .push_back(message_update(update_id, message_id, text));
}

fn write_config(home: &Path, workspace: &Path, inference: &str) {
    let openhuman_dir = home.join(".openhuman");
    std::fs::create_dir_all(&openhuman_dir).expect("create OpenHuman config dir");
    std::fs::create_dir_all(workspace).expect("create workspace");
    let config = format!(
        r#"default_model = "mock:session-model"
chat_provider = "mock:session-model"
api_url = "{inference}"
default_temperature = 0.0

[secrets]
encrypt = false

[memory]
provider = "none"
embedding_provider = "none"
embedding_model = "none"
embedding_dimensions = 0

[update]
enabled = false

[channels_config.telegram]
bot_token = "{BOT_TOKEN}"
allowed_users = ["42"]
mention_only = false

[[cloud_providers]]
id = "p_session_restart"
slug = "mock"
label = "Session restart mock"
endpoint = "{inference}/v1"
auth_style = "none"
default_model = "session-model"
"#
    );
    std::fs::write(openhuman_dir.join("config.toml"), &config).expect("write config");
    let parsed: openhuman_core::config::Config =
        toml::from_str(&config).expect("mock config matches the core schema");
    assert_eq!(
        parsed.channels_config.telegram.as_ref().unwrap().bot_token,
        BOT_TOKEN
    );
}

struct CoreProcess {
    child: Child,
    log_path: std::path::PathBuf,
    stopped: bool,
}

impl CoreProcess {
    fn stop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            self.child.kill().expect("stop core process");
        }
        self.child.wait().expect("reap core process");
        self.stopped = true;
    }
}

impl Drop for CoreProcess {
    fn drop(&mut self) {
        if !self.stopped {
            let _ = self.child.kill();
            let _ = self.child.wait();
            if let Ok(log) = std::fs::read_to_string(&self.log_path) {
                let tail = log.lines().rev().take(80).collect::<Vec<_>>();
                eprintln!(
                    "core process was dropped before the test completed; last {} log lines:\n{}",
                    tail.len(),
                    tail.into_iter().rev().collect::<Vec<_>>().join("\n")
                );
            }
        }
    }
}

fn start_core(
    home: &Path,
    workspace: &Path,
    telegram: &str,
    inference: &str,
    log_path: &Path,
) -> (CoreProcess, String) {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .expect("reserve core port")
        .local_addr()
        .expect("core port")
        .port();
    let stdout = File::create(log_path).expect("create core log");
    let stderr = stdout.try_clone().expect("clone core log handle");
    let child = Command::new(env!("CARGO_BIN_EXE_openhuman-core"))
        .args(["serve", "--port", &port.to_string()])
        .current_dir(home)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("OPENHUMAN_WORKSPACE", workspace)
        .env("OPENHUMAN_CORE_TOKEN", RPC_TOKEN)
        .env(
            "RUST_LOG",
            "tinychannels=debug,tinychannels_module=debug,tinystoragedrivers=debug,openhuman::storage_host=info,openhuman_core::channels=debug",
        )
        .env("OPENHUMAN_MODEL", "mock:session-model")
        .env("OPENHUMAN_BACKEND_API_KEY", MOCK_API_KEY)
        .env("BACKEND_URL", inference)
        .env("OPENHUMAN_API_URL", inference)
        .env("OPENHUMAN_AUTO_UPDATE_ENABLED", "false")
        .env("OPENHUMAN_TELEGRAM_BOT_API_BASE", telegram)
        .env("OPENHUMAN_COMPONENT_CHANNELS", "1")
        .env("OPENHUMAN_MODULE_SOURCE", "linked")
        .env(
            "OPENHUMAN_STORAGE_URL",
            format!("sqlite:{}", workspace.join("sessions.db").display()),
        )
        .env("OPENHUMAN_KEYRING_MASTER_KEY", "44".repeat(32))
        .env_remove("OPENHUMAN_KEYRING_MASTER_KEY_FILE")
        .env_remove("OPENHUMAN_MODULE_PATH")
        .env_remove("OPENHUMAN_MODULE_SOURCE_DIR")
        .env_remove("OPENHUMAN_MODE")
        .env_remove("OPENHUMAN_BACKEND_SESSION_TOKEN")
        .envs(
            [
                "SANDBOX",
                "CONNECTORS",
                "HOSTING",
                "MEMES",
                "COMPUTER",
                "SEARCH",
                "DOCUMENTS",
                "WALLET",
                "JUICE",
                "VOICE",
            ]
            .map(|name| (format!("OPENHUMAN_COMPONENT_{name}"), "0")),
        )
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .expect("spawn openhuman-core");
    (
        CoreProcess {
            child,
            log_path: log_path.to_path_buf(),
            stopped: false,
        },
        format!("http://127.0.0.1:{port}"),
    )
}

async fn wait_until(label: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(45);
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for {label}");
        sleep(Duration::from_millis(50)).await;
    }
}

async fn wait_healthy(base: &str, process: &mut CoreProcess, log_path: &Path) {
    let client = reqwest::Client::new();
    let health = format!("{base}/health");
    let deadline = Instant::now() + Duration::from_secs(90);
    loop {
        if client
            .get(&health)
            .send()
            .await
            .is_ok_and(|response| response.status().is_success())
        {
            return;
        }
        if let Ok(Some(exit)) = process.child.try_wait() {
            let log = std::fs::read_to_string(log_path).unwrap_or_default();
            panic!("core exited before health became ready ({exit}):\n{log}");
        }
        assert!(Instant::now() < deadline, "core did not become healthy");
        sleep(Duration::from_millis(100)).await;
    }
}

async fn wait_for_completion(state: &MockState, count: usize) {
    let deadline = Instant::now() + Duration::from_secs(45);
    while state.completions.lock().expect("model requests").len() < count {
        if Instant::now() >= deadline {
            let model_requests = state.completions.lock().expect("model requests").len();
            let offsets = state
                .update_offsets
                .lock()
                .expect("Telegram offsets")
                .clone();
            let sent_messages = state
                .sent_messages
                .lock()
                .expect("Telegram sent messages")
                .clone();
            panic!(
                "timed out waiting for model request {count}; observed {model_requests} model requests, Telegram offsets {offsets:?}, and outbound bodies {sent_messages:#?}"
            );
        }
        sleep(Duration::from_millis(50)).await;
    }
    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        let sent_messages = state
            .sent_messages
            .lock()
            .expect("Telegram sent messages")
            .clone();
        if sent_messages.iter().any(|body| {
            body["text"]
                .as_str()
                .is_some_and(|text| text.contains(&format!("mock reply {count}")))
        }) {
            break;
        }
        if Instant::now() >= deadline {
            panic!("timed out waiting for final Telegram reply {count}; captured outbound bodies: {sent_messages:#?}");
        }
        sleep(Duration::from_millis(50)).await;
    }
}

fn model_saw_user_text(request: &Value, text: &str) -> bool {
    request["messages"].as_array().is_some_and(|messages| {
        messages.iter().any(|message| {
            message["role"] == "user" && content_contains_text(&message["content"], text)
        })
    })
}

fn content_contains_text(content: &Value, expected: &str) -> bool {
    match content {
        Value::String(text) => text.contains(expected),
        Value::Array(parts) => parts
            .iter()
            .any(|part| content_contains_text(part, expected)),
        Value::Object(fields) => {
            fields
                .get("text")
                .is_some_and(|text| content_contains_text(text, expected))
                || fields
                    .get("content")
                    .is_some_and(|nested| content_contains_text(nested, expected))
        }
        _ => false,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_telegram_session_resumes_after_the_core_process_restarts() {
    let tmp = tempfile::tempdir().expect("temporary core home");
    let home = std::fs::canonicalize(tmp.path()).expect("canonicalize temporary home");
    let workspace = home.join("workspace");
    let logs = home.join("logs");
    std::fs::create_dir_all(&logs).expect("create logs");

    let mocks = start_mock_servers().await;
    write_config(&home, &workspace, &mocks.inference);

    // The first listener instance consumes one real native Telegram update.
    let (mut first, first_rpc) = start_core(
        &home,
        &workspace,
        &mocks.telegram,
        &mocks.inference,
        &logs.join("first.log"),
    );
    wait_healthy(&first_rpc, &mut first, &logs.join("first.log")).await;
    let first_pid = first.child.id();
    push_update(&mocks.state, 100, 10, FIRST_TEXT);
    wait_for_completion(&mocks.state, 1).await;
    let first_request = mocks
        .state
        .completions
        .lock()
        .expect("model requests")
        .first()
        .cloned()
        .expect("first Telegram update reached the model");
    assert!(
        model_saw_user_text(&first_request, FIRST_TEXT),
        "the first process must reach the model through native Telegram ingress: {}",
        serde_json::to_string_pretty(&first_request).unwrap()
    );
    assert_eq!(
        mocks
            .state
            .completions
            .lock()
            .expect("model requests")
            .len(),
        1,
        "the first process must finish exactly one model turn before restart"
    );
    // Telegram only acknowledges a delivered update after the client advances
    // its polling offset; wait for that protocol step before stopping process 1.
    wait_until("Telegram acknowledgement of first update", || {
        mocks
            .state
            .update_offsets
            .lock()
            .expect("Telegram offsets")
            .iter()
            .any(|offset| *offset > 100)
    })
    .await;
    first.stop();

    // The second process starts from the same durable workspace and gets a
    // distinct message in the same Telegram conversation.
    push_update(&mocks.state, 101, 11, SECOND_TEXT);
    let (mut second, second_rpc) = start_core(
        &home,
        &workspace,
        &mocks.telegram,
        &mocks.inference,
        &logs.join("second.log"),
    );
    assert_ne!(
        first_pid,
        second.child.id(),
        "restart must use a fresh process"
    );
    wait_healthy(&second_rpc, &mut second, &logs.join("second.log")).await;
    wait_until("second model request", || {
        mocks
            .state
            .completions
            .lock()
            .expect("model requests")
            .len()
            >= 2
    })
    .await;

    let requests = mocks
        .state
        .completions
        .lock()
        .expect("model requests")
        .clone();
    assert_eq!(
        requests.len(),
        2,
        "each Telegram update must reach the model once"
    );
    assert!(
        model_saw_user_text(&requests[1], FIRST_TEXT),
        "the restarted process must resume the first Telegram turn in the exact second model request; got {}",
        serde_json::to_string_pretty(&requests[1]).unwrap()
    );
    assert!(
        model_saw_user_text(&requests[1], SECOND_TEXT),
        "the resumed model request must include the current Telegram message too; got {}",
        serde_json::to_string_pretty(&requests[1]).unwrap()
    );
    wait_for_completion(&mocks.state, 2).await;
    second.stop();
}
