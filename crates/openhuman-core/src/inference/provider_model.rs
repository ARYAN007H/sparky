//! Provider-neutral helpers for resolving model ids against registered model
//! vocabularies.

use crate::config::{Config, WORKLOAD_ROLES};

/// The workload role represented by a provider-specific model alias, defaulting
/// to the generic `chat` role for concrete and unknown ids.
pub fn role_for_model(provider_id: &str, model: &str) -> String {
    crate::inference::registry::provider_model_role(provider_id, model)
        .unwrap_or_else(|| "chat".to_string())
}

/// Generic workload role for a provider-owned model alias.
pub fn role_for_provider_model(provider_id: &str, model: &str) -> &'static str {
    let role = model
        .strip_prefix("hint:")
        .filter(|role| WORKLOAD_ROLES.contains(role))
        .map(str::to_string)
        .or_else(|| crate::inference::registry::provider_model_role(provider_id, model));
    role.as_deref()
        .and_then(|role| WORKLOAD_ROLES.iter().copied().find(|known| *known == role))
        .unwrap_or("chat")
}

/// Whether `model` is a provider-owned alias or a concrete model id.
pub fn is_raw_model(provider_id: &str, model: &str) -> bool {
    crate::inference::registry::provider_model_is_raw(provider_id, model)
}

/// `hint:*` is an abstract routing marker even if no installed provider
/// recognizes the suffix; never pass it to a concrete BYOK endpoint.
pub fn is_abstract_model(provider_id: &str, model: &str) -> bool {
    model.trim().starts_with("hint:")
        || crate::inference::registry::provider_model_is_alias(provider_id, model)
}

/// Resolve a model hint according to the configured provider route and the
/// route's registered provider model policy.
pub fn resolve_provider_model_hint(provider_id: &str, hint: &str, config: &Config) -> String {
    let trimmed = hint.trim();
    if trimmed.is_empty() {
        return trimmed.to_string();
    }
    let Some(role) = crate::inference::registry::provider_model_role(provider_id, trimmed) else {
        return trimmed.to_string();
    };

    let route = crate::inference::provider::factory::provider_for_role(&role, config);
    let route = route.trim();
    if route.is_empty()
        || route == "cloud"
        || route == provider_id
        || route == crate::inference::provider::factory::BYOK_INCOMPLETE_SENTINEL
    {
        return crate::inference::registry::resolve_provider_model(
            provider_id,
            &role,
            config,
            Some(trimmed),
        )
        .unwrap_or_else(|| trimmed.to_string());
    }
    if let Some((_, model)) = route.split_once(':') {
        return tinyinference_providers::route::split_model_and_temperature(model).0;
    }
    route.to_string()
}
