use super::*;

#[test]
fn absent_provider_keeps_default_absent_and_concrete_model_ids_verbatim() {
    assert_eq!(
        crate::inference::registry::provider_default_model("missing-provider"),
        None
    );
    let config = Config::default();
    assert_eq!(
        resolve_provider_model_hint("missing-provider", "vendor/model-id", &config),
        "vendor/model-id"
    );
}

#[test]
fn abstract_hint_markers_do_not_become_raw_model_ids() {
    assert!(is_abstract_model("missing-provider", "hint:unknown"));
    assert!(!is_raw_model("missing-provider", "hint:unknown"));
    assert!(is_raw_model("missing-provider", "vendor/model-id"));
    assert_eq!(role_for_provider_model("missing-provider", "hint:coding"), "coding");
    assert_eq!(role_for_provider_model("missing-provider", "hint:unknown"), "chat");
}

#[test]
fn registered_model_defaults_and_aliases_are_context_scoped() {
    use crate::core::slots::Extensions;
    use crate::inference::registry::{
        register_chat_model_in, ChatModelEntry, ProviderModelHooks,
    };
    use std::sync::Arc;

    let extensions = Extensions::new();
    register_chat_model_in(
        &extensions,
        "scoped-provider",
        ChatModelEntry {
            build: Arc::new(|_| Err(anyhow::anyhow!("test factory"))),
            probe: None,
            models: Some(ProviderModelHooks {
                default_model: Arc::new(|| Some("scoped/model".to_string())),
                role_for_model: Arc::new(|model| {
                    (model == "scoped-alias").then(|| "reasoning".to_string())
                }),
                is_alias: Arc::new(|model| model == "scoped-alias"),
                is_raw_model: Arc::new(|model| model == "scoped/model"),
                supports_vision: Arc::new(|_| false),
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

    assert_eq!(
        crate::inference::registry::provider_default_model("scoped-provider"),
        None
    );
    crate::core::runtime::CoreContext::sync_scope(context, || {
        assert_eq!(
            crate::inference::registry::provider_default_model("scoped-provider").as_deref(),
            Some("scoped/model")
        );
        assert_eq!(
            crate::inference::registry::provider_model_role("scoped-provider", "scoped-alias")
                .as_deref(),
            Some("reasoning")
        );
    });
    assert_eq!(
        crate::inference::registry::provider_default_model("scoped-provider"),
        None
    );
}
