//! JSON-RPC E2E cases for the auth security namespace.

#[path = "support/rpc_e2e_harness.rs"]
mod harness;
use harness::*;

#[tokio::test]
async fn notification_settings_roundtrip_and_disabled_ingest_skip() {
    let _env_lock = json_rpc_e2e_env_lock_async().await;
    let tmp = tempdir().expect("tempdir");
    let home = tmp.path();
    let openhuman_home = home.join(".openhuman");

    let _home_guard = EnvVarGuard::set_to_path("HOME", home);
    let _workspace_guard = EnvVarGuard::unset("OPENHUMAN_WORKSPACE");
    let _backend_url_guard = EnvVarGuard::unset("BACKEND_URL");
    let _vite_backend_guard = EnvVarGuard::unset("VITE_BACKEND_URL");

    let (mock_addr, mock_join) = serve_on_ephemeral(mock_upstream_router()).await;
    let mock_origin = format!("http://{}", mock_addr);
    write_min_config(&openhuman_home, &mock_origin);

    let (rpc_addr, rpc_join) = serve_on_ephemeral(build_core_http_router(false)).await;
    let rpc_base = format!("http://{}", rpc_addr);
    tokio::time::sleep(Duration::from_millis(100)).await;

    let set = post_json_rpc(
        &rpc_base,
        4001,
        "openhuman.notification_settings_set",
        json!({
            "provider": "gmail",
            "enabled": false,
            "importance_threshold": 0.8,
            "route_to_orchestrator": false
        }),
    )
    .await;
    let set_result = assert_no_jsonrpc_error(&set, "notification_settings_set");
    assert_eq!(set_result.get("ok").and_then(Value::as_bool), Some(true));

    let get = post_json_rpc(
        &rpc_base,
        4002,
        "openhuman.notification_settings_get",
        json!({ "provider": "gmail" }),
    )
    .await;
    let get_result = assert_no_jsonrpc_error(&get, "notification_settings_get");
    let settings = get_result.get("settings").expect("settings object");
    assert_eq!(
        settings.get("enabled").and_then(Value::as_bool),
        Some(false)
    );
    let threshold = settings
        .get("importance_threshold")
        .and_then(Value::as_f64)
        .unwrap_or_default();
    assert!(
        (threshold - 0.8).abs() < 0.0001,
        "expected threshold ~= 0.8, got {threshold}"
    );
    assert_eq!(
        settings
            .get("route_to_orchestrator")
            .and_then(Value::as_bool),
        Some(false)
    );

    let ingest = post_json_rpc(
        &rpc_base,
        4003,
        "openhuman.notification_ingest",
        json!({
            "provider": "gmail",
            "account_id": "acct-1",
            "title": "subject",
            "body": "body",
            "raw_payload": { "source": "test" }
        }),
    )
    .await;
    let ingest_result = assert_no_jsonrpc_error(&ingest, "notification_ingest");
    assert_eq!(
        ingest_result.get("skipped").and_then(Value::as_bool),
        Some(true)
    );

    mock_join.abort();
    rpc_join.abort();
}


