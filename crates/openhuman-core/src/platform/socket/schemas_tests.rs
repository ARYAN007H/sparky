use super::*;
use crate::core::all::{
    assert_schema_controller_parity, expect_schema_fields, expect_schema_identity,
    expect_schema_required_inputs,
};
use crate::core::runtime::DispatchCtx;

#[test]
fn socket_schema_contracts_are_preserved() {
    assert_schema_controller_parity(
        &all_controller_schemas(),
        &all_registered_controllers(),
        "connect",
    );
    let functions: Vec<_> = all_controller_schemas()
        .iter()
        .map(|schema| schema.function)
        .collect();
    assert_eq!(functions.len(), 5);
    for expected in ["connect", "disconnect", "state", "emit", "connect_with_session"] {
        assert!(functions.contains(&expected), "missing socket.{expected}");
    }
    for schema in all_controller_schemas() {
        assert_eq!(schema.namespace, "socket");
        assert!(
            !schema.outputs.is_empty(),
            "{} output fields",
            schema.function
        );
    }
    for (function, required) in [
        ("connect", &["url", "token"][..]),
        ("disconnect", &[][..]),
        ("state", &[][..]),
        ("connect_with_session", &[][..]),
    ] {
        let schema = schemas(function);
        expect_schema_identity(&schema, "socket", function);
        expect_schema_required_inputs(&schema, required);
        if required.is_empty() {
            assert!(schema.inputs.is_empty());
        }
    }
    let emit = schemas("emit");
    expect_schema_identity(&emit, "socket", "emit");
    expect_schema_fields(&emit, &[("event", Some(true)), ("data", Some(false))]);
}

// ── handlers (without manager): require_manager errors ─────────

#[tokio::test]
async fn handlers_error_without_initialized_manager() {
    // Production bootstrap calls `set_global_socket_manager` once; in
    // these unit tests the global singleton is intentionally NOT set,
    // so every handler should hit the `SocketManager not initialized`
    // branch via `require_manager()` first.
    //
    // We can't reliably clear a OnceLock once set. If another test in
    // the same binary has already installed a global manager, skip
    // rather than cross-contaminating.
    if super::global_socket_manager().is_some() {
        eprintln!(
            "[socket:schemas tests] global manager already installed — \
             skipping require_manager error-path assertions"
        );
        return;
    }

    let err = handle_disconnect(DispatchCtx::process_default(), Map::new())
        .await
        .unwrap_err();
    assert!(err.contains("SocketManager not initialized"));

    let err = handle_state(DispatchCtx::process_default(), Map::new())
        .await
        .unwrap_err();
    assert!(err.contains("SocketManager not initialized"));

    let err = handle_connect(DispatchCtx::process_default(), Map::new())
        .await
        .unwrap_err();
    assert!(err.contains("SocketManager not initialized"));

    let err = handle_emit(DispatchCtx::process_default(), Map::new())
        .await
        .unwrap_err();
    assert!(err.contains("SocketManager not initialized"));
}

/// #6111 — the lifecycle handlers and `socket_state` must publish `status` in one vocabulary.
///
/// The three lifecycle handlers used to build their payload with `format!("{:?}", status)` while
/// `socket_state` went through `ConnectionStatus`' `#[serde(rename_all = "lowercase")]`, so one
/// namespace answered `"Disconnected"` and `"disconnected"` for the same field. This pins the
/// decided direction (serde) against the source of truth — the serde encoding itself — rather
/// than against a hand-written list that could drift with it.
#[test]
fn status_payload_matches_the_serde_encoding_for_every_variant() {
    for status in [
        ConnectionStatus::Disconnected,
        ConnectionStatus::Connecting,
        ConnectionStatus::Connected,
        ConnectionStatus::Reconnecting,
        ConnectionStatus::Error,
    ] {
        let state = SocketState {
            status,
            socket_id: None,
            error: None,
        };

        // What `socket_state` publishes for this status.
        let from_state = serde_json::to_value(&state).expect("serialize SocketState");
        let expected = from_state
            .get("status")
            .and_then(Value::as_str)
            .expect("SocketState serialises a status field");

        // What the lifecycle handlers publish for the same status.
        let payload = status_payload(&state);
        assert_eq!(
            payload.get("status").and_then(Value::as_str),
            Some(expected),
            "lifecycle handlers and socket_state must agree on {status:?}"
        );
        assert_ne!(
            payload.get("status").and_then(Value::as_str),
            Some(format!("{status:?}").as_str()),
            "{status:?} must not be published in Rust's Debug spelling"
        );
    }
}

/// The slug is lowercase for every variant — the property `connectivity_diag` and the frontend's
/// lowercase comparisons rely on.
#[test]
fn status_slug_is_lowercase_for_every_variant() {
    for status in [
        ConnectionStatus::Disconnected,
        ConnectionStatus::Connecting,
        ConnectionStatus::Connected,
        ConnectionStatus::Reconnecting,
        ConnectionStatus::Error,
    ] {
        let slug = status_slug(status);
        assert!(!slug.is_empty(), "{status:?} produced an empty slug");
        assert_eq!(
            slug,
            slug.to_lowercase(),
            "{status:?} slug is not lowercase"
        );
    }
}
