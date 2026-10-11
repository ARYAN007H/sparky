//! MemoryBus tests for the TinyBox host adapter.

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use tinybus::broker::Broker;
use tinybus::service::Interface;
use tinybus::transport::memory::MemoryBus;
use tinybus::{Connection, InterfaceName, MemberName, ObjectPath};

use super::*;

struct MockTinyBox {
    calls: Arc<Mutex<Vec<(String, Value)>>>,
}

#[async_trait::async_trait]
impl Interface for MockTinyBox {
    fn name(&self) -> InterfaceName {
        InterfaceName::new(contract::INTERFACE).expect("TinyBox interface")
    }

    fn members(&self) -> Vec<MemberName> {
        contract::METHODS
            .iter()
            .map(|member| MemberName::new(*member).expect("TinyBox contract member"))
            .collect()
    }

    async fn call(&self, member: &MemberName, args: Value) -> tinybus::Result<Value> {
        self.calls
            .lock()
            .expect("call log lock")
            .push((member.as_str().to_owned(), args));
        match member.as_str() {
            CAPABILITIES => serde_json::to_value(contract::ModuleCapabilities {
                contract_version: contract::CONTRACT_VERSION,
                create_backends: vec!["docker".to_owned()],
                exec_backends: vec!["docker".to_owned()],
                spawn_backends: vec!["docker".to_owned()],
            })
            .map_err(tinybus::Error::from),
            ANALYZE_COMMAND => serde_json::to_value(contract::CommandAnalysis {
                segments: Vec::new(),
                hidden_execution: false,
                redirection: false,
                expansion: false,
                tee: false,
                background: false,
                literal_words: vec!["printf".to_owned(), "hello".to_owned()],
            })
            .map_err(tinybus::Error::from),
            EXEC => Err(tinybus::Error::MethodFailed {
                name: "ai.tinyhumans.tinybox.Error.Backend".to_owned(),
                message: "private argument /home/user/token leaked by provider".to_owned(),
            }),
            _ => serde_json::to_value(()).map_err(tinybus::Error::from),
        }
    }
}

async fn mock_client() -> (Connection, ModuleClient, Arc<Mutex<Vec<(String, Value)>>>) {
    let bus = MemoryBus::new();
    Broker::new().spawn(bus.clone());
    let service = Connection::connect(bus.connect().await.expect("service transport"))
        .await
        .expect("service connection");
    let calls = Arc::new(Mutex::new(Vec::new()));
    service
        .serve_at(
            ObjectPath::new(contract::OBJECT_PATH).expect("TinyBox path"),
            MockTinyBox {
                calls: Arc::clone(&calls),
            },
        )
        .await
        .expect("serve mock TinyBox");
    service
        .request_name(contract::INTERFACE)
        .await
        .expect("TinyBox bus name");
    let client = Connection::connect(bus.connect().await.expect("client transport"))
        .await
        .expect("client connection");
    let proxy = client
        .proxy(
            contract::INTERFACE,
            contract::OBJECT_PATH,
            contract::INTERFACE,
        )
        .expect("TinyBox proxy");
    (service, ModuleClient::fixture(proxy), calls)
}

#[test]
fn host_and_module_contract_versions_must_cover_the_full_host_surface() {
    assert_eq!(require_compatible(contract::CONTRACT_VERSION), Ok(()));
    assert_eq!(
        require_compatible((1, contract::CONTRACT_VERSION.1 + 1)),
        Ok(())
    );
    assert_eq!(
        require_compatible((1, contract::CONTRACT_VERSION.1 - 1))
            .expect_err("older member set is refused"),
        ModuleCallError::IncompatibleContract
    );
    assert_eq!(
        require_compatible((2, contract::CONTRACT_VERSION.1))
            .expect_err("different major is refused"),
        ModuleCallError::IncompatibleContract
    );
}