#[tokio::test]
async fn credentials_crud_roundtrip() {
    // Tests the provider-credential lifecycle over the JSON-RPC transport:
    //   store → list → list-filtered → remove → verify-gone
    //
    // Provider credentials are stored locally (auth-profiles.json) and require
    // no upstream network calls, so no mock session/JWT is needed.
    let _env_lock = json_rpc_e2e_env_lock_async().await;
    let tmp = tempdir().expect("tempdir");
    let home = tmp.path();
    let openhuman_home = home.join(".openhuman");

    let _home_guard = EnvVarGuard::set_to_path("HOME", home);
    let _workspace_guard = EnvVarGuard::unset("OPENHUMAN_WORKSPACE");
    let _backend_url_guard = EnvVarGuard::unset("BACKEND_URL");
    let _vite_backend_guard = EnvVarGuard::unset("VITE_BACKEND_URL");

    // A mock upstream is required so config validation passes and api_url is
    // well-formed, even though provider-credential calls don't hit the network.
    let (mock_addr, mock_join) = serve_on_ephemeral(mock_upstream_router()).await;
    let mock_origin = format!("http://{}", mock_addr);
    write_min_config(&openhuman_home, &mock_origin);

    let (rpc_addr, rpc_join) = serve_on_ephemeral(build_core_http_router(false)).await;
    let rpc_base = format!("http://{}", rpc_addr);
    tokio::time::sleep(Duration::from_millis(100)).await;

    // ── 1. store a provider credential ──────────────────────────────────────
    let store = post_json_rpc(
        &rpc_base,
        5001,
        "openhuman.auth_store_provider_credentials",
        json!({
            "provider": "openai",
            "profile": "default",
            "token": "sk-e2e-test-key",
            "setActive": true
        }),
    )
    .await;
    // assert_no_jsonrpc_error returns the JSON-RPC `result` field which is the
    // Outcome envelope: {"logs": [...], "result": { <AuthProfileSummary> }}.
    let store_outer = assert_no_jsonrpc_error(&store, "auth_store_provider_credentials");
    let store_result = store_outer.get("result").unwrap_or(store_outer);
    assert_eq!(
        store_result.get("provider").and_then(Value::as_str),
        Some("openai"),
        "stored profile should have provider=openai: {store_result}"
    );
    assert_eq!(
        store_result.get("profileName").and_then(Value::as_str),
        Some("default"),
        "stored profile should have profileName=default: {store_result}"
    );
    assert_eq!(
        store_result.get("hasToken").and_then(Value::as_bool),
        Some(true),
        "stored profile should report hasToken=true: {store_result}"
    );

    // ── 2. list all provider credentials — should find openai ───────────────
    let list_all = post_json_rpc(
        &rpc_base,
        5002,
        "openhuman.auth_list_provider_credentials",
        json!({}),
    )
    .await;
    let list_outer = assert_no_jsonrpc_error(&list_all, "auth_list_provider_credentials (all)");
    let list_result = list_outer.get("result").unwrap_or(list_outer);
    let profiles = list_result
        .as_array()
        .unwrap_or_else(|| panic!("expected array from list: {list_result}"));
    assert_eq!(profiles.len(), 1, "expected exactly one stored credential");
    assert_eq!(
        profiles[0].get("provider").and_then(Value::as_str),
        Some("openai")
    );

    // ── 3. list filtered by provider name ───────────────────────────────────
    let list_filtered = post_json_rpc(
        &rpc_base,
        5003,
        "openhuman.auth_list_provider_credentials",
        json!({ "provider": "openai" }),
    )
    .await;
    let filtered_outer =
        assert_no_jsonrpc_error(&list_filtered, "auth_list_provider_credentials (filtered)");
    let filtered_result = filtered_outer.get("result").unwrap_or(filtered_outer);
    let filtered_profiles = filtered_result
        .as_array()
        .unwrap_or_else(|| panic!("expected array from filtered list: {filtered_result}"));
    assert_eq!(
        filtered_profiles.len(),
        1,
        "filter by openai should return exactly one entry"
    );

    // ── 4. remove the stored credential ─────────────────────────────────────
    let remove = post_json_rpc(
        &rpc_base,
        5004,
        "openhuman.auth_remove_provider_credentials",
        json!({
            "provider": "openai",
            "profile": "default"
        }),
    )
    .await;
    let remove_outer = assert_no_jsonrpc_error(&remove, "auth_remove_provider_credentials");
    let remove_result = remove_outer.get("result").unwrap_or(remove_outer);
    assert_eq!(
        remove_result.get("removed").and_then(Value::as_bool),
        Some(true),
        "remove should report removed=true: {remove_result}"
    );

    // ── 5. verify the credential is gone ────────────────────────────────────
    let list_after = post_json_rpc(
        &rpc_base,
        5005,
        "openhuman.auth_list_provider_credentials",
        json!({}),
    )
    .await;
    let after_outer =
        assert_no_jsonrpc_error(&list_after, "auth_list_provider_credentials (after remove)");
    let after_result = after_outer.get("result").unwrap_or(after_outer);
    let after_profiles = after_result
        .as_array()
        .unwrap_or_else(|| panic!("expected array after remove: {after_result}"));
    assert!(
        after_profiles.is_empty(),
        "credentials list should be empty after remove, got {after_profiles:?}"
    );

    mock_join.abort();
    rpc_join.abort();
}

