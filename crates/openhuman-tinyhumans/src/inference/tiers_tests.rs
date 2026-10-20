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
