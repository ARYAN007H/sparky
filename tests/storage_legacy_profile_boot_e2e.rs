//! A signed-in profile's existing v1 `storage.db` survives the desktop host
//! boot migration and remains readable through the domain owners.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tinystoragedrivers::secrets::DerivedKeys;
use zeroize::Zeroizing;

const TOKEN: &str = "legacy-profile-boot-e2e-token";
const FIXTURE: &[u8] = include_bytes!(
    "../crates/openhuman-core/src/storage/import/fixtures/legacy-v1-profile/storage.db"
);
const THREAD_LOG: &[u8] = include_bytes!(
    "../crates/openhuman-core/src/storage/import/fixtures/legacy-v1-profile/legacy-workspace/memory/conversations/threads.jsonl"
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
        .stderr(std::fs::File::create(home.join("core.stderr.log")).unwrap())
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
    rpc_with_params(base, method, json!({}))
}

fn rpc_with_params(base: &str, method: &str, params: Value) -> Value {
    let response: Value = reqwest::blocking::Client::new()
        .post(format!("{base}/rpc"))
        .bearer_auth(TOKEN)
        .json(&json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }))
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
    let conversations = workspace.join("memory/conversations");
    std::fs::create_dir_all(&conversations).unwrap();
    std::fs::write(conversations.join("threads.jsonl"), THREAD_LOG).unwrap();
    (profile, source)
}

#[test]
fn desktop_boot_reads_a_preserved_existing_v1_profile_database() {
    let home = tempfile::tempdir().unwrap();
    let (profile, source) = seed_existing_profile(home.path());
    let source_bytes = std::fs::read(&source).unwrap();

    let (_server, base) = start(home.path());
    let credential = rpc_with_params(
        &base,
        "openhuman.auth_set_credential",
        json!({
            "token": "fixture-session-token",
            "kind": "session",
            "userId": "u1",
            "user": {"id": "u1", "name": "Fixture User"}
        }),
    );
    assert!(credential.to_string().contains("u1"), "profile credential was not installed: {credential}");
    let threads = rpc(&base, "openhuman.threads_list");
    let flows = rpc(&base, "openhuman.flows_list");

    assert!(
        threads.to_string().contains("fixture-thread"),
        "thread owner did not read its historical profile record: {threads}"
    );
    assert!(
        flows.to_string().contains("fixture-flow"),
        "flow owner did not read its historical profile record: {flows}"
    );

    assert!(
        profile.join("storage.db").is_file(),
        "boot migrated the active profile; profile entries: {:?}; workspace entries: {:?}; logs: {}",
        std::fs::read_dir(&profile)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>(),
        std::fs::read_dir(profile.join("workspace"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>(),
        std::fs::read_to_string(home.path().join("core.stderr.log")).unwrap_or_default()
    );
    assert_eq!(
        std::fs::read(&source).unwrap(),
        source_bytes,
        "boot must leave the historical v1 storage.db byte-for-byte unchanged"
    );

    // The profile's V2 workspace resolves its default backend to the exact
    // storage.db that desktop boot just opened.
    let workspace = profile.join("files");
    let mut config = openhuman_core::config::Config::default();
    config.config_path = profile.join("config.toml");
    config.workspace_dir = workspace.clone();
    let owner_workspace = workspace.clone();
    std::thread::spawn(move || {
        let owner_runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        owner_runtime.block_on(async move {
            let runtime = openhuman_core::core::runtime::CoreBuilder::new(
                openhuman_core::core::types::HostKind::TauriShell,
            )
            .boot_mode(openhuman_core::core::runtime::CoreBootMode::Inspection)
            .token(openhuman_core::core::runtime::TokenSource::Fixed(
                std::sync::Arc::new("legacy-profile-inspection-token".to_owned()),
            ))
            .services(openhuman_core::core::runtime::ServiceSet::none())
            .config(config.clone())
            .build()
            .await
            .unwrap();
            openhuman_core::core::runtime::CoreContext::scope(
                std::sync::Arc::clone(runtime.context()),
                async {
                let dispatch = openhuman_core::core::runtime::DispatchCtx::ambient();
                let thread_store = openhuman_core::threads::store::ConversationStore::new(
                    dispatch,
                    owner_workspace,
                );
                assert_eq!(
                    thread_store.transcript_key("fixture-thread").unwrap().as_deref(),
                    Some("fixture-transcript-key"),
                    "thread owner did not read the v1 database transcript key"
                );
                let approvals =
                    openhuman_core::security::approval::store::list_pending(&config).unwrap();
                assert_eq!(
                    approvals.len(),
                    1,
                    "approval owner did not read the v1 database record"
                );
                assert_eq!(approvals[0].request_id, "fixture-approval");
                },
            )
            .await;
        });
    })
    .join()
    .unwrap();

    // Read the confidential record with TinyStorage's document-secret owner
    // API after the desktop boot has installed and migrated the profile DB.
    let url = format!("sqlite:{}", profile.join("storage.db").display());
    let backend = openhuman_core::storage::block_on(async move {
        openhuman_core::storage::open(&url).await
    })
    .unwrap();
    let scope = openhuman_core::storage::Scope::local().unwrap();
    let scoped = backend.for_scope(&scope).unwrap();
    let secrets = openhuman_core::storage::secrets::over(
        &scoped,
        scope,
        std::sync::Arc::new(DerivedKeys::new(Zeroizing::new([0x5a; 32]))),
    );
    let secret = openhuman_core::storage::secrets::get_blocking(&secrets, "fixture-secret")
        .unwrap()
        .unwrap();
    assert_eq!(secret.as_slice(), b"historical-secret-value");
}