/// End-to-end coverage for `openhuman.skills_uninstall`.
///
/// Validates that the RPC method is registered, wire-decodes
/// `UninstallSkillParams`, resolves the slug against
/// `~/.openhuman/skills/<slug>/`, removes the directory on success, and
/// forwards the core error message verbatim for the two documented
/// failure modes (missing SKILL.md and path traversal). Previously only
/// the `uninstall_skill(...)` helper was tested — the wire layer
/// (controller registration, param decoding, response shape) was not.

#[tokio::test]
async fn rpc_rejects_unauthenticated_request() {
    let _env_lock = json_rpc_e2e_env_lock_async().await;
    ensure_test_rpc_auth();

    let (rpc_addr, rpc_join) = serve_on_ephemeral(build_core_http_router(false)).await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("http://{rpc_addr}/rpc"))
        .header("Content-Type", "application/json")
        .body(r#"{"jsonrpc":"2.0","id":1,"method":"core.ping","params":{}}"#)
        .send()
        .await
        .expect("request");

    assert_eq!(resp.status(), 401, "missing Authorization must yield 401");
    let body: Value = resp.json().await.expect("json body");
    assert_eq!(
        body["error"], "unauthorized",
        "error field must be 'unauthorized'"
    );

    rpc_join.abort();
}

/// POST /rpc with a syntactically valid but wrong bearer token → 401.

#[tokio::test]
async fn rpc_rejects_wrong_token() {
    let _env_lock = json_rpc_e2e_env_lock_async().await;
    ensure_test_rpc_auth();

    let (rpc_addr, rpc_join) = serve_on_ephemeral(build_core_http_router(false)).await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("http://{rpc_addr}/rpc"))
        .header(
            AUTHORIZATION,
            "Bearer deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
        )
        .header("Content-Type", "application/json")
        .body(r#"{"jsonrpc":"2.0","id":1,"method":"core.ping","params":{}}"#)
        .send()
        .await
        .expect("request");

    assert_eq!(resp.status(), 401, "wrong token must yield 401");
    let body: Value = resp.json().await.expect("json body");
    assert_eq!(body["error"], "unauthorized");

    rpc_join.abort();
}

/// Every path in PUBLIC_PATHS must bypass the auth middleware — i.e. never
/// return 401 — even without an Authorization header.  Some paths return
/// non-2xx for other reasons (missing query params, no WebSocket upgrade
/// headers) so the assertion is `!= 401`, not `.is_success()`.

