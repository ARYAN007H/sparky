use super::*;

// #6932: the offline local profile is a valid sign-in with no TinyHumans
// account behind it. It must never be sent as a managed inference bearer.
#[test]
fn the_offline_local_session_cannot_authenticate_managed_inference() {
    use openhuman_embed::__host::security::credentials::session_support::{
        SessionTokenCheck, LOCAL_SESSION_MANAGED_INFERENCE_UNAVAILABLE,
    };

    let error = managed_bearer(
        SessionTokenCheck::Live("header.payload.local".to_string()),
        "openhuman_backend_model.resolve_bearer",
    )
    .expect_err("a local session has no managed bearer");

    assert_eq!(
        error.to_string(),
        LOCAL_SESSION_MANAGED_INFERENCE_UNAVAILABLE
    );
    assert!(
        !openhuman_embed::__host::core::observability::is_session_expired_message(
            &error.to_string()
        ),
        "the offline local session is unavailable, not expired"
    );
}

#[test]
fn a_signed_in_session_still_authenticates_managed_inference() {
    use openhuman_embed::__host::security::credentials::session_support::SessionTokenCheck;

    let bearer = managed_bearer(
        SessionTokenCheck::Live("header.payload.signature".to_string()),
        "openhuman_backend_model.resolve_bearer",
    )
    .expect("a hosted session is the bearer");

    assert_eq!(bearer, "header.payload.signature");
}
