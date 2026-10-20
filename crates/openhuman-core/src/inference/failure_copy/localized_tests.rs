use super::*;

#[test]
fn native_failure_copy_uses_supported_locale_and_interpolates_retry_delay() {
    let rendered = render_channel_failure(
        "zh-CN",
        "Your AI provider is rate-limiting requests. Retry in 12 seconds.",
        Some("chat_error.rate_limited"),
        Some(&serde_json::json!({ "retry_after_secs": 12 })),
    );

    assert!(rendered.contains("你的 AI 提供商"));
    assert!(rendered.contains("12"));
    assert!(!rendered.contains("Your AI provider"));
}

#[test]
fn native_failure_copy_preserves_queued_prefix_and_sanitized_detail() {
    let rendered = render_channel_failure(
        "fr",
        "queued turn was not run: authentication failed\n\n> sanitized provider detail",
        Some("chat_error.auth_error"),
        Some(&serde_json::json!({ "detail": "sanitized provider detail" })),
    );

    assert!(rendered.starts_with("queued turn was not run: "));
    assert!(rendered.contains("problème d'authentification"));
    assert!(rendered.ends_with("> sanitized provider detail"));
    assert!(!rendered.contains("authentication failed"));
}

#[test]
fn unknown_locale_key_and_parameter_types_fall_back_safely() {
    assert_eq!(supported_channel_locale(None), None);
    assert_eq!(supported_channel_locale(Some(" xx ")), None);
    assert_eq!(supported_channel_locale(Some(" zh-CN ")), Some("zh-CN"));

    let english = render_channel_failure(
        "xx",
        "safe English fallback",
        Some("chat_error.rate_limited"),
        Some(&serde_json::json!({ "retry_after_secs": "not-a-number", "detail": 4 })),
    );
    assert_eq!(
        english,
        chat_error_catalog::translation("en", "chat_error.rate_limited").unwrap()
    );
    assert!(!english.contains("not-a-number"));
    assert!(!english.contains('4'));

    let unknown = render_channel_failure(
        "fr",
        "safe English fallback",
        Some("chat_error.from_a_newer_core"),
        None,
    );
    assert_eq!(unknown, "safe English fallback");
}
