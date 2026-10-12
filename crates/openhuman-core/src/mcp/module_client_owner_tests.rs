use super::*;
use std::sync::atomic::AtomicUsize;
use tinybus::{broker::Broker, transport::memory::MemoryBus, Connection, ObjectPath};

#[derive(Clone, Default)]
struct LifecycleFixture {
    closes: Arc<AtomicUsize>,
    polls: Arc<AtomicUsize>,
}

#[tinybus::interface(name = "ai.tinyhumans.tinymcp.Mcp")]
impl LifecycleFixture {
    async fn close(&self, _data_dir: String) -> tinybus::Result<Option<String>> {
        self.closes.fetch_add(1, Ordering::SeqCst);
        Ok(Some("original-close".into()))
    }

    async fn transition_status(&self, _receipt: String) -> tinybus::Result<TransitionStatus> {
        self.polls.fetch_add(1, Ordering::SeqCst);
        Ok(TransitionStatus::Complete)
    }
}

#[tokio::test]
async fn cleanup_receipts_stay_with_the_original_module_after_alias_replacement() {
    let bus = MemoryBus::new();
    Broker::new().spawn(bus.clone());
    let original = Connection::connect(bus.connect().await.unwrap())
        .await
        .unwrap();
    let replacement = Connection::connect(bus.connect().await.unwrap())
        .await
        .unwrap();
    let caller = Connection::connect(bus.connect().await.unwrap())
        .await
        .unwrap();
    let original_fixture = LifecycleFixture::default();
    let replacement_fixture = LifecycleFixture::default();
    for (connection, fixture) in [
        (&original, original_fixture.clone()),
        (&replacement, replacement_fixture.clone()),
    ] {
        connection
            .serve_at(ObjectPath::new(names::OBJECT_PATH).unwrap(), fixture)
            .await
            .unwrap();
    }
    original.request_name(names::INTERFACE).await.unwrap();
    let root = ModuleClient::fixture(
        caller
            .proxy(names::INTERFACE, names::OBJECT_PATH, names::INTERFACE)
            .unwrap(),
    );
    let port = TinyMcpPort { root };
    // Resolve the original owner through a real bus call. No secret is sent
    // to these deliberately unattested lifecycle fixtures.
    assert_eq!(
        port.transition_status("original-open").await,
        Ok(TransitionStatus::Complete)
    );
    let client = McpConfiguredClient::with_port("/test/agent".into(), Box::new(port));
    *client.state.lock().await = State::Open(OpenObject {
        config: McpClientConfig::default(),
        object_path: "/test/original-child".into(),
    });
    original.release_name(names::INTERFACE).await.unwrap();
    replacement.request_name(names::INTERFACE).await.unwrap();

    client.close().await.unwrap();
    assert_eq!(
        original_fixture.closes.load(Ordering::SeqCst),
        1,
        "cleanup must address the original module instance"
    );
    assert_eq!(
        original_fixture.polls.load(Ordering::SeqCst),
        2,
        "the close receipt must be polled on its original owner"
    );
    assert_eq!(replacement_fixture.closes.load(Ordering::SeqCst), 0);
    assert_eq!(replacement_fixture.polls.load(Ordering::SeqCst), 0);
}
