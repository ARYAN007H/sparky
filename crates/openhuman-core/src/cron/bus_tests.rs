use super::*;
use crate::channels::runtime::{ChannelHandle, TestChannel};
use std::sync::atomic::Ordering;

fn delivery_event(channel: &str) -> DomainEvent {
    DomainEvent::CronDeliveryRequested {
        job_id: "test-job".into(),
        channel: channel.into(),
        target: "chat-123".into(),
        output: "hello".into(),
    }
}

fn test_channel(name: &str, fail: bool) -> (Arc<ChannelHandle>, Arc<TestChannel>) {
    let mut state = TestChannel::new(name);
    state.send_error = fail.then(|| "mock send failure".to_owned());
    let state = Arc::new(state);
    (Arc::new(ChannelHandle::Test(Arc::clone(&state))), state)
}

fn make_subscriber(channels: Vec<Arc<ChannelHandle>>) -> CronDeliverySubscriber {
    let map: HashMap<String, Arc<ChannelHandle>> = channels
        .into_iter()
        .map(|channel| (channel.name().to_string(), channel))
        .collect();
    CronDeliverySubscriber::new(Arc::new(map))
}

#[tokio::test]
async fn ignores_non_delivery_events() {
    let (ch, state) = test_channel("telegram", false);
    let sub = make_subscriber(vec![ch]);

    sub.handle(&DomainEvent::CronJobTriggered {
        job_id: "j1".into(),
        job_name: "test-job".into(),
        job_type: "shell".into(),
    })
    .await;

    assert_eq!(state.send_count.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn dispatches_to_matching_channel() {
    let (ch, state) = test_channel("telegram", false);
    let sub = make_subscriber(vec![ch]);

    sub.handle(&delivery_event("Telegram")).await;

    assert_eq!(state.send_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn missing_channel_does_not_panic() {
    let sub = make_subscriber(vec![]);
    // Should log a warning but not panic.
    sub.handle(&delivery_event("nonexistent")).await;
}

#[tokio::test]
async fn send_failure_does_not_panic() {
    let (ch, state) = test_channel("slack", true);
    let sub = make_subscriber(vec![ch]);

    // Should log a warning but not panic.
    sub.handle(&delivery_event("slack")).await;
    assert_eq!(state.send_count.load(Ordering::SeqCst), 1);
}