#[tokio::test]
async fn public_paths_accessible_without_token() {
    let _env_lock = json_rpc_e2e_env_lock_async().await;
    ensure_test_rpc_auth();

    let (rpc_addr, rpc_join) = serve_on_ephemeral(build_core_http_router(false)).await;
    let client = reqwest::Client::new();
    let base = format!("http://{rpc_addr}");

    // Paths that return 200 without any extra params.
    // `/events/webhooks` was REMOVED from this list when issue #1922 wired
    // bearer auth onto it (header or `?token=…`). Coverage for that path
    // lives in the dedicated `webhook_sse_*` tests below.
    for path in ["/", "/health", "/schema"] {
        let resp = client
            .get(format!("{base}{path}"))
            .send()
            .await
            .unwrap_or_else(|e| panic!("GET {path}: {e}"));
        assert!(
            resp.status().is_success(),
            "public path {path} must return 2xx without auth, got {}",
            resp.status()
        );
    }

    // Paths that bypass the *middleware* header check but return non-2xx for
    // unrelated reasons (missing required query params, no WebSocket upgrade
    // headers, etc.). The invariant here is that the auth middleware does NOT
    // reject them with 401.
    //
    // NOTE: `/events` enforces its own credential inside the handler, so an
    // unauthenticated GET returns 401 from the HANDLER (not the middleware).
    // It requires a `client_id` query param, so an empty request is rejected
    // by Axum's extractor (400) before the handler's own auth runs.
    for path in ["/events"] {
        let resp = client
            .get(format!("{base}{path}"))
            .send()
            .await
            .unwrap_or_else(|e| panic!("GET {path}: {e}"));
        assert_ne!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "public path {path} must not be auth-gated by the middleware (got {})",
            resp.status()
        );
    }

    // The removed dictation socket must fall through to the normal 404.
    let resp = client
        .get(format!("{base}/ws/dictation"))
        .send()
        .await
        .unwrap_or_else(|e| panic!("GET /ws/dictation: {e}"));
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "removed /ws/dictation endpoint must return 404 (got {})",
        resp.status()
    );

    rpc_join.abort();
}

// ---------------------------------------------------------------------------
// Webhook SSE auth (issue #1922) — /events/webhooks now requires bearer auth
// via either the Authorization header OR a `?token=…` query param. The
// query-param fallback exists because browser `EventSource` cannot attach
// custom headers (whatwg/html §10.7). See `QUERY_TOKEN_PATHS` in
// crates/openhuman-core/src/core/auth.rs.
// ---------------------------------------------------------------------------

/// GET /events/webhooks with neither header nor query token → 401.

#[tokio::test]
async fn webhook_sse_rejects_unauthenticated() {
    let _env_lock = json_rpc_e2e_env_lock_async().await;
    ensure_test_rpc_auth();

    let (rpc_addr, rpc_join) = serve_on_ephemeral(build_core_http_router(false)).await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("http://{rpc_addr}/events/webhooks"))
        .send()
        .await
        .expect("request");

    assert_eq!(
        resp.status(),
        401,
        "missing credentials on /events/webhooks must yield 401"
    );
    let body: Value = resp.json().await.expect("json body");
    assert_eq!(body["error"], "unauthorized");

    rpc_join.abort();
}

/// GET /events/webhooks?token= (empty value) → 401. Defends against
/// `encodeURIComponent(null)` / `encodeURIComponent("")` mishaps on the FE.

#[tokio::test]
async fn webhook_sse_rejects_empty_query_token() {
    let _env_lock = json_rpc_e2e_env_lock_async().await;
    ensure_test_rpc_auth();

    let (rpc_addr, rpc_join) = serve_on_ephemeral(build_core_http_router(false)).await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("http://{rpc_addr}/events/webhooks?token="))
        .send()
        .await
        .expect("request");

    assert_eq!(resp.status(), 401, "empty token value must be 401");

    rpc_join.abort();
}

/// GET /events/webhooks?token=garbage → 401.

#[tokio::test]
async fn webhook_sse_rejects_wrong_query_token() {
    let _env_lock = json_rpc_e2e_env_lock_async().await;
    ensure_test_rpc_auth();

    let (rpc_addr, rpc_join) = serve_on_ephemeral(build_core_http_router(false)).await;
    let client = reqwest::Client::new();

    let bad = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
    assert_ne!(bad, TEST_RPC_TOKEN);

    let resp = client
        .get(format!("http://{rpc_addr}/events/webhooks?token={bad}"))
        .send()
        .await
        .expect("request");

    assert_eq!(resp.status(), 401, "wrong query token must be 401");
    let body: Value = resp.json().await.expect("json body");
    assert_eq!(body["error"], "unauthorized");

    rpc_join.abort();
}

