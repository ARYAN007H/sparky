use super::*;
use crate::core::events::DomainEvent;
use reqwest::StatusCode;

fn response(status: StatusCode, body: &str) -> reqwest::Response {
    reqwest::Response::from(
        axum::http::Response::builder()
            .status(status)
            .body(body.to_string())
            .expect("build response"),
    )
}

/// Drain the shared bus until `pick` matches an event or the queue is empty.
fn drain_until<T>(
    rx: &mut tinybus::EventReceiver<DomainEvent>,
    mut pick: impl FnMut(DomainEvent) -> Option<T>,
) -> Option<T> {
    loop {
        match rx.try_recv() {
            Ok(event) => {
                if let Some(found) = pick(event) {
                    return Some(found);
                }
            }
            Err(tinybus::TryRecvError::Lagged(_)) => continue,
            Err(_) => return None,
        }
    }
}

#[test]
fn transient_statuses_are_not_reported_but_client_errors_are() {
    for s in [408u16, 429, 502, 503, 504, 520] {
        assert!(
            !should_report_provider_http_failure(StatusCode::from_u16(s).unwrap()),
            "{s} is transient"
        );
    }
    for s in [400u16, 401, 404, 500] {
        assert!(
            should_report_provider_http_failure(StatusCode::from_u16(s).unwrap()),
            "{s} is reportable"
        );
    }
}

#[tokio::test]
async fn api_error_replaces_ollama_cloud_500_body_with_guidance() {
    let err = api_error(
        "ollama",
        response(
            StatusCode::INTERNAL_SERVER_ERROR,
            r#"{"error":"Internal Server Error (ref: 1b2c3d4e-0000-4000-8000-000000000000)"}"#,
        ),
    )
    .await;
    assert!(tinyinference_llm::failure::is_ollama_cloud_internal_500_message(&err.to_string()));
}

#[tokio::test]
async fn api_error_replaces_no_model_loaded_body_with_guidance() {
    let err = api_error(
        "lm_studio",
        response(
            StatusCode::BAD_REQUEST,
            r#"{"error":{"message":"No models loaded. Please load a model in the developer page or via `lms load`."}}"#,
        ),
    )
    .await;
    assert!(err
        .to_string()
        .to_ascii_lowercase()
        .contains("load a model"));
}

#[tokio::test]
async fn api_error_keeps_raw_message_for_unclassified_status() {
    let err = api_error("custom", response(StatusCode::NOT_FOUND, "nope")).await;
    assert!(err.to_string().contains("custom API error (404"));
}

#[tokio::test]
async fn backend_401_publishes_session_expired_through_reporter() {
    crate::core::bus::init().await.expect("bus init");
    let mut rx = crate::core::bus::BUS.get().expect("bus").receiver();
    let marker = "REPORTER_MARKER_SESSION";
    let _ = api_error(
        "OpenHuman",
        response(StatusCode::UNAUTHORIZED, marker),
    )
    .await;
    let seen = drain_until(&mut rx, |e| match e {
        DomainEvent::SessionExpired { reason, .. } if reason.contains(marker) => Some(()),
        _ => None,
    });
    assert!(seen.is_some(), "backend 401 must publish SessionExpired");
}

#[tokio::test]
async fn byo_401_publishes_api_key_rejected_not_session_expired() {
    crate::core::bus::init().await.expect("bus init");
    let mut rx = crate::core::bus::BUS.get().expect("bus").receiver();
    let provider = "reporter_byo_provider";
    let _ = api_error(
        provider,
        response(
            StatusCode::UNAUTHORIZED,
            r#"{"error":{"message":"Invalid or missing API key","type":"authentication_error"}}"#,
        ),
    )
    .await;
    let mut session_expired = false;
    let rejected = drain_until(&mut rx, |e| match e {
        DomainEvent::SessionExpired { .. } => {
            session_expired = true;
            None
        }
        DomainEvent::ProviderApiKeyRejected { provider: p, .. } if p == provider => Some(()),
        _ => None,
    });
    assert!(rejected.is_some(), "BYO key rejection must be surfaced");
    assert!(!session_expired || rejected.is_some());
}

#[tokio::test]
async fn backend_provider_does_not_get_byo_key_rejection() {
    crate::core::bus::init().await.expect("bus init");
    let mut rx = crate::core::bus::BUS.get().expect("bus").receiver();
    let _ = api_error(
        "OpenHuman",
        response(
            StatusCode::UNAUTHORIZED,
            r#"{"error":{"type":"authentication_error"}}"#,
        ),
    )
    .await;
    let rejected = drain_until(&mut rx, |e| match e {
        DomainEvent::ProviderApiKeyRejected { provider, .. }
            if provider == "OpenHuman" =>
        {
            Some(())
        }
        _ => None,
    });
    assert!(
        rejected.is_none(),
        "backend 401 is session expiry, not a BYO key"
    );
}

