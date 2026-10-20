use super::*;

#[tokio::test]
async fn queued_terminal_delivery_does_not_leave_a_completion_entry() {
    let shared = NativeShared::default();
    shared.begin_delivery_wait("queued-request");
    shared.abandon_delivery_wait("queued-request");

    // The queued request returns to its caller before the active runner later
    // reaches its terminal event. The event still has to release the slot.
    shared.with("queued-request", |_| {});
    shared.record_delivery("queued-request", true);

    assert!(shared
        .slots
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .is_empty());
}

#[tokio::test]
async fn active_terminal_delivery_is_returned_to_its_waiting_caller() {
    let shared = NativeShared::default();
    shared.begin_delivery_wait("active-request");
    shared.record_delivery("active-request", true);

    assert_eq!(shared.take_delivery("active-request"), Some(true));
    assert!(shared
        .slots
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .is_empty());
}
