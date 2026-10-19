use super::*;
use std::sync::{Arc, Mutex};

use crate::config::Config;
use crate::core::runtime::{ContextOverlay, CoreContext, DispatchCtx, DomainSet};
use crate::storage::StorageBackend;
use tinybus::{broker::Broker, transport::memory::MemoryBus, Connection, ObjectPath};

#[derive(Default)]
struct Calls {
    methods: Mutex<Vec<String>>,
    installs: Mutex<Vec<InstallRequest>>,
}

#[derive(Clone)]
struct JuiceFixture(Arc<Calls>);

#[tinybus::interface(name = "ai.tinyhumans.tinyjuice.Compression")]
impl JuiceFixture {
    async fn install(&self, request: InstallRequest) -> tinybus::Result<()> {
        self.0.methods.lock().unwrap().push("Install".into());
        self.0.installs.lock().unwrap().push(request);
        Ok(())
    }

    async fn detect(
        &self,
        _content: String,
        _hint: tinyjuice_bus::types::ContentHint,
    ) -> tinybus::Result<String> {
        self.0.methods.lock().unwrap().push("Detect".into());
        Ok("plain_text".into())
    }

    async fn compress(
        &self,
        content: String,
        _hint: tinyjuice_bus::types::ContentHint,
    ) -> tinybus::Result<tinyjuice_bus::types::CompressedOutput> {
        self.0.methods.lock().unwrap().push("Compress".into());
        Ok(tinyjuice_bus::types::CompressedOutput::passthrough(
            content,
            tinyjuice_bus::types::ContentKind::PlainText,
        ))
    }

    async fn retrieve(
        &self,
        _token: String,
        _range: Option<tinyjuice_bus::wire::RetrieveRange>,
    ) -> tinybus::Result<Option<String>> {
        self.0.methods.lock().unwrap().push("Retrieve".into());
        Ok(Some("original".into()))
    }

    async fn cache_stats(&self) -> tinybus::Result<tinyjuice_bus::wire::CacheStats> {
        self.0.methods.lock().unwrap().push("CacheStats".into());
        Ok(tinyjuice_bus::wire::CacheStats {
            entries: 2,
            bytes: 12,
        })
    }

    async fn import_savings(
        &self,
        _aggregate: tinyjuice_bus::savings::SavingsAggregate,
    ) -> tinybus::Result<bool> {
        self.0.methods.lock().unwrap().push("ImportSavings".into());
        Ok(true)
    }
}

async fn fixture() -> (tinybus::Proxy, Arc<Calls>) {
    let bus = MemoryBus::new();
    Broker::new().spawn(bus.clone());
    let service = Connection::connect(bus.connect().await.unwrap())
        .await
        .unwrap();
    let caller = Connection::connect(bus.connect().await.unwrap())
        .await
        .unwrap();
    let calls = Arc::new(Calls::default());
    service
        .serve_at(
            ObjectPath::new(tinyjuice_bus::names::OBJECT_PATH).unwrap(),
            JuiceFixture(calls.clone()),
        )
        .await
        .unwrap();
    service
        .request_name(tinyjuice_bus::names::BUS_NAME)
        .await
        .unwrap();
    let proxy = caller
        .proxy(
            tinyjuice_bus::names::BUS_NAME,
            tinyjuice_bus::names::OBJECT_PATH,
            tinyjuice_bus::names::BUS_NAME,
        )
        .unwrap();
    (proxy, calls)
}

fn derived_context(base: &Arc<CoreContext>, config: Config, profile: &str) -> DispatchCtx {
    let overlay =
        ContextOverlay::new(config, DomainSet::full(), Default::default()).profile(profile);
    DispatchCtx::new(base.derive_with(overlay))
}

async fn invoke(ctx: DispatchCtx, function: &str, params: serde_json::Value) {
    let controller = schemas::all_registered_controllers()
        .into_iter()
        .find(|controller| controller.schema.function == function)
        .expect("the tokenjuice RPC is registered");
    crate::core::invoke::invoke_method(
        ctx,
        crate::core::invoke::default_state(),
        &controller.rpc_method_name(),
        params,
    )
    .await
    .unwrap_or_else(|error| panic!("{function} RPC failed: {error}"));
}

async fn invoke_for_config(
    ctx: DispatchCtx,
    function: &str,
    params: serde_json::Value,
    expected: &Config,
    seen: &Mutex<Vec<module_stub::ObservedConfig>>,
) {
    let before = seen.lock().unwrap().len();
    invoke(ctx, function, params).await;
    let actual = seen.lock().unwrap();
    let calls = &actual[before..];
    assert!(
        !calls.is_empty(),
        "{function} must construct a ModuleClient for its caller"
    );
    assert!(
        calls.iter().all(|actual| {
            actual.workspace_dir == expected.workspace_dir
                && actual.config_path == expected.config_path
                && actual.min_bytes_to_compress == expected.tokenjuice.min_bytes_to_compress
        }),
        "{function} constructed a ModuleClient from a different caller's config"
    );
}

fn profile_config(workspace: &std::path::Path, threshold: usize) -> Config {
    let mut config = Config::default();
    config.workspace_dir = workspace.to_owned();
    config.config_path = workspace.join("config.toml");
    config.tokenjuice.min_bytes_to_compress = threshold;
    config.tokenjuice.ccr_disk_enabled = true;
    config.tokenjuice.repl_save_enabled = true;
    config
}

struct StorageGuard(Option<Arc<dyn StorageBackend>>);

