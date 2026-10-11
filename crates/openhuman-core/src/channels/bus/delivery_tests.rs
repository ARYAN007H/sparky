use super::*;
use crate::config::test_env::EnvVarGuard;

#[test]
fn outbound_body_has_stable_idempotency_key() {
    let body = json!({ "text": "hello" });
    let first = channel_message_body_with_idempotency("telegram", body.clone());
    let second = channel_message_body_with_idempotency("telegram", body);

    assert_eq!(first, second);
    assert!(first.get("idempotencyKey").is_some());
}

#[tokio::test]
async fn transport_captures_no_local_or_missing_hosted_credential() {
    let _env_lock = crate::config::TEST_ENV_LOCK.lock().await;
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let _workspace_env = EnvVarGuard::workspace_unlocked(workspace.path());
    assert!(ManagedTransport::capture(&crate::config::Config::default()).is_err());
}
