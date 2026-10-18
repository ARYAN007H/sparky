use super::*;

#[test]
fn dropped_attempt_releases_in_flight_state() {
    let state = new_retry_state();
    let attempt = begin_outage_attempt(Some(&state), "stub-cloud").expect("attempt");
    drop(attempt);
    assert!(begin_outage_attempt(Some(&state), "stub-cloud").is_some());
}

#[test]
fn only_one_initial_outage_attempt_can_be_in_flight() {
    let state = new_retry_state();

    let first = begin_outage_attempt(Some(&state), "stub-cloud").expect("first attempt");
    assert!(begin_outage_attempt(Some(&state), "stub-cloud").is_none());
    drop(first);
}

#[test]
fn failed_attempt_after_concurrent_recovery_does_not_become_terminal() {
    let state = new_retry_state();
    let mut attempt = begin_outage_attempt(Some(&state), "stub-cloud").expect("attempt");

    clear_outage(Some(&mut attempt));

    assert!(matches!(
        record_outage(Some(&mut attempt)),
        Some(TriageOutcome::Deferred { .. })
    ));
}
