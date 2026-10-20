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
