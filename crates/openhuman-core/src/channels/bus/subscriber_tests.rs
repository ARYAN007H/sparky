use super::*;
use crate::core::runtime::DispatchCtx;
use std::sync::Mutex;
use std::time::Duration;
use tinychannels_bus::web::{WebChannelEvent, WebEventContext};

#[derive(Default)]
struct RecordingReply {
    progress: Mutex<Vec<ProgressiveEvent>>,
    finalized: Mutex<Vec<String>>,
}

#[async_trait::async_trait]
impl ProgressiveReplySink for RecordingReply {
    async fn feed(&self, events: Vec<ProgressiveEvent>) -> Result<(), String> {
        self.progress.lock().unwrap().extend(events);
        Ok(())
    }

    async fn finalize(&self, text: &str) -> Result<(), String> {
        self.finalized.lock().unwrap().push(text.to_owned());
        Ok(())
    }
}

/// The real backend-channel event consumer must keep the live shared-runner
/// turn through its terminal event. Progress at 100 seconds is followed by a
/// successful runner completion at 201 seconds; the old subscriber deadline
/// finalizes its timeout copy at 180 seconds instead.
#[tokio::test(start_paused = true)]
async fn backend_channel_wait_does_not_cut_off_a_live_turn_after_180_seconds() {
    let _serial = crate::web_chat::RUN_CHAT_TASK_TEST_LOCK.lock().await;
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let block = crate::web_chat::TestRunChatTaskBlock {
        started: Default::default(),
        dropped: Default::default(),
        release: Default::default(),
        max_park_for: Duration::from_secs(300),
        succeed_in: Some(workspace.path().to_path_buf()),
    };
    crate::web_chat::set_test_run_chat_task_block(Some(block.clone())).await;

    let ctx = DispatchCtx::ambient();
    let client_id = "channel-timeout-client";
    let thread_id = "channel-timeout-thread";
    let mut event_rx = crate::web_chat::subscribe_web_channel_events();
    let request_id = crate::web_chat::start_chat(
        &ctx,
        client_id,
        thread_id,
        "keep working",
        None,
        None,
        None,
        None,
        crate::web_chat::ChatRequestMetadata {
            source: Some("channel_inbound".into()),
            ..Default::default()
        },
    )
    .await
    .expect("the shared runner accepts the inbound channel turn");
    while !block.started.load(std::sync::atomic::Ordering::SeqCst) {
        tokio::task::yield_now().await;
    }

    let sink = RecordingReply::default();
    let session = tokio_util::sync::CancellationToken::new();
    let producer = async {
        tokio::time::sleep(Duration::from_secs(100)).await;
        crate::web_chat::publish_web_channel_event_in(
            &ctx,
            WebChannelEvent::TextDelta {
                context: WebEventContext {
                    client_id: client_id.into(),
                    thread_id: thread_id.into(),
                    request_id: request_id.clone(),
                    ..Default::default()
                },
                delta: Some("still working".into()),
                delta_kind: None,
                round: Some(1),
            },
        );
        tokio::time::sleep(Duration::from_secs(101)).await;
        block.release.notify_one();
    };
    let consumer = run_channel_reply_loop(event_rx, request_id, "telegram".into(), session, &sink);
    tokio::join!(producer, consumer);
    crate::web_chat::set_test_run_chat_task_block(None).await;

    assert_eq!(
        *sink.progress.lock().unwrap(),
        vec![ProgressiveEvent::TextDelta("still working".into())]
    );
    assert_eq!(
        *sink.finalized.lock().unwrap(),
        vec!["parked reply".to_string()],
        "the subscriber should deliver the runner's terminal reply once, not its timeout copy"
    );
}
