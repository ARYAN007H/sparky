use super::*;
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use tinychannels_bus::ChannelMessage;
use tokio::sync::mpsc;

struct MockChannel {
    sends: Arc<AtomicUsize>,
    last_target: Arc<Mutex<String>>,
}

#[async_trait]
impl Channel for MockChannel {
    fn name(&self) -> &str {
        "telegram"
    }
    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        self.sends.fetch_add(1, Ordering::SeqCst);
        *self.last_target.lock().unwrap() = format!(
            "{}|{}",
            message.recipient,
            message.idempotency_key.clone().unwrap_or_default()
        );
        Ok(())
    }
    async fn listen(&self, _tx: mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        Ok(())
    }
}

// One test: the bridge is process-global, so keep its steps sequential.
#[tokio::test]
async fn bridge_sends_with_a_run_specific_idempotency_key() {
    let sends = Arc::new(AtomicUsize::new(0));
    let last_target = Arc::new(Mutex::new(String::new()));
    let ch: Arc<dyn Channel> = Arc::new(MockChannel {
        sends: Arc::clone(&sends),
        last_target: Arc::clone(&last_target),
    });
    let channels = Arc::new(HashMap::from([("telegram".to_string(), ch)]));
    register_channel_bridge(channels);

    send_to_channel("Telegram", "42", None, "hi", "cron:j:r1")
        .await
        .unwrap();
    assert_eq!(sends.load(Ordering::SeqCst), 1);
    assert_eq!(
        *last_target.lock().unwrap(),
        "42|cron:j:r1",
        "the send carries the run-specific idempotency key"
    );
    assert!(send_to_channel("slack", "42", None, "hi", "k")
        .await
        .is_err());
}
