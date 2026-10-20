use super::*;
use tinyinference_llm::model::ChatModel;

#[test]
fn managed_model_preserves_the_openhuman_response_cache_identity() {
    let model = build_chat_model(
        Some("https://api.example.test"),
        &ManagedOptions::default(),
        "reasoning-v1",
        true,
        None,
        false,
    )
    .expect("managed owner model");

    assert_eq!(
        model.cache_identity().as_deref(),
        Some("openhuman:https://api.example.test/openai/v1:reasoning-v1")
    );
}

#[test]
fn managed_model_trims_a_configured_model_and_uses_the_product_default_when_blank() {
    let configured = build_chat_model(
        Some("https://api.example.test"),
        &ManagedOptions::default(),
        "  reasoning-v1  ",
        true,
        None,
        false,
    )
    .expect("configured model");
    assert_eq!(
        configured.cache_identity().as_deref(),
        Some("openhuman:https://api.example.test/openai/v1:reasoning-v1")
    );

    let blank = build_chat_model(
        Some("https://api.example.test"),
        &ManagedOptions::default(),
        "  ",
        true,
        None,
        false,
    )
    .expect("blank model falls back to the product default");
    assert_eq!(
        blank.cache_identity().as_deref(),
        Some("openhuman:https://api.example.test/openai/v1:openrouter/deepseek/deepseek-v4-flash")
    );
}

#[test]
fn managed_bearer_keeps_the_stored_key_transport_guard() {
    let tmp = tempfile::TempDir::new().expect("temporary credentials");
    openhuman_embed::__host::security::credentials::api_key::store_api_key_in(
        tmp.path(),
        false,
        "test-managed-key",
    )
    .expect("store test API key");
    let options = ManagedOptions {
        openhuman_dir: Some(tmp.path().to_path_buf()),
        secrets_encrypt: false,
        ..ManagedOptions::default()
    };

    assert_eq!(
        options
            .resolve_bearer("https://api.tinyhumans.ai/openai/v1")
            .expect("managed TLS endpoint accepts the API key"),
        "test-managed-key"
    );
    assert_eq!(
        options
            .resolve_bearer("http://127.0.0.1:9999/openai/v1")
            .expect("loopback is a valid local test endpoint"),
        "test-managed-key"
    );
    let error = options
        .resolve_bearer("https://api.example.test/openai/v1")
        .expect_err("a foreign HTTPS origin must not receive the stored key");
    assert!(error.to_string().contains("unmanaged or insecure"));
}