/// Sentry). Getting this wrong in either direction is a regression:
/// over-matching silences real misconfig; under-matching is TAURI-RUST-N.
#[test]
fn is_backend_auth_failure_only_matches_openhuman_backend_401_403() {
    use reqwest::StatusCode;
    let backend = "OpenHuman";

    assert!(is_backend_auth_failure(backend, StatusCode::UNAUTHORIZED));
    assert!(is_backend_auth_failure(backend, StatusCode::FORBIDDEN));

    // Non-auth backend statuses stay reportable (real server bugs / transient).
    for s in [
        StatusCode::INTERNAL_SERVER_ERROR,
        StatusCode::TOO_MANY_REQUESTS,
        StatusCode::BAD_REQUEST,
        StatusCode::NOT_FOUND,
    ] {
        assert!(
            !is_backend_auth_failure(backend, s),
            "backend {s} must not be treated as session-expiry"
        );
    }

    // Third-party BYO-key 401/403 (user's own key revoked) must NOT be
    // silenced — that is actionable misconfiguration for Sentry.
    for provider in ["custom_openai", "OpenAI", "Anthropic", "openrouter"] {
        assert!(
            !is_backend_auth_failure(provider, StatusCode::UNAUTHORIZED),
            "{provider} 401 must reach Sentry as actionable BYO-key error"
        );
        assert!(
            !is_backend_auth_failure(provider, StatusCode::FORBIDDEN),
            "{provider} 403 must reach Sentry as actionable BYO-key error"
        );
    }
}

/// End-to-end through `api_error`: a non-backend 401 with an auth-error body
/// returns the sanitized provider error (so the chat/UI surface is unchanged)
/// while the BYO-auth branch demotes it from Sentry. Exercises the wired-in
/// cascade, not just the predicate in isolation.
#[tokio::test]
async fn api_error_byo_auth_failure_returns_message_via_demoted_branch() {
    let http_response = axum::http::Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .body(
            r#"{"error":{"message":"Invalid or missing API key","type":"authentication_error"}}"#
                .to_string(),
        )
        .expect("build 401 response");
    let response = reqwest::Response::from(http_response);

    let err = api_error("kiro", response).await;
    let msg = err.to_string();
    assert!(
        msg.contains("kiro API error (401"),
        "error must still carry the provider/status prefix for the UI: {msg}"
    );
    assert!(
        msg.to_ascii_lowercase()
            .contains("invalid or missing api key"),
        "sanitized upstream body must propagate to the caller: {msg}"
    );
}

/// End-to-end through `api_error`: a 500-wrapped monthly-quota refusal (the
/// Kiro IDE proxy nests its 402 / `MONTHLY_REQUEST_COUNT` inside a 500
/// envelope, TAURI-RUST-C9A) returns the sanitized provider error to the UI
/// while routing through the quota-exhausted demote branch — *before* the
/// `should_report_provider_http_failure(500)` status gate that would otherwise
/// page once per memory-extraction retry.
#[tokio::test]
async fn api_error_monthly_quota_returns_message_via_demoted_branch() {
    let body = "{\"error\":{\"message\":\"HTTP 402 from Kiro IDE: \
        {\\\"message\\\":\\\"You have reached the limit.\\\",\
        \\\"reason\\\":\\\"MONTHLY_REQUEST_COUNT\\\"}\",\"type\":\"server_error\"}}";
    let http_response = axum::http::Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(body.to_string())
        .expect("build 500 response");
    let response = reqwest::Response::from(http_response);

    let err = api_error("kiro", response).await;
    let msg = err.to_string();
    assert!(
        msg.contains("kiro API error (500"),
        "error must still carry the provider/status prefix for the UI: {msg}"
    );
    assert!(
        msg.contains("MONTHLY_REQUEST_COUNT"),
        "sanitized upstream quota body must propagate to the caller: {msg}"
    );
    // The body must classify as quota-exhausted so the demote branch — not the
    // 500 status gate — handles it.
    assert!(tinyinference_llm::failure::body_indicates_quota_exhausted(
        body
    ));
    assert!(should_report_provider_http_failure(
        StatusCode::INTERNAL_SERVER_ERROR
    ));
}

