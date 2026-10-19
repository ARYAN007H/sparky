use super::*;
use std::sync::{Arc, Mutex};

use crate::core::runtime::{ContextOverlay, CoreContext, DispatchCtx, DomainSet};
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
    Ok((proxy, calls))
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

fn profile_config(workspace: &std::path::Path, threshold: usize) -> Config {
    let mut config = Config::default();
    config.workspace_dir = workspace.to_owned();
    config.config_path = workspace.join("config.toml");
    config.tokenjuice.min_bytes_to_compress = threshold;
    config.tokenjuice.ccr_disk_enabled = true;
    config.tokenjuice.repl_save_enabled = true;
    config
}

#[tokio::test]
async fn tokenjuice_rpcs_use_their_derived_config_for_module_calls_and_install() {
    let _env_guard = crate::config::TEST_ENV_LOCK.lock().await;
    let operator = tempfile::tempdir().unwrap();
    let operator_workspace = operator.path().join("workspace");
    std::fs::create_dir_all(&operator_workspace).unwrap();
    let previous_workspace = std::env::var_os("OPENHUMAN_WORKSPACE");
    std::env::set_var("OPENHUMAN_WORKSPACE", &operator_workspace);

    let workspaces = [tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap()];
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
    let result = module_stub::with_rpc_client(client_fixture, async {
        invoke(
            contexts[0].clone(),
            "detect",
            serde_json::json!({ "content": "alpha" }),
        )
        .await;
        invoke(
            contexts[0].clone(),
            "compress",
            serde_json::json!({ "content": "alpha" }),
        )
        .await;
        invoke(contexts[1].clone(), "cache_stats", serde_json::json!({})).await;
        invoke(
            contexts[1].clone(),
            "retrieve",
            serde_json::json!({ "token": "0123456789abcdef0123456789abcdef" }),
        )
        .await;
    })
    .await;

    if let Some(previous) = previous_workspace {
        std::env::set_var("OPENHUMAN_WORKSPACE", previous);
    } else {
        std::env::remove_var("OPENHUMAN_WORKSPACE");
    }
    result;

    let captured = seen_configs.lock().unwrap();
    for (index, config) in configs.iter().enumerate() {
        assert!(
            captured
                .iter()
                .any(|actual| actual.workspace_dir == config.workspace_dir),
            "RPC operations must construct their module clients from derived config {index}"
        );
    }
    let methods = calls.methods.lock().unwrap();
    for method in ["Detect", "Compress", "CacheStats", "Retrieve"] {
        assert!(
            methods.iter().any(|actual| actual == method),
            "{method} reached the TinyBus client"
        );
    }
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
    }
}
