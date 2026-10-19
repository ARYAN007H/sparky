use super::*;
use std::time::Duration;

/// This uses the same fixed-deadline helper as the backend socket subscriber.
/// Progress at 100 seconds is followed by a terminal event at 201 seconds,
/// which the shared turn runner accepts but the subscriber legacy wrapper
/// currently cuts off at 180 seconds.
#[tokio::test(start_paused = true)]
async fn backend_channel_wait_does_not_cut_off_a_live_turn_after_180_seconds() {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let event_loop = async move {
        let consumer = async move {
            let mut progress = 0;
            while let Some(event) = rx.recv().await {
                match event {
                    "progress" => progress += 1,
                    "terminal" => return progress,
                    _ => {}
                }
            }
            progress
        };
        let producer = async move {
            tokio::time::sleep(Duration::from_secs(100)).await;
            tx.send("progress").expect("event consumer remains active");
            tokio::time::sleep(Duration::from_secs(101)).await;
            tx.send("terminal")
                .expect("terminal consumer remains active");
        };
        let (progress, ()) = tokio::join!(consumer, producer);
        progress
    };

    let result = run_channel_reply_loop(async {
        let _progress_count = event_loop.await;
    })
    .await;
    assert!(
        result.is_ok(),
        "the live turn should reach its terminal event after 201 seconds"
    );
}