/// `publish_backend_session_expired` must emit a `SessionExpired` event on
/// the `auth` domain with the canonical source and a sanitized reason, so
/// the credentials subscriber can drive reauth.
#[tokio::test]
async fn publish_backend_session_expired_emits_sanitized_session_expired() {
    use crate::core::events::DomainEvent;

    crate::core::bus::init().await.expect("bus init");
    let mut rx = crate::core::bus::BUS
        .get()
        .expect("event bus initialized")
        .receiver();

    // `TEST_MARKER_A` makes this event distinguishable from the sibling
    // `chat_completions_backend_401_*` test's event on the shared global
    // bus (both run in parallel against the same singleton). The `sk-`
    // token probes that `sanitize_api_error` actually scrubs secrets out
    // of the SessionExpired reason rather than just emitting the event.
    let secret = ["sk-", "SYNTHETIC", "000000000000000000000000"].concat();
    let msg = format!(
        r#"OpenHuman API error (401 Unauthorized): {{"success":false,"error":"TEST_MARKER_A Invalid token {secret}"}}"#
    );
    publish_backend_session_expired(
        "chat_completions",
        "OpenHuman",
        reqwest::StatusCode::UNAUTHORIZED,
        &msg,
    );

    let mut reason_seen: Option<String> = None;
    loop {
        match rx.try_recv() {
            Ok(DomainEvent::SessionExpired { source, reason }) => {
                if source == "llm_provider.openhuman_backend" && reason.contains("TEST_MARKER_A") {
                    reason_seen = Some(reason);
                    break;
                }
            }
            Ok(_) => continue,
            Err(tinybus::TryRecvError::Lagged(_)) => continue,
            Err(_) => break,
        }
    }
    let reason = reason_seen.expect(
        "publish_backend_session_expired must emit SessionExpired(source=llm_provider.openhuman_backend) carrying TEST_MARKER_A",
    );
    assert!(
        reason.contains("[REDACTED]"),
        "sanitize_api_error must redact the sk- token in the reason: {reason}"
    );
    assert!(
        !reason.contains(&secret),
        "raw secret must not survive into the SessionExpired reason: {reason}"
    );
}

#[test]
fn skips_sentry_report_for_transient_upstream_statuses() {
    // Transient statuses — 429 rate-limit, 408 client timeout, and 502/503/504
    // gateway-layer failures — are retried by reliable.rs. The aggregate
    // "all providers exhausted" event still fires for genuine outages.
    // Reporting each attempt individually floods Sentry (OPENHUMAN-TAURI-2E
    // ~1393 events, 84 ~1050 events, T ~871 events).
    for transient in [
        reqwest::StatusCode::TOO_MANY_REQUESTS,
        reqwest::StatusCode::REQUEST_TIMEOUT,
        reqwest::StatusCode::BAD_GATEWAY,
        reqwest::StatusCode::SERVICE_UNAVAILABLE,
        reqwest::StatusCode::GATEWAY_TIMEOUT,
    ] {
        assert!(
            !should_report_provider_http_failure(transient),
            "transient status {transient} must not trigger per-attempt Sentry report"
        );
    }
    // Auth + permanent server faults remain reportable — those are
    // misconfiguration or genuine bugs, not transient capacity issues.
    for reportable in [
        reqwest::StatusCode::UNAUTHORIZED,
        reqwest::StatusCode::FORBIDDEN,
        reqwest::StatusCode::BAD_REQUEST,
        reqwest::StatusCode::NOT_FOUND,
        reqwest::StatusCode::INTERNAL_SERVER_ERROR,
    ] {
        assert!(
            should_report_provider_http_failure(reportable),
            "status {reportable} must still report to Sentry"
        );
    }
}

