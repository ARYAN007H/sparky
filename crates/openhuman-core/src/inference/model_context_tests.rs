use super::*;
#[test]
fn tier_aliases_resolve() {
    assert_eq!(context_window_for_model("reasoning-v1"), Some(1_000_000));
    assert_eq!(context_window_for_model("agentic-v1"), Some(200_000));
    // chat-v1 is backed by DeepSeek v4 Flash (~1M), the same model as
    // summarization-v1 — not a 128K model (issue #4706).
    assert_eq!(context_window_for_model("chat-v1"), Some(1_000_000));
    // Burst tier — 128k on the managed backend. Matched on the alias, not
    // the local-gemma 8k substring arm.
    assert_eq!(context_window_for_model("burst-v1"), Some(128_000));
    // reasoning-quick-v1 is the legacy alias of chat-v1 (backend renamed it
    // 2026-05), so it resolves to the same ~1M flash window.
    assert_eq!(
        context_window_for_model("reasoning-quick-v1"),
        Some(1_000_000)
    );
    // summarization-v1 maps to a ~1M-token flash model so the extractor can
    // single-shot whole oversized payloads.
    assert_eq!(
        context_window_for_model("summarization-v1"),
        Some(1_000_000)
    );
    // The three flash-backed tiers share one window and must not drift.
    assert_eq!(
        context_window_for_model("chat-v1"),
        context_window_for_model("summarization-v1")
    );
}

#[test]
fn copilot_haiku_resolves_to_200k() {
    assert_eq!(
        context_window_for_model("github_copilot/claude-haiku-4.5"),
        Some(200_000)
    );
}

#[test]
fn unknown_model_returns_none() {
    assert_eq!(context_window_for_model("totally-unknown-model-xyz"), None);
}

#[test]
fn empty_model_returns_none() {
    assert_eq!(context_window_for_model("   "), None);
}

#[test]
fn model_vision_enabled_reads_registry_only() {
    use crate::config::schema::ModelRegistryEntry;
    let mut config = crate::config::Config::default();
    config.model_registry = vec![
        ModelRegistryEntry {
            id: "my-llava".into(),
            provider: "openai".into(),
            cost_per_1m_output: 0.0,
            vision: true,
            ..Default::default()
        },
        ModelRegistryEntry {
            id: "text-only".into(),
            provider: "openai".into(),
            cost_per_1m_output: 0.0,
            vision: false,
            ..Default::default()
        },
    ];
    assert!(model_vision_enabled("my-llava", &config));
    assert!(!model_vision_enabled("text-only", &config));
    assert!(!model_vision_enabled("unlisted", &config));
    assert!(!model_vision_enabled("   ", &config));
}

#[test]
fn model_supports_vision_combines_tier_map_and_registry() {
    use crate::config::schema::ModelRegistryEntry;
    let mut config = crate::config::Config::default();
    config.model_registry = vec![ModelRegistryEntry {
        id: "my-llava".into(),
        provider: "openai".into(),
        cost_per_1m_output: 0.0,
        vision: true,
        ..Default::default()
    }];
    // BYOK model flagged in the registry is vision-capable.
    assert!(model_supports_vision("my-llava", &config));
    // Unlisted custom model is not.
    assert!(!model_supports_vision("gpt-5", &config));
}

#[test]
fn o1_o3_segment_match_does_not_overmatch() {
    // Real OpenAI o1/o3 model ids must still resolve.
    assert_eq!(context_window_for_model("o1"), Some(200_000));
    assert_eq!(context_window_for_model("o1-mini"), Some(200_000));
    assert_eq!(context_window_for_model("o3-mini"), Some(200_000));
    assert_eq!(context_window_for_model("openai/o1-preview"), Some(200_000));

    // Names that merely *contain* the substring "o1" / "o3" must NOT
    // inherit the 200K window (regression guard for PR #2100 review).
    assert_eq!(context_window_for_model("solo1-7b"), None);
    assert_eq!(context_window_for_model("proto3-chat"), None);
    assert_eq!(
        context_window_for_model("ollama/mistral-for-o1-benchmark"),
        Some(200_000),
        "`-o1-` segment should still match"
    );
    assert_eq!(context_window_for_model("octo3thing"), None);
}

#[test]
fn managed_vision_capability_is_absent_without_provider_policy() {
    let config = crate::config::Config::default();
    assert!(!model_supports_vision("hint:reasoning", &config));
    assert!(!model_supports_vision(
        "openrouter/qwen/qwen3.5-flash-02-23",
        &config
    ));
}

#[test]
fn model_supports_vision_uses_the_scoped_provider_capability_hook() {
    use crate::core::slots::Extensions;
    use crate::inference::registry::{register_chat_model_in, ChatModelEntry, ProviderModelHooks};
    use std::sync::Arc;

    let extensions = Extensions::new();
    register_chat_model_in(
        &extensions,
        crate::inference::provider::factory::PROVIDER_OPENHUMAN,
        ChatModelEntry {
            build: Arc::new(|_| Err(anyhow::anyhow!("test factory"))),
            probe: None,
            models: Some(ProviderModelHooks {
                default_model: Arc::new(|| None),
                role_for_model: Arc::new(|_| None),
                is_alias: Arc::new(|_| false),
                is_raw_model: Arc::new(|_| true),
                supports_vision: Arc::new(|model| model == "host/vision"),
                resolve_model: Arc::new(|_, _, _| None),
                context_window: Arc::new(|_| None),
                default_pricing: Arc::new(|| None),
                without_reasoning: Arc::new(|request| request),
            }),
        },
    );
    let context = crate::core::runtime::CoreContext::for_test_with_extensions(
        crate::core::runtime::DomainSet::full(),
        None,
        extensions,
    );
    let config = crate::config::Config::default();

    assert!(!model_supports_vision("host/vision", &config));
    crate::core::runtime::CoreContext::sync_scope(context, || {
        assert!(model_supports_vision("host/vision", &config));
        assert!(!model_supports_vision("host/text", &config));
    });
    assert!(!model_supports_vision("host/vision", &config));
}