/// GET /events/webhooks?token=<valid> → 200 (SSE stream opens).

#[tokio::test]
async fn webhook_sse_accepts_valid_query_token() {
    let _env_lock = json_rpc_e2e_env_lock_async().await;
    ensure_test_rpc_auth();

    let (rpc_addr, rpc_join) = serve_on_ephemeral(build_core_http_router(false)).await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!(
            "http://{rpc_addr}/events/webhooks?token={TEST_RPC_TOKEN}"
        ))
        .send()
        .await
        .expect("request");

    assert_eq!(
        resp.status(),
        200,
        "valid query token must open the SSE stream"
    );
    let ct = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        ct.starts_with("text/event-stream"),
        "expected SSE content-type, got {ct}"
    );

    rpc_join.abort();
}

/// GET /events/webhooks with a percent-encoded query token → 200. Locks the
/// URL-decoding contract `EventSource` callers depend on (the FE uses
/// `encodeURIComponent`, which percent-encodes reserved characters even
/// when the token itself is hex-only).

#[tokio::test]
async fn webhook_sse_accepts_percent_encoded_query_token() {
    let _env_lock = json_rpc_e2e_env_lock_async().await;
    ensure_test_rpc_auth();

    let (rpc_addr, rpc_join) = serve_on_ephemeral(build_core_http_router(false)).await;
    let client = reqwest::Client::new();

    let encoded = urlencoding::encode(TEST_RPC_TOKEN);
    // Sanity: encoding the canonical hex test token must remain a non-empty
    // string. The encoder may pass-through (hex is URL-safe) — that's fine;
    // the test still proves the decode path doesn't double-decode or strip.
    assert!(!encoded.is_empty());

    let resp = client
        .get(format!("http://{rpc_addr}/events/webhooks?token={encoded}"))
        .send()
        .await
        .expect("request");

    assert_eq!(
        resp.status(),
        200,
        "url-encoded valid query token must open the SSE stream"
    );
    let ct = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        ct.starts_with("text/event-stream"),
        "expected SSE content-type, got {ct}"
    );

    rpc_join.abort();
}

/// GET /events/webhooks with `Authorization: Bearer <valid>` → 200.
/// CLI / non-browser callers should still be able to subscribe the header way.

#[tokio::test]
async fn webhook_sse_accepts_valid_bearer_header() {
    let _env_lock = json_rpc_e2e_env_lock_async().await;
    ensure_test_rpc_auth();

    let (rpc_addr, rpc_join) = serve_on_ephemeral(build_core_http_router(false)).await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("http://{rpc_addr}/events/webhooks"))
        .header(AUTHORIZATION, format!("Bearer {TEST_RPC_TOKEN}"))
        .send()
        .await
        .expect("request");

    assert_eq!(
        resp.status(),
        200,
        "valid Bearer header must open the SSE stream"
    );

    rpc_join.abort();
}

/// Simulate an external process using a guessed token — must be rejected.