#[test]
fn backend_error_code_owned_gates_managed_errors_except_malformed_bad_request() {
    const PROVIDER_LABEL: &str = "OpenHuman";

    // F2/F4: backend-owned / expected-user-state errorCodes must NOT page the
    // provider HTTP layer.
    for code in [
        "RATE_LIMITED",
        "USER_INSUFFICIENT_CREDITS",
        "UPSTREAM_UNAVAILABLE",
        "MODEL_UNAVAILABLE",
        "INTERNAL_ERROR",
    ] {
        let body = format!("{{\"error\":{{\"errorCode\":\"{code}\",\"message\":\"x\"}}}}");
        assert!(
            is_backend_error_code_owned(PROVIDER_LABEL, &body),
            "errorCode={code} must be backend-owned (no provider-layer Sentry)"
        );
    }

    // A user-param BAD_REQUEST is still backend-owned (F8 only carves out the
    // malformed variant).
    assert!(is_backend_error_code_owned(
        PROVIDER_LABEL,
        "{\"error\":{\"errorCode\":\"BAD_REQUEST\",\"message\":\"bad param\"}}"
    ));

    // Client-guard-leak codes page: the client enforces these limits before
    // sending (attachment size gates; context-window management), so a backend
    // rejection means our guard leaked — the gate must NOT claim them.
    for code in ["PAYLOAD_TOO_LARGE", "CONTEXT_LENGTH_EXCEEDED"] {
        let body = format!("{{\"error\":{{\"errorCode\":\"{code}\",\"message\":\"x\"}}}}");
        assert!(
            !is_backend_error_code_owned(PROVIDER_LABEL, &body),
            "errorCode={code} is a client guard leak and must page (not owned)"
        );
    }

    // F8: a backend-flagged malformed BAD_REQUEST is also a case the FE still
    // pages — the gate must NOT claim it.
    assert!(!is_backend_error_code_owned(
        PROVIDER_LABEL,
        "{\"error\":{\"errorCode\":\"BAD_REQUEST\",\"malformed\":true}}"
    ));

    // BYO (no errorCode) is never claimed by this gate — it falls through to
    // the status-based decision.
    assert!(!is_backend_error_code_owned(
        PROVIDER_LABEL,
        "{\"error\":{\"message\":\"Incorrect API key provided\"}}"
    ));

    // CodeRabbit: a BYO / direct provider whose body merely contains an
    // `errorCode`-shaped field must NOT be claimed as backend-owned — the
    // provider gate keeps it reaching Sentry via the status decision.
    assert!(!is_backend_error_code_owned(
        "custom_openai",
        "{\"error\":{\"errorCode\":\"RATE_LIMITED\"}}"
    ));
}

// Exercises the real `config_rejection_demoted` decision used
// by `api_error`, including the inverted provider-aware polarity.
mod provider_config_rejection_suppression {
    use super::super::*;

    // The exact #2079 Sentry body shape.
    const TIER_LEAK_BODY: &str =
        "The supported API model names are deepseek-v4-pro or deepseek-v4-flash, \
         but you passed reasoning-v1.";
    // #2076 Moonshot Kimi K2 temperature constraint.
    const TEMP_BODY: &str = "invalid temperature: only 1 is allowed for this model";

    #[test]
    fn custom_provider_4xx_config_rejection_is_suppressed() {
        assert!(config_rejection_demoted(
            &tinyinference_llm::failure::http::classify_http_failure(
                "custom_openai",
                reqwest::StatusCode::BAD_REQUEST.as_u16(),
                TIER_LEAK_BODY
            ),
            "custom_openai"
        ));
        assert!(config_rejection_demoted(
            &tinyinference_llm::failure::http::classify_http_failure(
                "custom_openai",
                reqwest::StatusCode::BAD_REQUEST.as_u16(),
                TEMP_BODY
            ),
            "custom_openai"
        ));
        // 404 "model does not exist" is the same user-config class.
        assert!(config_rejection_demoted(
            &tinyinference_llm::failure::http::classify_http_failure(
                "custom_openai",
                reqwest::StatusCode::NOT_FOUND.as_u16(),
                "The model `gpt-5.5` does not exist or you do not have access to it."
            ),
            "custom_openai"
        ));
    }

    #[test]
    fn openhuman_backend_same_body_is_not_suppressed() {
        // Inverted polarity: for tier-leak / temperature / litellm /
        // OpenRouter-style phrases, the OpenHuman backend never
        // emits them, so the same body from our OWN backend would
        // mean we sent it a bad request — a real regression that
        // must still reach Sentry. (Mirror of the 401/403 backend
        // rule.)
        assert!(!config_rejection_demoted(
            &tinyinference_llm::failure::http::classify_http_failure(
                "OpenHuman",
                reqwest::StatusCode::BAD_REQUEST.as_u16(),
                TIER_LEAK_BODY
            ),
            "OpenHuman"
        ));
        assert!(!config_rejection_demoted(
            &tinyinference_llm::failure::http::classify_http_failure(
                "OpenHuman",
                reqwest::StatusCode::BAD_REQUEST.as_u16(),
                TEMP_BODY
            ),
            "OpenHuman"
        ));
    }

