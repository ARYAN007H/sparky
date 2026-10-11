//! A library host initializes encryption in its supplied workspace without
//! keyring environment setup or an OS provider.

mod common;

use common::{offline_config, runtime_without_master_key};
use openhuman_embed::{RuntimeBuilder, ServiceSet, Workspace};

#[test]
fn embedded_key_initialization_uses_the_supplied_workspace() {
    std::thread::Builder::new()
        .stack_size(openhuman_core::core::runtime::AGENT_WORKER_STACK_BYTES)
        .spawn(|| {
            runtime_without_master_key().block_on(async {
                let root = tempfile::tempdir().unwrap();
                let mut config = offline_config();
                config.workspace_dir = root.path().join("workspace");
                config.config_path = root.path().join("config.toml");
                let workspace = config.workspace_dir.clone();
                let runtime = RuntimeBuilder::lean()
                    .workspace(Workspace::Dir(workspace.clone()))
                    .config(config)
                    .services(ServiceSet::none())
                    .build()
                    .await
                    .unwrap();
                let key = workspace.join("keyring/master.key");
                assert!(
                    key.is_file(),
                    "master key must live in the supplied workspace"
                );
                assert!(
                    !workspace.join("dev-keychain.json").exists(),
                    "the library default must encrypt its secret store"
                );
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    assert_eq!(
                        std::fs::metadata(key).unwrap().permissions().mode() & 0o777,
                        0o600
                    );
                }
                drop(runtime);
            })
        })
        .expect("test thread")
        .join()
        .expect("test thread should not panic");
}