#[tokio::test]
async fn external_process_with_guessed_token_is_rejected() {
    let _env_lock = json_rpc_e2e_env_lock_async().await;
    ensure_test_rpc_auth(); // server validates against TEST_RPC_TOKEN

    let (rpc_addr, rpc_join) = serve_on_ephemeral(build_core_http_router(false)).await;
    let client = reqwest::Client::new();

    // An attacker process trying a plausible-looking token that isn't the real one.
    let attacker_token = "aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899";
    assert_ne!(
        attacker_token, TEST_RPC_TOKEN,
        "attacker token must differ from real one"
    );

    let resp = client
        .post(format!("http://{rpc_addr}/rpc"))
        .header(AUTHORIZATION, format!("Bearer {attacker_token}"))
        .header("Content-Type", "application/json")
        .body(r#"{"jsonrpc":"2.0","id":1,"method":"core.ping","params":{}}"#)
        .send()
        .await
        .expect("request");

    assert_eq!(
        resp.status(),
        401,
        "external process with wrong token must be rejected"
    );

    rpc_join.abort();
}


#[tokio::test]
async fn rpc_update_apply_can_be_disabled_by_config_policy() {
    let _env_lock = json_rpc_e2e_env_lock_async().await;
    ensure_test_rpc_auth();

    let tmp = tempdir().expect("tempdir");
    let _workspace_guard = EnvVarGuard::set_to_path("OPENHUMAN_WORKSPACE", tmp.path());

    let mut config = openhuman_core::config::Config {
        workspace_dir: tmp.path().join("workspace"),
        action_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("config.toml"),
        ..openhuman_core::config::Config::default()
    };
    config.update.rpc_mutations_enabled = false;
    config
        .save()
        .await
        .expect("persist config with update rpc disabled");

    let (rpc_addr, rpc_join) = serve_on_ephemeral(build_core_http_router(false)).await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("http://{rpc_addr}/rpc"))
        .header(AUTHORIZATION, format!("Bearer {TEST_RPC_TOKEN}"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "openhuman.update_apply",
            "params": {
                "download_url": "https://github.com/owner/repo/releases/download/v1/x",
                "asset_name": "openhuman-core-x86_64-unknown-linux-gnu"
            }
        }))
        .send()
        .await
        .expect("request");

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("json body");
    let error_msg = body
        .get("result")
        .and_then(|outer| outer.get("result"))
        .and_then(|inner| inner.get("error"))
        .and_then(Value::as_str)
        .expect("policy error result message");
    assert!(
        error_msg.contains("rpc_mutations_enabled=false"),
        "unexpected error: {body}"
    );

    rpc_join.abort();
}

/// End-to-end coverage for issue #1149: storing a managed-DM channel
/// credential under `channel:<slug>:<mode>` and immediately observing
/// `connected:true` from `openhuman.channels_status`.
///
/// Before the fix, `channels_status` always returned `connected:false`
/// because the underlying `list_provider_credentials` call used an
/// exact-match filter (`provider == "channel:"`) that never matched
/// the real credential keys (`channel:telegram:managed_dm`,
/// `channel:slack:bot_token`, …). The user could connect Telegram in
/// the UI but the chat / Settings page would still report it
/// disconnected on the next reload.
///
/// This test exercises the full RPC wire path so a regression in
/// either the prefix helper or the channels controller is caught at
/// the transport layer, not just at the unit level.

