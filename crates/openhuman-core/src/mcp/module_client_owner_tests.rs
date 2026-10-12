use super::*;
use std::sync::atomic::AtomicUsize;
use tinybus::{broker::Broker, transport::memory::MemoryBus, Connection, ObjectPath};

#[derive(Clone, Default)]
struct LifecycleFixture {
    closes: Arc<AtomicUsize>,
    polls: Arc<AtomicUsize>,
    block_close_poll: Arc<AtomicBool>,
    close_poll_started: Arc<tokio::sync::Notify>,
    release_close_poll: Arc<tokio::sync::Notify>,
}

#[tinybus::interface(name = "ai.tinyhumans.tinymcp.Mcp")]
impl LifecycleFixture {
    async fn close(&self, _data_dir: String) -> tinybus::Result<Option<String>> {
        self.closes.fetch_add(1, Ordering::SeqCst);
        Ok(Some("original-close".into()))
    }

    async fn transition_status(&self, receipt: String) -> tinybus::Result<TransitionStatus> {
        self.polls.fetch_add(1, Ordering::SeqCst);
        if receipt == "original-close" && self.block_close_poll.swap(false, Ordering::SeqCst) {
            self.close_poll_started.notify_one();
            self.release_close_poll.notified().await;
        }
        Ok(TransitionStatus::Complete)
    }
}

#[tokio::test]
async fn cancelled_close_retries_its_receipt_on_the_original_owner() {
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
    let port = TinyMcpPort {
        root: ModuleClient::fixture(
            caller
                .proxy(names::INTERFACE, names::OBJECT_PATH, names::INTERFACE)
                .unwrap(),
        ),
    };
    assert_eq!(
        port.transition_status("original-open").await,
        Ok(TransitionStatus::Complete)
    );
    let client = Arc::new(McpConfiguredClient::with_port(
        "/test/agent".into(),
        Box::new(port),
    ));
    *client.state.lock().await = State::Open(OpenObject {
        config: McpClientConfig::default(),
        object_path: "/test/original-child".into(),
    });
    original_fixture
        .block_close_poll
        .store(true, Ordering::SeqCst);
    let closing = tokio::spawn({
        let client = Arc::clone(&client);
        async move { client.close().await }
    });
    tokio::time::timeout(
        Duration::from_secs(10),
        original_fixture.close_poll_started.notified(),
    )
    .await
    .expect("close receipt reaches the original module");
    closing.abort();
    assert!(closing.await.unwrap_err().is_cancelled());
    original_fixture.release_close_poll.notify_one();
    assert!(
        matches!(&*client.state.lock().await, State::Closing { transition_id, .. } if transition_id == "original-close")
    );
    original.release_name(names::INTERFACE).await.unwrap();
    replacement.request_name(names::INTERFACE).await.unwrap();
    client.close().await.unwrap();
    assert_eq!(original_fixture.closes.load(Ordering::SeqCst), 1);
    assert_eq!(
        original_fixture.polls.load(Ordering::SeqCst),
        3,
        "retry must poll the retained receipt on the original owner"
    );
    assert_eq!(replacement_fixture.closes.load(Ordering::SeqCst), 0);
    assert_eq!(replacement_fixture.polls.load(Ordering::SeqCst), 0);
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
