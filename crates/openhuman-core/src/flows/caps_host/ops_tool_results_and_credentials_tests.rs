use super::*;

/// A `http_cred:<name>` ref resolves to the stored bearer credential and
/// injects `Authorization: Bearer <token>` onto the outbound request.
#[test]
fn http_cred_resolves_and_injects_bearer_header() {
    let (_dir, store) = http_cred_store();
    store
        .upsert(&HttpCredential::bearer("stripe", "sk_live_secret"))
        .unwrap();

    let cred = resolve_http_credential(&store, Some("http_cred:stripe"))
        .expect("resolve ok")
        .expect("credential present");

    let mut request = json!({ "method": "GET", "url": "https://api.example.com" });
    let header = inject_http_credential(&mut request, &cred).unwrap();
    assert_eq!(header, "Authorization");
    assert_eq!(
        request["headers"]["Authorization"],
        json!("Bearer sk_live_secret")
    );
}

/// A custom-header credential injects under its own header name while
/// preserving any headers the flow author already set.
#[test]
fn http_cred_injection_preserves_existing_headers() {
    let (_dir, store) = http_cred_store();
    store
        .upsert(&HttpCredential::header("apikey", "X-API-Key", "topsecret"))
        .unwrap();
    let cred = resolve_http_credential(&store, Some("http_cred:apikey"))
        .unwrap()
        .unwrap();

    let mut request = json!({
        "method": "POST",
        "url": "https://api.example.com",
        "headers": { "Content-Type": "application/json" }
    });
    inject_http_credential(&mut request, &cred).unwrap();
    assert_eq!(
        request["headers"]["Content-Type"],
        json!("application/json")
    );
    assert_eq!(request["headers"]["X-API-Key"], json!("topsecret"));
}

/// A basic credential injects `Authorization: Basic ...` even when the flow
/// author set no `headers` object at all.
#[test]
fn http_cred_injects_basic_into_absent_headers() {
    let (_dir, store) = http_cred_store();
    store
        .upsert(&HttpCredential::basic("acme", "alice", "pw"))
        .unwrap();
    let cred = resolve_http_credential(&store, Some("http_cred:acme"))
        .unwrap()
        .unwrap();

    let mut request = json!({ "method": "GET", "url": "https://x.example.com" });
    inject_http_credential(&mut request, &cred).unwrap();
    let value = request["headers"]["Authorization"]
        .as_str()
        .expect("Authorization header injected");
    assert!(
        value.starts_with("Basic "),
        "unexpected basic header: {value}"
    );
}

/// A `http_cred:<name>` naming a credential that does not exist FAILS the
/// request closed — it must never proceed silently unauthenticated.
#[test]
fn unknown_http_cred_fails_closed() {
    let (_dir, store) = http_cred_store();
    let result = resolve_http_credential(&store, Some("http_cred:ghost"));
    assert!(result.is_err(), "unknown http_cred must fail closed");
}

/// A malformed `http_cred:` ref (empty or whitespace-only name) must fail
/// closed the same as an unknown credential name — it must never be
/// treated as "no connection_ref" and silently sent unauthenticated
/// (Codex P2 finding).
#[test]
fn malformed_http_cred_name_fails_closed() {
    let (_dir, store) = http_cred_store();
    assert!(
        resolve_http_credential(&store, Some("http_cred:")).is_err(),
        "an empty http_cred name must fail closed, not fall through as no-op"
    );
    assert!(
        resolve_http_credential(&store, Some("http_cred:   ")).is_err(),
        "a whitespace-only http_cred name must fail closed, not fall through as no-op"
    );
}

/// No `connection_ref`, or a non-`http_cred:` prefix, injects nothing and
/// is not an error.
#[test]
fn no_http_cred_ref_injects_nothing() {
    let (_dir, store) = http_cred_store();
    assert!(resolve_http_credential(&store, None).unwrap().is_none());
    assert!(
        resolve_http_credential(&store, Some("composio:gmail:conn_1"))
            .unwrap()
            .is_none()
    );
}

/// The secret is server-side-only: the approval-gate redaction (computed on
/// the pre-injection request) never contains it, and after injection it
/// lives ONLY in the outbound `Authorization` header.
#[test]
fn injected_secret_never_reaches_the_audit_redaction() {
    let (_dir, store) = http_cred_store();
    let secret = "sk_live_never_log_me";
    store
        .upsert(&HttpCredential::bearer("stripe", secret))
        .unwrap();
    let cred = resolve_http_credential(&store, Some("http_cred:stripe"))
        .unwrap()
        .unwrap();

    let mut request = json!({ "method": "GET", "url": "https://api.example.com" });
    // Pre-injection redaction — what the approval UI / audit trail sees.
    let redacted = crate::security::approval::redact_args(&request);
    assert!(!serde_json::to_string(&redacted).unwrap().contains(secret));

    inject_http_credential(&mut request, &cred).unwrap();
    assert_eq!(
        request["headers"]["Authorization"],
        json!(format!("Bearer {secret}"))
    );
}
