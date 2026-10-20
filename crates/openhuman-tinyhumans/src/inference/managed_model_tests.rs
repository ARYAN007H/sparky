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
        Some("openhuman:https://api.example.test/openai/v1:openrouter/deepseek/deepseek-v4-flash")
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
        Some("openhuman:https://api.example.test/openai/v1:reasoning-v1")
    );
}