    #[test]
    fn openhuman_backend_openai_compatible_unknown_model_is_suppressed() {
        // TAURI-RUST-2Z1 — the OpenHuman backend DOES emit the
        // OpenAI-compatible "Model 'X' is not available. Use GET
        // /openai/v1/models …" wire body for user-configured unknown
        // model ids (here `MiniMax-M2.7-highspeed` and two
        // `custom:`-prefixed fallback variants from the user's own
        // `model_fallbacks` config). That's user-state, not a
        // regression — drop the polarity guard for this specific
        // shape so the per-attempt event stops reaching Sentry.
        // (The aggregate sibling TAURI-RUST-2Z2 is already covered by
        // `expected_error_kind` via the broader message-only
        // classifier.)
        for body in [
            r#"OpenHuman API error (400 Bad Request): {"success":false,"error":"Model 'MiniMax-M2.7-highspeed' is not available. Use GET /openai/v1/models to list available models."}"#,
            r#"OpenHuman API error (400 Bad Request): {"success":false,"error":"Model 'custom:MiniMax-M2.7' is not available. Use GET /openai/v1/models to list available models."}"#,
        ] {
            assert!(
                config_rejection_demoted(
                    &tinyinference_llm::failure::http::classify_http_failure(
                        "OpenHuman",
                        reqwest::StatusCode::BAD_REQUEST.as_u16(),
                        body
                    ),
                    "OpenHuman"
                ),
                "TAURI-RUST-2Z1 body must be suppressed for openhuman backend: {body:?}"
            );
        }
    }

    #[test]
    fn server_error_is_not_suppressed() {
        // A 5xx is a server bug, not user-config — keep reporting.
        assert!(!config_rejection_demoted(
            &tinyinference_llm::failure::http::classify_http_failure(
                "custom_openai",
                reqwest::StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                TIER_LEAK_BODY
            ),
            "custom_openai"
        ));
    }

    #[test]
    fn transient_429_is_not_suppressed_here() {
        // 429 is transient; handled by should_report_provider_http_failure,
        // not this classifier (must not be swallowed as user-config).
        assert!(!config_rejection_demoted(
            &tinyinference_llm::failure::http::classify_http_failure(
                "custom_openai",
                reqwest::StatusCode::TOO_MANY_REQUESTS.as_u16(),
                TIER_LEAK_BODY
            ),
            "custom_openai"
        ));
    }

    #[test]
    fn unrelated_4xx_body_is_not_suppressed() {
        assert!(!config_rejection_demoted(
            &tinyinference_llm::failure::http::classify_http_failure(
                "custom_openai",
                reqwest::StatusCode::BAD_REQUEST.as_u16(),
                "Bad request: missing required field 'messages'"
            ),
            "custom_openai"
        ));
    }

    /// TAURI-RUST-4XK — Ollama Cloud returns HTTP 403 with body
    /// `{"error":"this model requires a subscription, upgrade for access: …"}`.
    /// Before this fix, `is_provider_config_rejection_http` rejected 403
    /// before reaching the phrase matcher, so the subscription-gate body
    /// fell through to Sentry. Adding 403 to the allowed status set closes
    /// that gap; the existing phrase in `config_rejection.rs` already
    /// handles the body content.
    #[test]
    fn ollama_cloud_403_subscription_gate_is_suppressed() {
        // Verbatim wire body from TAURI-RUST-4XK Sentry issue 5338.
        let body = r#"ollama API error (403 Forbidden): {"error":"this model requires a subscription, upgrade for access: https://ollama.com/upgrade (ref: bc48f3c8-fba1-40b6-93a9-786a167d16f9)"}"#;
        assert!(
            config_rejection_demoted(
                &tinyinference_llm::failure::http::classify_http_failure(
                    "ollama",
                    reqwest::StatusCode::FORBIDDEN.as_u16(),
                    body
                ),
                "ollama"
            ),
            "TAURI-RUST-4XK: ollama 403 subscription-gate must be classified as provider config-rejection"
        );
    }

    #[test]
    fn openhuman_backend_403_subscription_phrase_is_not_suppressed() {
        // Polarity guard: if our own backend somehow returned a 403 with
        // the subscription phrase, that would be an unexpected regression
        // and must still reach Sentry. The phrase does not appear in any
        // expected backend body, so this is purely defensive.
        let body = r#"{"error":"this model requires a subscription, upgrade for access: https://ollama.com/upgrade (ref: test)"}"#;
        assert!(
            !config_rejection_demoted(
                &tinyinference_llm::failure::http::classify_http_failure(
                    "OpenHuman",
                    reqwest::StatusCode::FORBIDDEN.as_u16(),
                    body
                ),
                "OpenHuman"
            ),
            "backend 403 subscription phrase must NOT be suppressed (polarity guard)"
        );
    }
}