#[tokio::test]
async fn channels_status_reflects_managed_dm_credential_e2e() {
    let _env_lock = json_rpc_e2e_env_lock_async().await;
    let tmp = tempdir().expect("tempdir");
    let home = tmp.path();
    let openhuman_home = home.join(".openhuman");

    let _home_guard = EnvVarGuard::set_to_path("HOME", home);
    let _workspace_guard = EnvVarGuard::unset("OPENHUMAN_WORKSPACE");
    let _backend_url_guard = EnvVarGuard::unset("BACKEND_URL");
    let _vite_backend_guard = EnvVarGuard::unset("VITE_BACKEND_URL");

    let (mock_addr, mock_join) = serve_on_ephemeral(mock_upstream_router()).await;
    let mock_origin = format!("http://{}", mock_addr);
    write_min_config(&openhuman_home, &mock_origin);

    let (rpc_addr, rpc_join) = serve_on_ephemeral(build_core_http_router(false)).await;
    let rpc_base = format!("http://{}", rpc_addr);
    tokio::time::sleep(Duration::from_millis(100)).await;

    // ── 1. baseline: telegram should report disconnected ────────────────────
    let baseline = post_json_rpc(
        &rpc_base,
        7001,
        "openhuman.channels_status",
        json!({ "channel": "telegram" }),
    )
    .await;
    let baseline_outer = assert_no_jsonrpc_error(&baseline, "channels_status (baseline)");
    let baseline_result = baseline_outer.get("result").unwrap_or(baseline_outer);
    let baseline_entries = baseline_result
        .as_array()
        .unwrap_or_else(|| panic!("expected array: {baseline_result}"));
    let baseline_managed = baseline_entries
        .iter()
        .find(|e| e.get("auth_mode").and_then(Value::as_str) == Some("managed_dm"))
        .expect("managed_dm entry should exist for telegram");
    assert_eq!(
        baseline_managed.get("connected").and_then(Value::as_bool),
        Some(false),
        "fresh config should report telegram managed_dm disconnected: {baseline_managed}"
    );

    // ── 2. simulate a successful managed-DM link by storing the credential
    //      marker the way `telegram_login_check` does in production ─────────
    let store = post_json_rpc(
        &rpc_base,
        7002,
        "openhuman.auth_store_provider_credentials",
        json!({
            "provider": "channel:telegram:managed_dm",
            "profile": "default",
            "token": "managed",
            "fields": { "linked": true },
            "setActive": true,
        }),
    )
    .await;
    assert_no_jsonrpc_error(&store, "auth_store_provider_credentials");

    // ── 3. channels_status must now report telegram managed_dm connected ─
    let after = post_json_rpc(
        &rpc_base,
        7003,
        "openhuman.channels_status",
        json!({ "channel": "telegram" }),
    )
    .await;
    let after_outer = assert_no_jsonrpc_error(&after, "channels_status (after link)");
    let after_result = after_outer.get("result").unwrap_or(after_outer);
    let after_entries = after_result
        .as_array()
        .unwrap_or_else(|| panic!("expected array: {after_result}"));
    let after_managed = after_entries
        .iter()
        .find(|e| e.get("auth_mode").and_then(Value::as_str) == Some("managed_dm"))
        .expect("managed_dm entry should exist for telegram");
    assert_eq!(
        after_managed.get("connected").and_then(Value::as_bool),
        Some(true),
        "managed-DM credential should surface as connected: {after_managed}"
    );
    assert_eq!(
        after_managed
            .get("has_credentials")
            .and_then(Value::as_bool),
        Some(true)
    );

    mock_join.abort();
    rpc_join.abort();
}

/// Regression guard for issue #1289: `openhuman.voice_cloud_transcribe`
/// must stay registered in the controller registry and reachable via
/// JSON-RPC dispatch.
///
/// The user-visible symptom was "Voice transcription failed: unknown
/// method: openhuman.voice_cloud_transcribe" — the frontend (mascot
/// mic-only composer) was calling a method that wasn't reachable.
/// This test pins both ends:
///
/// 1. `/schema` exposes `openhuman.voice_cloud_transcribe` so the
///    discovery surface stays in sync with the live registry.
/// 2. Calling the method over RPC does NOT hit the dispatcher's
///    unknown-method branch (`Err("unknown method: …")`). The call may
///    still fail downstream (missing audio, unauthenticated, missing
///    upstream STT key) — but it must reach the registered handler,
///    which proves the method is wired all the way through.

