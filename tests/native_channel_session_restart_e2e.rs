//! Durable native-channel history across actual CLI process restarts.
//!
//! The first process receives a Telegram update and calls a loopback model.
//! After that process exits, a second `openhuman-core serve` process receives a
//! new update for the same Telegram conversation. The second model request
//! must contain the first user's message from the session transcript.
//!
//! Both Telegram and inference are loopback mocks. This target loads a locally
//! compiled TinyChannels module so it exercises the native Telegram listener
//! through the production TinyBus artifact path.

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
const OTHER_CHAT_TEXT: &str = "remember a separate chat history";
const AFTER_RESET_TEXT: &str = "this starts a fresh session after new";
const RESET_AFTER_RESTART_TEXT: &str = "continue the reset conversation after restart";
const OTHER_AFTER_RESTART_TEXT: &str = "continue the separate chat after restart";

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
    message_update_for(update_id, message_id, 42, text)
}

fn message_update_for(update_id: i64, message_id: i64, chat_id: i64, text: &str) -> Value {
    json!({
        "update_id": update_id,
        "message": {
            "message_id": message_id,
            "date": 1_800_000_000,
            "from": { "id": chat_id, "is_bot": false, "first_name": "Ada" },
            "chat": { "id": chat_id, "type": "private", "first_name": "Ada" },
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

fn push_update_for(state: &MockState, update_id: i64, message_id: i64, chat_id: i64, text: &str) {
    state
        .updates
        .lock()
        .expect("Telegram updates")
        .push_back(message_update_for(update_id, message_id, chat_id, text));
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
allowed_users = ["42", "43"]
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
    module_source_dir: &Path,
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
            "tinychannels=debug,tinychannels::remote=debug,tinychannels_module=debug,tinychannels_module::service=debug,tinystoragedrivers=debug,openhuman::storage_host=info,openhuman_core::channels=debug",
        )
        .env("OPENHUMAN_MODEL", "mock:session-model")
        .env("OPENHUMAN_BACKEND_API_KEY", MOCK_API_KEY)
        .env("BACKEND_URL", inference)
        .env("OPENHUMAN_API_URL", inference)
        .env("OPENHUMAN_AUTO_UPDATE_ENABLED", "false")
        .env("OPENHUMAN_TELEGRAM_BOT_API_BASE", telegram)
        .env("OPENHUMAN_COMPONENT_CHANNELS", "1")
        .env("OPENHUMAN_MODULE_SOURCE", "source")
        .env("OPENHUMAN_MODULE_SOURCE_DIR", module_source_dir)
        .env(
            "OPENHUMAN_STORAGE_URL",
            format!("sqlite:{}", workspace.join("sessions.db").display()),
        )
        .env("OPENHUMAN_KEYRING_MASTER_KEY", "44".repeat(32))
        .env_remove("OPENHUMAN_KEYRING_MASTER_KEY_FILE")
        .env_remove("OPENHUMAN_MODULE_PATH")
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

async fn wait_for_new_session_reply(state: &MockState) {
    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        let sent = state.sent_messages.lock().expect("Telegram sent messages");
        if sent.iter().any(|body| {
            body["text"]
                .as_str()
                .is_some_and(|text| text.contains("Started new session"))
        }) {
            return;
        }
        if Instant::now() >= deadline {
            panic!("timed out waiting for Telegram /new reply; outbound bodies: {sent:#?}");
        }
        drop(sent);
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
    let module_source_dir = std::env::var_os("OPENHUMAN_TEST_MODULE_SOURCE_DIR")
        .map(std::path::PathBuf::from)
        .expect("build TinyChannels and set OPENHUMAN_TEST_MODULE_SOURCE_DIR");
    assert!(
        module_source_dir
            .join("tinychannels/modules.toml")
            .is_file(),
        "compiled TinyChannels source descriptor is missing from {}",
        module_source_dir.display()
    );

    let mocks = start_mock_servers().await;
    write_config(&home, &workspace, &mocks.inference);

    // The first listener instance consumes one real native Telegram update.
    let (mut first, first_rpc) = start_core(
        &home,
        &workspace,
        &mocks.telegram,
        &mocks.inference,
        &module_source_dir,
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
    let wait_for_update_ack = |update_id| {
        let state = Arc::clone(&mocks.state);
        async move {
            wait_until("Telegram acknowledgement of update", || {
                state
                    .update_offsets
                    .lock()
                    .expect("Telegram offsets")
                    .iter()
                    .any(|offset| *offset > update_id)
            })
            .await;
        }
    };
    wait_for_update_ack(100).await;

    // A second private conversation persists independent durable history.
    push_update_for(&mocks.state, 101, 11, 43, OTHER_CHAT_TEXT);
    wait_for_completion(&mocks.state, 2).await;
    wait_for_update_ack(101).await;
    let requests = mocks
        .state
        .completions
        .lock()
        .expect("model requests")
        .clone();
    assert!(model_saw_user_text(&requests[1], OTHER_CHAT_TEXT));
    assert!(!model_saw_user_text(&requests[1], FIRST_TEXT));

    // The real module command must reset only chat 42's durable SessionRef.
    push_update(&mocks.state, 102, 12, "/new");
    wait_for_new_session_reply(&mocks.state).await;
    wait_for_update_ack(102).await;

    push_update(&mocks.state, 103, 13, AFTER_RESET_TEXT);
    wait_for_completion(&mocks.state, 3).await;
    wait_for_update_ack(103).await;
    let reset_request = mocks
        .state
        .completions
        .lock()
        .expect("model requests")
        .get(2)
        .cloned()
        .expect("post-reset Telegram update reached the model");
    assert!(model_saw_user_text(&reset_request, AFTER_RESET_TEXT));
    assert!(!model_saw_user_text(&reset_request, FIRST_TEXT));
    assert!(!model_saw_user_text(&reset_request, OTHER_CHAT_TEXT));
    first.stop();

    // Both durable session histories survive a process restart: chat 42 keeps
    // its post-reset messages, while chat 43 retains its unrelated history.
    push_update(&mocks.state, 104, 14, RESET_AFTER_RESTART_TEXT);
    push_update_for(&mocks.state, 105, 15, 43, OTHER_AFTER_RESTART_TEXT);
    let (mut second, second_rpc) = start_core(
        &home,
        &workspace,
        &mocks.telegram,
        &mocks.inference,
        &module_source_dir,
        &logs.join("second.log"),
    );
    assert_ne!(
        first_pid,
        second.child.id(),
        "restart must use a fresh process"
    );
    wait_healthy(&second_rpc, &mut second, &logs.join("second.log")).await;
    wait_until("post-restart model requests", || {
        mocks
            .state
            .completions
            .lock()
            .expect("model requests")
            .len()
            >= 5
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
        5,
        "each Telegram update must reach the model once"
    );
    assert!(
        model_saw_user_text(&requests[3], AFTER_RESET_TEXT),
        "chat 42 must resume its post-reset transcript after process restart; got {}",
        serde_json::to_string_pretty(&requests[3]).unwrap()
    );
    assert!(
        model_saw_user_text(&requests[3], RESET_AFTER_RESTART_TEXT),
        "the resumed chat 42 request must include its current message"
    );
    assert!(!model_saw_user_text(&requests[3], FIRST_TEXT));
    assert!(!model_saw_user_text(&requests[3], OTHER_CHAT_TEXT));
    assert!(model_saw_user_text(&requests[4], OTHER_CHAT_TEXT));
    assert!(model_saw_user_text(&requests[4], OTHER_AFTER_RESTART_TEXT));
    assert!(!model_saw_user_text(&requests[4], FIRST_TEXT));
    assert!(!model_saw_user_text(&requests[4], AFTER_RESET_TEXT));
    wait_for_completion(&mocks.state, 5).await;
    second.stop();
}
