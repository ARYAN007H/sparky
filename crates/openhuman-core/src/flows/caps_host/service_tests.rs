use super::*;

use super::super::binding::AgentRunRegistry;

fn state() -> Arc<tokio::sync::Mutex<super::super::agent::AgentRunState>> {
    Arc::new(tokio::sync::Mutex::new(
        super::super::agent::AgentRunState::default(),
    ))
}

#[tokio::test]
async fn cancel_before_run_admission_tombstones_id_and_prevents_task_start() {
    let mut registry = AgentRunRegistry::default();
    registry.cancel("request-1".into());
    let cancellation = tokio_util::sync::CancellationToken::new();

    let admission = registry
        .admit("request-1".into(), state(), cancellation.clone())
        .unwrap();

    assert!(!admission.start_task);
    assert!(admission.cancelled_before_admission);
    assert!(cancellation.is_cancelled());
}

#[tokio::test]
async fn cancel_after_run_admission_cancels_the_existing_invocation() {
    let mut registry = AgentRunRegistry::default();
    let cancellation = tokio_util::sync::CancellationToken::new();
    let admission = registry
        .admit("request-1".into(), state(), cancellation.clone())
        .unwrap();
    assert!(admission.start_task);

    registry.cancel("request-1".into());

    assert!(cancellation.is_cancelled());
}

#[tokio::test]
async fn duplicate_agent_run_request_returns_the_original_handle_without_restarting() {
    let mut registry = AgentRunRegistry::default();
    let first_state = state();
    let first = registry
        .admit(
            "request-1".into(),
            Arc::clone(&first_state),
            tokio_util::sync::CancellationToken::new(),
        )
        .unwrap();

    let duplicate = registry
        .admit("request-1".into(), state(), tokio_util::sync::CancellationToken::new())
        .unwrap();

    assert!(first.start_task);
    assert!(!duplicate.start_task);
    assert_eq!(duplicate.entry.handle, first.entry.handle);
    assert!(Arc::ptr_eq(&duplicate.entry.state, &first_state));
}

#[test]
fn malformed_agent_request_ids_are_rejected() {
    assert!(!valid_agent_request_id(""));
    assert!(!valid_agent_request_id(&"x".repeat(129)));
    assert!(valid_agent_request_id("client-request-1"));
}