#[tokio::test]
async fn auth_store_provider_credentials_writes_an_owner_only_store_file() {
    use std::os::unix::fs::PermissionsExt as _;

    // Re-exec this one test under a known-permissive umask before doing
    // anything else.
    //
    // umask is process-global and libtest runs tests as threads, so it cannot
    // be set in-process without racing every other test in this binary. It has
    // to be controlled, though: a reverted `fs::write` creates at
    // `0o666 & ~umask`, which is 0644 under the usual 022 but ALSO 0600 under a
    // hardened 077 — and under 077 an owner-only assertion passes whether or not
    // the fix is present. Skipping in that case is not a way out either: libtest
    // records an early `return` as a PASS, so on a hardened runner this
    // security regression test would go green having checked nothing.
    if std::env::var(OWNER_ONLY_CHILD_ENV).is_err() {
        let exe = std::env::current_exe().expect("current_exe");
        // The exe path and the test name go in as POSITIONAL PARAMETERS rather
        // than interpolated into the command string. A repository or
        // CARGO_TARGET_DIR containing a single quote (`/home/o'connor/openhuman`)
        // would close the literal `'...'` early and hand `sh` invalid syntax, so
        // this test would fail before its child ever started — on exactly the
        // machines whose paths are least likely to be noticed. `$0` is the
        // conventional placeholder slot for `sh -c`, so the real arguments start
        // at `$1`.
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg("umask 022 && exec \"$1\" --exact \"$2\" --nocapture --test-threads=1")
            .arg("sh")
            .arg(&exe)
            .arg(OWNER_ONLY_TEST_NAME)
            .env(OWNER_ONLY_CHILD_ENV, "1")
            .status()
            .expect("re-exec the owner-only test under umask 022");
        assert!(
            status.success(),
            "the owner-only credential-store check failed under umask 022 (see child output above)"
        );
        return;
    }

    let _env_lock = json_rpc_e2e_env_lock_async().await;
    let tmp = tempdir().expect("tempdir");
    let home = tmp.path();
    let openhuman_home = home.join(".openhuman");

    let _home_guard = EnvVarGuard::set_to_path("HOME", home);
    let _workspace_guard = EnvVarGuard::unset("OPENHUMAN_WORKSPACE");
    let _backend_url_guard = EnvVarGuard::unset("BACKEND_URL");
    let _vite_backend_guard = EnvVarGuard::unset("VITE_BACKEND_URL");

    // Precondition, asserted rather than skipped: with umask 022 an ordinary
    // create must land group/world-readable. If it does not, the umask did not
    // take and the assertion below could not tell the fix from its absence —
    // that is a failure, not a pass.
    let probe = home.join("umask-probe");
    std::fs::write(&probe, b"probe").expect("write probe");
    let probe_mode = std::fs::metadata(&probe)
        .expect("probe metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_ne!(
        probe_mode & 0o077,
        0,
        "umask 022 should leave a plain create group-readable, got {probe_mode:#o}; \
         without that this test cannot distinguish the owner-only fix from its absence"
    );

    let (mock_addr, mock_join) = serve_on_ephemeral(mock_upstream_router()).await;
    let mock_origin = format!("http://{}", mock_addr);
    write_min_config(&openhuman_home, &mock_origin);

    let (rpc_addr, rpc_join) = serve_on_ephemeral(build_core_http_router(false)).await;
    let rpc_base = format!("http://{}", rpc_addr);
    tokio::time::sleep(Duration::from_millis(100)).await;

    let stored = post_json_rpc(
        &rpc_base,
        9_795,
        "openhuman.auth_store_provider_credentials",
        json!({
            "provider": "provider:openai",
            "profile": "default",
            "token": "sk-owner-only-e2e",
            "setActive": true
        }),
    )
    .await;
    assert_no_jsonrpc_error(&stored, "auth_store_provider_credentials");

    let store_path =
        find_auth_profiles_store(home).expect("the save must have produced an auth-profiles.json");
    let mode = std::fs::metadata(&store_path)
        .expect("store metadata")
        .permissions()
        .mode()
        & 0o777;

    assert_eq!(
        mode,
        0o600,
        "the credential store at {} must be owner-only, got {mode:#o} \
         (a plain create in this environment yields {probe_mode:#o})",
        store_path.display()
    );

    mock_join.abort();
    rpc_join.abort();
}

/// #5947 — `validate_only` is a dry run for **stt**, not only for tts.
///
/// It used to be checked inside the `"tts"` arm alone. Every provider the
/// settings modal can reach maps to `"stt"`, so the modal's dry-run request
/// fell through to a live transcription — the opposite of what "Test Key"
/// promises. A reserved slug is used because it is the one dry-run answer that
/// is fully determined with no network: there is no user-held key to check, so
/// the handler reports success without calling a provider.
