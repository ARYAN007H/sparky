use tinymemory_api::Error as MemoryApiError;

#[test]
fn upstream_errors_keep_the_memory_rpc_wire_codes_and_messages() {
    let cases = [
        (
            MemoryApiError::Off("sign in".into()),
            "MEMORY_OFF",
            "memory is off: sign in",
            true,
        ),
        (
            MemoryApiError::Unsupported("fetch".into()),
            "UNSUPPORTED",
            "unsupported: fetch",
            true,
        ),
        (
            MemoryApiError::InvalidRequest("bad filter".into()),
            "INVALID_REQUEST",
            "invalid request: bad filter",
            true,
        ),
        (
            MemoryApiError::NotFound("item".into()),
            "INVALID_REQUEST",
            "invalid request: item",
            true,
        ),
        (
            MemoryApiError::Config("missing endpoint".into()),
            "INVALID_REQUEST",
            "invalid request: missing endpoint",
            true,
        ),
        (
            MemoryApiError::Unauthorized("key rejected".into()),
            "UNAUTHORIZED",
            "unauthorized: key rejected",
            true,
        ),
        (
            MemoryApiError::InsufficientCredits("top up".into()),
            "INSUFFICIENT_CREDITS",
            "insufficient credits: top up",
            true,
        ),
        (
            MemoryApiError::Unavailable("timed out".into()),
            "UNAVAILABLE",
            "unavailable: timed out",
            true,
        ),
        (
            MemoryApiError::Conflict("already exists".into()),
            "ENGINE",
            "engine error: already exists",
            false,
        ),
        (
            MemoryApiError::Engine("index corrupt".into()),
            "ENGINE",
            "engine error: index corrupt",
            false,
        ),
    ];

    for (error, code, message, expected_user_state) in cases {
        let structured = crate::core::StructuredRpcError::from(error);
        assert_eq!(structured.data.as_ref().unwrap()["code"], code);
        assert_eq!(structured.data.as_ref().unwrap()["kind"], code);
        assert_eq!(structured.message, message);
        assert_eq!(structured.expected_user_state, expected_user_state);
        assert_eq!(
            crate::core::StructuredRpcError::decode(&structured.encode()),
            Some(structured)
        );
    }
}

#[test]
fn upstream_account_wide_classification_includes_credits_and_unavailable() {
    use tinymemory_api::Error as E;

    assert!(E::Off("disabled".into()).is_account_wide());
    assert!(E::Unauthorized("bad key".into()).is_account_wide());
    assert!(E::InsufficientCredits("top up".into()).is_account_wide());
    assert!(E::Unavailable("timeout".into()).is_account_wide());
    assert!(!E::InvalidRequest("bad filter".into()).is_account_wide());
    assert!(!E::Conflict("duplicate".into()).is_account_wide());
    assert!(!E::Engine("corrupt index".into()).is_account_wide());
}

#[test]
fn typed_insufficient_credits_maps_without_parsing_engine_text() {
    let error = crate::core::StructuredRpcError::from(MemoryApiError::InsufficientCredits(
        "HTTP 402".into(),
    ));
    assert_eq!(error.data.as_ref().unwrap()["code"], "INSUFFICIENT_CREDITS");
    assert_eq!(error.message, "insufficient credits: HTTP 402");
    assert!(error.expected_user_state);
}
