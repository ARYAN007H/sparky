use super::*;

#[test]
fn hosted_normalizer_rewrites_legacy_tiers_and_preserves_concrete_pins() {
    let mut config = Config::default();
    config.default_model = Some("reasoning-v1".to_string());
    config.orchestrator.model = Some("custom/orchestrator-model".to_string());
    config.agents.insert(
        "pinned".to_string(),
        openhuman_embed::__host::config::schema::DelegateAgentConfig {
            model: "vendor/model-id".to_string(),
            system_prompt: None,
            temperature: None,
            max_depth: 3,
        },
    );

    assert!(normalize_config(&mut config));
    assert_eq!(config.default_model.as_deref(), Some(MODEL_MANAGED_DEFAULT));
    assert_eq!(
        config.orchestrator.model.as_deref(),
        Some("custom/orchestrator-model")
    );
    assert_eq!(config.agents["pinned"].model, "vendor/model-id");
    assert!(!normalize_config(&mut config));
}

#[test]
fn managed_tier_vocabulary_maps_aliases_and_leaves_concrete_ids_raw() {
    for (model, role) in [
        ("reasoning-v1", "reasoning"),
        ("chat-v1", "chat"),
        ("agentic-v1", "agentic"),
        ("burst-v1", "burst"),
        ("coding-v1", "coding"),
        ("vision-v1", "vision"),
        ("summarization-v1", "summarization"),
        ("reasoning-quick-v1", "chat"),
        ("hint:reasoning", "reasoning"),
        ("hint:coding", "coding"),
    ] {
        assert_eq!(role_for_model(model).as_deref(), Some(role));
        assert!(is_alias(model));
    }
    assert!(is_raw_model("vendor/concrete-model"));
    assert!(!is_raw_model("hint:unknown"));
}
