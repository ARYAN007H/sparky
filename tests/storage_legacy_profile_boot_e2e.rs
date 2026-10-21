//! A signed-in profile's existing v1 `storage.db` survives the desktop host
//! boot migration and remains readable through the domain owners.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const TOKEN: &str = "legacy-profile-boot-e2e-token";
const FIXTURE: &[u8] = include_bytes!(
    "../crates/openhuman-core/src/storage/import/fixtures/legacy-v1-profile/storage.db"
);

struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn start(home: &Path) -> (Server, String) {
    let port = free_port();
    let server = Command::new(env!("CARGO_BIN_EXE_openhuman-core"))
        .args(["serve", "--port", &port.to_string()])
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("OPENHUMAN_CORE_TOKEN", TOKEN)
        .env("OPENHUMAN_KEYRING_MASTER_KEY", "22".repeat(32))
        .env("OPENHUMAN_MODULE_SOURCE", "linked")
        .env_remove("OPENHUMAN_KEYRING_MASTER_KEY_FILE")
        .env_remove("OPENHUMAN_WORKSPACE")
        .env_remove("OPENHUMAN_STORAGE_URL")
        .env_remove("OPENHUMAN_MODE")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn the desktop core host");
    let mut server = Server(server);
    let base = format!("http://127.0.0.1:{port}");
    let client = reqwest::blocking::Client::new();
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        if client
            .get(format!("{base}/health"))
            .send()
            .is_ok_and(|response| response.status().is_success())
        {
            return (server, base);
        }
        if let Ok(Some(status)) = server.0.try_wait() {
            panic!("desktop core host exited before ready: {status}");
        }
        assert!(Instant::now() < deadline, "desktop core host never became healthy");
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn rpc(base: &str, method: &str) -> Value {
    let response: Value = reqwest::blocking::Client::new()
        .post(format!("{base}/rpc"))
        .bearer_auth(TOKEN)
        .json(&json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": {} }))
        .send()
        .unwrap_or_else(|error| panic!("POST {method}: {error}"))
        .json()
        .unwrap_or_else(|error| panic!("decode {method}: {error}"));
    assert!(response.get("error").is_none(), "{method}: {response}");
    response["result"].clone()
}

fn seed_existing_profile(home: &Path) -> (PathBuf, PathBuf) {
    let openhuman = home.join(".openhuman");
    let profile = openhuman.join("users").join("u1");
    let workspace = profile.join("workspace");
    std::fs::create_dir_all(workspace.join("storage")).unwrap();
    std::fs::write(openhuman.join("active_user.toml"), "user_id = 'u1'\n").unwrap();
    std::fs::write(
        profile.join("config.toml"),
        "[storage]\nauto_migrate_layout = true\n",
    )
    .unwrap();
    let source = workspace.join("storage/storage.db");
    std::fs::write(&source, FIXTURE).unwrap();
    (profile, source)
}

#[test]
fn desktop_boot_reads_a_preserved_existing_v1_profile_database() {
    let home = tempfile::tempdir().unwrap();
    let (profile, source) = seed_existing_profile(home.path());
    let source_bytes = std::fs::read(&source).unwrap();

    let (_server, base) = start(home.path());
    let threads = rpc(&base, "openhuman.threads_list");
    let approvals = rpc(&base, "openhuman.approval_list_pending");
    let flows = rpc(&base, "openhuman.flows_list");

    assert!(
        threads.to_string().contains("fixture-thread"),
        "thread owner did not read its historical profile record: {threads}"
    );
    assert!(
        approvals.to_string().contains("fixture-approval"),
        "approval owner did not read its historical profile record: {approvals}"
    );
    assert!(
        flows.to_string().contains("fixture-flow"),
        "flow owner did not read its historical profile record: {flows}"
    );

    assert!(profile.join("storage.db").is_file(), "boot migrated the active profile");
    assert_eq!(
        std::fs::read(&source).unwrap(),
        source_bytes,
        "boot must leave the historical v1 storage.db byte-for-byte unchanged"
    );
}
