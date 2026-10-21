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
    assert!(is_raw_model("missing-provider", "chat"));
    assert!(is_raw_model("missing-provider", "coding"));
    assert_eq!(
        role_for_provider_model("missing-provider", "hint:coding"),
        "coding"
    );
    assert_eq!(
        role_for_provider_model("missing-provider", "hint:unknown"),
        "chat"
    );
}

#[test]
fn registered_model_defaults_and_aliases_are_context_scoped() {
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
                default_model: Arc::new(|| Some("scoped/model".to_string())),
                role_for_model: Arc::new(|model| {
                    (model == "scoped-alias").then(|| "reasoning".to_string())
                }),
                is_alias: Arc::new(|model| model == "scoped-alias"),
                is_raw_model: Arc::new(|model| model == "scoped/model"),
                supports_vision: Arc::new(|_| false),
                resolve_model: Arc::new(|_, _, _| None),
                context_window: Arc::new(|model| match model {
                    "scoped/model" => Some(77_777),
                    "hint:reasoning" => Some(88_888),
                    _ => None,
                }),
                default_pricing: Arc::new(|| None),
                without_reasoning: Arc::new(|request| request),
            }),
        },
    );
    assert_eq!(
        crate::inference::registry::provider_default_model_for(
            Some(&extensions),
            crate::inference::provider::factory::PROVIDER_OPENHUMAN
        )
        .as_deref(),
        Some("scoped/model")
    );
    assert_eq!(
        crate::inference::context_window::project_context_window_for_model(
            "scoped/model",
            Some(Some(&extensions))
        )
        .window,
        Some(77_777)
    );
    assert_eq!(
        crate::inference::context_window::project_context_window_for_model(
            "hint:reasoning",
            Some(Some(&extensions))
        )
        .window,
        Some(88_888),
        "a registered provider hook can override a workload tier alias"
    );
    let context = crate::core::runtime::CoreContext::for_test_with_extensions(
        crate::core::runtime::DomainSet::full(),
        None,
        extensions,
    );

    assert_eq!(
        crate::inference::registry::provider_default_model(
            crate::inference::provider::factory::PROVIDER_OPENHUMAN
        ),
        None
    );
    crate::core::runtime::CoreContext::sync_scope(context, || {
        assert_eq!(
            crate::inference::context_window::project_context_window_for_model(
                "scoped/model",
                None
            )
            .window,
            Some(77_777),
            "the ambient projection uses the active provider registry"
        );
        assert_ne!(
            crate::inference::context_window::project_context_window_for_model(
                "scoped/model",
                Some(None)
            )
            .window,
            Some(77_777),
            "an explicit process-only projection does not borrow ambient context"
        );
        assert_eq!(
            crate::inference::registry::provider_default_model(
                crate::inference::provider::factory::PROVIDER_OPENHUMAN
            )
            .as_deref(),
            Some("scoped/model")
        );
        assert_eq!(
            crate::inference::registry::provider_model_role(
                crate::inference::provider::factory::PROVIDER_OPENHUMAN,
                "scoped-alias"
            )
            .as_deref(),
            Some("reasoning")
        );
    });
    assert_eq!(
        crate::inference::registry::provider_default_model(
            crate::inference::provider::factory::PROVIDER_OPENHUMAN
        ),
        None
    );
}
