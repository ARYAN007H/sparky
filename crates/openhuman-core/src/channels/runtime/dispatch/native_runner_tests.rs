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

#[tokio::test]
async fn terminal_delivery_releases_session_profile_and_busy_guards() {
    let dispatch = crate::core::runtime::DispatchCtx::unbound();
    let profile = format!("native-runner-test-{}", uuid::Uuid::new_v4());
    let session = format!("native-session-{}", uuid::Uuid::new_v4());
    let lease = crate::agent::session_lease::SessionLease::acquire(
        crate::agent::session_lease::SessionLeaseKey::new("native-test-tenant", "agent", &session),
        None,
    )
    .await
    .expect("session lease");
    let profile_turn = crate::profiles::turn_gate::acquire_profile(&profile).await;
    let busy = crate::agent::orchestration::busy_guard::TurnBusy::start_on_thread(
        &dispatch, &session,
    );
    let shared = NativeShared::default();
    shared.begin_delivery_wait("guarded-request");
    shared.with("guarded-request", |slot| {
        slot.lease = Some(lease);
        slot.profile_turn = Some(profile_turn);
        slot.busy = Some(busy);
    });

    assert!(crate::agent::orchestration::busy_guard::is_busy(&dispatch, &session));
    let waiting = tokio::spawn({
        let profile = profile.clone();
        async move { crate::profiles::turn_gate::acquire_profile(&profile).await }
    });
    tokio::task::yield_now().await;
    assert!(!waiting.is_finished(), "profile gate stays held through delivery");

    shared.record_delivery("guarded-request", true);

    assert!(!crate::agent::orchestration::busy_guard::is_busy(&dispatch, &session));
    let next = tokio::time::timeout(std::time::Duration::from_secs(1), waiting)
        .await
        .expect("profile gate is released after delivery")
        .expect("profile waiter does not panic");
    drop(next);
}
