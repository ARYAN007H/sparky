//! A real CLI host boot must tolerate pre-removal voice settings without
//! starting the retired always-on listeners.

use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const TOKEN: &str = "legacy-voice-config-boot-e2e-token";

struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn rpc(base: &str, method: &str, params: Value) -> Value {
    reqwest::blocking::Client::new()
        .post(format!("{base}/rpc"))
        .bearer_auth(TOKEN)
        .json(&json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}))
        .send()
        .expect("POST /rpc")
        .json()
        .expect("JSON-RPC response")
}

fn serve_local_backend(listener: TcpListener) {
    std::thread::spawn(move || {
        for incoming in listener.incoming() {
            let Ok(mut stream) = incoming else { break };
            answer_not_found(&mut stream);
        }
    });
}

fn answer_not_found(stream: &mut TcpStream) {
    let mut request = [0; 4096];
    let _ = stream.read(&mut request);
    let body = "{}";
    let response = format!(
        "HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
}

fn wait_healthy(server: &mut Server, base: &str) {
    let client = reqwest::blocking::Client::new();
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        if client
            .get(format!("{base}/health"))
            .send()
            .is_ok_and(|response| response.status().is_success())
        {
            return;
        }
        if let Ok(Some(status)) = server.0.try_wait() {
            panic!("openhuman-core exited before health: {status}");
        }
        assert!(Instant::now() < deadline, "openhuman-core did not become healthy");
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn write_legacy_config(path: &Path) {
    std::fs::write(
        path,
        r#"
[dictation]
enabled = true
hotkey = "Ctrl+Shift+Space"

[voice_server]
always_on_enabled = true
stt_engine = "elevenlabs"
"#,
    )
    .unwrap();
}

#[test]
fn legacy_voice_settings_boot_without_starting_retired_listeners() {
    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    write_legacy_config(&workspace.join("config.toml"));

    let backend = TcpListener::bind("127.0.0.1:0").unwrap();
    let backend_url = format!("http://{}", backend.local_addr().unwrap());
    serve_local_backend(backend);

    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let log_path = tmp.path().join("core.log");
    let log = std::fs::File::create(&log_path).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_openhuman-core"));
    command
        .args(["serve", "--port", &port.to_string()])
        .env("HOME", tmp.path())
        .env("USERPROFILE", tmp.path())
        .env("OPENHUMAN_WORKSPACE", &workspace)
        .env("OPENHUMAN_CORE_TOKEN", TOKEN)
        .env("OPENHUMAN_KEYRING_BACKEND", "file")
        .env("BACKEND_URL", backend_url)
        .env_remove("OPENHUMAN_BACKEND_API_KEY")
        .env_remove("OPENHUMAN_BACKEND_SESSION_TOKEN")
        .env_remove("OPENHUMAN_STORAGE_URL")
        .stdout(Stdio::null())
        .stderr(Stdio::from(log));
    let mut server = Server(command.spawn().expect("spawn openhuman-core"));
    let base = format!("http://127.0.0.1:{port}");
    wait_healthy(&mut server, &base);

    let credential = rpc(
        &base,
        "openhuman.auth_set_credential",
        json!({"token":"th_test_local_only","kind":"api-key"}),
    );
    assert!(credential.get("error").is_none(), "{credential}");
    let status = rpc(&base, "openhuman.voice_status", json!({}));
    assert!(status.get("error").is_none(), "{status}");
    assert_eq!(
        status.pointer("/result/stt_engine").and_then(Value::as_str),
        Some("elevenlabs"),
        "legacy non-default engine must survive host configuration load: {status}"
    );

    std::thread::sleep(Duration::from_millis(500));
    let logs = std::fs::read_to_string(log_path).unwrap();
    assert!(
        logs.contains("login-gated services started concurrently"),
        "the fake credential must exercise credential-gated startup: {logs}"
    );
    assert!(
        !logs.contains("[voice::") && !logs.contains("dictation listener") && !logs.contains("hotkey listener"),
        "retired voice listener started during boot: {logs}"
    );
}
