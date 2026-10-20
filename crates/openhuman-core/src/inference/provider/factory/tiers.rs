//! Generic callers of a registered provider's model vocabulary.

use super::*;

/// Whether the registered provider recognizes this model as an alias.
pub(super) fn is_abstract_tier_model(model: &str) -> bool {
    crate::inference::registry::provider_model_is_alias(PROVIDER_OPENHUMAN, model)
}

/// Resolve a model selection through the provider installed for this route.
pub fn resolve_model_for_hint(hint: &str, config: &Config) -> String {
    crate::inference::provider_model::resolve_model_hint(PROVIDER_OPENHUMAN, hint, config)
}

/// Workload role for an alias owned by the installed provider.
pub fn role_for_model_tier(model: &str) -> String {
    crate::inference::provider_model::role_for_model(PROVIDER_OPENHUMAN, model)
}

/// Model selected by the installed provider, if it supplied a default.
pub(crate) fn managed_default_model(config: &Config) -> String {
    crate::inference::registry::resolve_provider_model(PROVIDER_OPENHUMAN, "chat", config, None)
        .unwrap_or_default()
}

/// Whether this model is a concrete pass-through id.
pub(crate) fn is_raw_passthrough_model(model: &str) -> bool {
    crate::inference::provider_model::is_raw_model(PROVIDER_OPENHUMAN, model)
}

/// Whether the registered provider says this model accepts image input.
pub(crate) fn oh_tier_supports_vision(model: &str) -> bool {
    crate::inference::registry::provider_model_supports_vision(PROVIDER_OPENHUMAN, model)
}

/// Whether the model is an alias rather than a concrete provider model.
pub(crate) fn is_known_openhuman_tier(model: &str) -> bool {
    is_abstract_tier_model(model)
}

/// Concrete default model selected by the installed provider, if any.
pub(crate) fn pinned_managed_default_model(config: &Config) -> Option<String> {
    crate::inference::registry::resolve_provider_model(PROVIDER_OPENHUMAN, "chat", config, None)
}