#[tokio::test]
async fn command_facts_use_contract_member_and_positional_arguments() {
    let (_service, client, calls) = mock_client().await;
    let command = "printf 'hello'".to_owned();
    let facts: contract::CommandAnalysis =
        call_with_client(&client, ANALYZE_COMMAND, (command.clone(),), false)
            .await
            .expect("typed command facts");

    assert_eq!(facts.literal_words, ["printf", "hello"]);
    assert_eq!(
        calls.lock().expect("call log lock").as_slice(),
        [
            (CAPABILITIES.to_owned(), json!([])),
            (ANALYZE_COMMAND.to_owned(), json!([command]))
        ]
    );
}

#[tokio::test]
async fn provider_failure_does_not_echo_paths_or_request_values() {
    let (_service, client, calls) = mock_client().await;
    let secret_path = "/home/user/token";
    let error = call_with_client::<_, contract::ExecOutput>(
        &client,
        EXEC,
        (json!({"argv": ["cat", secret_path]}),),
        false,
    )
    .await
    .expect_err("provider refusal");

    assert!(error.contains("MODULE_CALL_REPORTED"), "{error}");
    assert!(!error.contains(secret_path));
    let calls = calls.lock().expect("call log lock");
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[1].0, EXEC);
    assert!(calls[1].1.to_string().contains(secret_path));
}

#[tokio::test]
async fn confidential_execution_refusal_is_not_retried_as_an_ordinary_call() {
    let (_service, client, calls) = mock_client().await;
    let error = call_with_client::<_, contract::ProcessRef>(
        &client,
        START_EXEC,
        (json!({"process": "private"}),),
        true,
    )
    .await
    .expect_err("an unattested fixture cannot receive confidential execution");

    assert!(error.contains("MODULE_CALL_REPORTED"), "{error}");
    let calls = calls.lock().expect("call log lock");
    assert_eq!(calls.len(), 1, "only the public capabilities probe ran");
    assert_eq!(calls[0].0, CAPABILITIES);
}

#[tokio::test]
#[ignore = "requires OPENHUMAN_BUNDLED_MODULES containing the pinned TinyBox release archive"]
async fn pinned_loader_serves_the_published_tinybox_release() {
    assert!(
        std::env::var_os(crate::modules::ops::BUNDLED_MODULES_ENV).is_some(),
        "set OPENHUMAN_BUNDLED_MODULES to the directory containing the pinned release archive"
    );

    let install = tempfile::tempdir().expect("temporary verified module cache");
    let mut config = Config::default();
    config.modules.allow_download = false;
    config.modules.install_dir = Some(install.path().display().to_string());

    let capabilities = super::capabilities(&config)
        .await
        .expect("the OpenHuman release registry and TinyBus loader admit v0.1.18");
    assert_eq!(capabilities.contract_version, contract::CONTRACT_VERSION);
    assert!(capabilities.create_backends.contains(&"docker".to_owned()));
    assert!(capabilities.exec_backends.contains(&"docker".to_owned()));
}

#[test]
fn every_host_operation_is_a_member_of_the_canonical_contract() {
    let methods = [
        DESCRIBE,
        CREATE,
        EXEC,
        INSPECT,
        CLOSE,
        SPAWN,
        IS_RUNNING,
        CANCEL,
        ANALYZE_SHELL,
        RESERVE,
        CAPABILITIES,
        SHUTDOWN,
        FORWARD,
        CLOSE_FORWARD,
        JAIL_STATUS,
        ANALYZE_COMMAND,
        BEGIN_FILE_READ,
        READ_FILE_CHUNK,
        FINISH_FILE_READ,
        BEGIN_FILE_WRITE,
        WRITE_FILE_CHUNK,
        FINISH_FILE_WRITE,
        ABORT_FILE_WRITE,
        START_EXEC,
        READ_OUTPUT,
        RELEASE_OUTPUT,
        START_JAILED_EXEC,
    ];
    assert_eq!(methods, contract::METHODS);
}
