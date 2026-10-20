use super::*;

use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::{oneshot, Notify};

#[test]
fn sandbox_callback_path_is_accepted_by_tinybus() {
    let path = super::sandbox_callback_object_path(uuid::Uuid::new_v4())
        .expect("generated callback path is a valid TinyBus object path");
    assert!(path.as_str().starts_with("/ai/tinyhumans/openhuman/FlowSandbox/"));
}

struct DropNotice {
    drops: Arc<AtomicUsize>,
    completed: Arc<Notify>,
}

impl Drop for DropNotice {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
        self.completed.notify_one();
    }
}

#[tokio::test]
async fn canceled_waiter_releases_callback_registered_after_cancellation() {
    let drops = Arc::new(AtomicUsize::new(0));
    let completed = Arc::new(Notify::new());
    let (started_tx, started_rx) = oneshot::channel();
    let (finish_tx, finish_rx) = oneshot::channel();
    let registration_drops = drops.clone();
    let registration_completed = completed.clone();

    let registration = tokio::spawn(async move {
        let _ = started_tx.send(());
        let _ = finish_rx.await;
        Ok::<_, tinybus::Error>(DropNotice {
            drops: registration_drops,
            completed: registration_completed,
        })
    });
    let waiter = tokio::spawn(await_registration(registration));

    started_rx.await.expect("registration started");
    waiter.abort();
    let _ = waiter.await;
    assert_eq!(drops.load(Ordering::SeqCst), 0);

    let _ = finish_tx.send(());
    tokio::time::timeout(std::time::Duration::from_secs(1), completed.notified())
        .await
        .expect("late registration result is dropped and releases its lease");
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}