impl StorageGuard {
    fn install_memory() -> Self {
        Self(crate::storage::install(Arc::new(
            crate::storage::MemoryStorage::new(),
        )))
    }
}

impl Drop for StorageGuard {
    fn drop(&mut self) {
        crate::storage::clear();
        if let Some(previous) = self.0.take() {
            crate::storage::install(previous);
        }
    }
}

fn savings_marker_id(workspace: &std::path::Path) -> String {
    use sha2::Digest as _;
    let path = workspace.join("state").join("tokenjuice_savings.json");
    let digest = sha2::Sha256::digest(path.as_os_str().as_encoded_bytes());
    format!("savings@{}", &hex::encode(digest)[..16])
}

async fn savings_imported(workspace: &std::path::Path, profile: Option<&str>) -> bool {
    let backend = crate::storage::installed().expect("the fixture storage backend is installed");
    let scope = crate::storage::scope_for_module(profile, "tinyjuice");
    let scoped = backend.for_scope(&scope).unwrap();
    crate::storage::import_state::host_import_done(
        &scoped,
        "tinyjuice_host_import",
        &savings_marker_id(workspace),
    )
    .await
    .unwrap()
}

const SAVINGS_SNAPSHOT: &str = r#"{
    "total":{"events":1,"originalTokens":12,"compactedTokens":6,"tokensSaved":6,"costSavedUsd":0.0},
    "byModel":{},
    "byCompressor":{}
}"#;

#[tokio::test]
async fn tokenjuice_rpcs_use_their_derived_config_for_module_calls_and_install() {
    let _env_guard = crate::config::TEST_ENV_LOCK.lock().await;
    let operator = tempfile::tempdir().unwrap();
    let operator_workspace = operator.path().join("workspace");
    std::fs::create_dir_all(&operator_workspace).unwrap();
    let _workspace_env =
        crate::config::test_env::EnvVarGuard::workspace_unlocked(&operator_workspace);
    let _storage = StorageGuard::install_memory();

    let workspaces = [tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap()];
    for workspace in &workspaces {
        let state = workspace.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(state.join("tokenjuice_savings.json"), SAVINGS_SNAPSHOT).unwrap();
    }
    let configs = [
        profile_config(workspaces[0].path(), 711),
        profile_config(workspaces[1].path(), 922),
    ];
    let root_config = Config::default();
    let base = CoreContext::for_test_with_config(DomainSet::full(), root_config);
    let contexts = [
        derived_context(&base, configs[0].clone(), "juice-profile-a"),
        derived_context(&base, configs[1].clone(), "juice-profile-b"),
    ];

    let (proxy, calls) = fixture().await;
    let seen_configs = Arc::new(Mutex::new(Vec::new()));
    let client_fixture = module_stub::RpcClientFixture {
        proxy,
        configs: Arc::clone(&seen_configs),
    };
    module_stub::with_rpc_client(client_fixture, async {
        for index in 0..2 {
            invoke_for_config(
                contexts[index].clone(),
                "detect",
                serde_json::json!({ "content": "alpha" }),
                &configs[index],
                &seen_configs,
            )
            .await;
            invoke_for_config(
                contexts[index].clone(),
                "compress",
                serde_json::json!({ "content": "alpha" }),
                &configs[index],
                &seen_configs,
            )
            .await;
            invoke_for_config(
                contexts[index].clone(),
                "cache_stats",
                serde_json::json!({}),
                &configs[index],
                &seen_configs,
            )
            .await;
            invoke_for_config(
                contexts[index].clone(),
                "retrieve",
                serde_json::json!({ "token": "0123456789abcdef0123456789abcdef" }),
                &configs[index],
                &seen_configs,
            )
            .await;
        }
    })
    .await;

    let methods = calls.methods.lock().unwrap();
    for method in ["Detect", "Compress", "CacheStats", "Retrieve"] {
        assert_eq!(
            methods
                .iter()
                .filter(|actual| actual.as_str() == method)
                .count(),
            2,
            "{method} reached the TinyBus client once per derived context"
        );
    }
    assert_eq!(
        methods
            .iter()
            .filter(|actual| actual.as_str() == "ImportSavings")
            .count(),
        2,
        "legacy savings from both caller workspaces reached TinyBus"
    );
    drop(methods);
    let installs = calls.installs.lock().unwrap();
    for (index, config) in configs.iter().enumerate() {
        let disk_root = config.workspace_dir.join(".tokenjuice/ccr");
        let save_root = config.workspace_dir.join(".tokenjuice/repl");
        assert!(
            installs.iter().any(|request| {
                request.disk_tier_root.as_deref() == Some(disk_root.to_string_lossy().as_ref())
                    && request.options.repl_save_dir.as_deref() == Some(save_root.as_path())
                    && request.options.min_bytes_to_compress
                        == config.tokenjuice.min_bytes_to_compress
            }),
            "Install must carry workspace and option values from derived config {index}"
        );
        let profile = ["juice-profile-a", "juice-profile-b"][index];
    }
    drop(installs);
    for (index, config) in configs.iter().enumerate() {
        let profile = ["juice-profile-a", "juice-profile-b"][index];
        assert!(
            savings_imported(&config.workspace_dir, Some(profile)).await,
            "legacy savings import must complete in the caller's profile scope"
        );
        assert!(
            !savings_imported(&config.workspace_dir, None).await,
            "legacy savings import must not write into the controlled operator scope"
        );
    }
}
