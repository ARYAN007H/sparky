use super::*;

#[test]
fn capability_payload_and_audit_display_match_the_original_error_envelope() {
    let payload = capability_payload(EngineError::Capability("blocked by autonomy tier".into()));
    assert_eq!(payload, "blocked by autonomy tier");
    assert_eq!(
        capability_error_display(&payload),
        EngineError::Capability("blocked by autonomy tier".into()).to_string()
    );
}
