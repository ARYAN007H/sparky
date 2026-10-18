use super::*;

#[test]
fn dropped_attempt_releases_in_flight_state() {
    let state = RetryState::default();
    let attempt = begin_outage_attempt(Some(&state), "stub-cloud").expect("attempt");
    drop(attempt);
    assert!(begin_outage_attempt(Some(&state), "stub-cloud").is_some());
}

#[test]
fn only_one_initial_outage_attempt_can_be_in_flight() {
    let state = RetryState::default();

    let first = begin_outage_attempt(Some(&state), "stub-cloud").expect("first attempt");
    assert!(begin_outage_attempt(Some(&state), "stub-cloud").is_none());
    drop(first);
}

#[test]
fn failed_attempt_after_concurrent_recovery_does_not_become_terminal() {
    let state = RetryState::default();
    let mut attempt = begin_outage_attempt(Some(&state), "stub-cloud").expect("attempt");

    clear_outage(Some(&state), "stub-cloud");

    assert!(matches!(
        record_outage(Some(&mut attempt)),
        None
    ));
}
