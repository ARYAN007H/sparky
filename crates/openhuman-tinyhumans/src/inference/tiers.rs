//! TinyHumans-managed model aliases, defaults, and capability metadata.

use openhuman_embed::__host::config::Config;

pub(crate) const MODEL_MANAGED_DEFAULT: &str = "openrouter/deepseek/deepseek-v4-flash";
pub(crate) const MODEL_MEDIA_UNDERSTANDING: &str = "openrouter/qwen/qwen3.5-flash-02-23";
pub(crate) const MODEL_IMAGE_GENERATION_AGENT: &str = "openrouter/qwen/qwen3.5-flash-02-23";
pub(crate) const MODEL_VIDEO_GENERATION_AGENT: &str = "openrouter/qwen/qwen3.5-flash-02-23";

const MANAGED_MULTIMODAL_MODELS: [&str; 3] = [
    MODEL_MEDIA_UNDERSTANDING,
    MODEL_IMAGE_GENERATION_AGENT,
    MODEL_VIDEO_GENERATION_AGENT,
];

pub(crate) fn legacy_tier_role(model: &str) -> Option<&'static str> {
    match model.trim() {
        "chat-v1" | "reasoning-quick-v1" => Some("chat"),
        "reasoning-v1" => Some("reasoning"),
        "agentic-v1" => Some("agentic"),
        "coding-v1" => Some("coding"),
        "burst-v1" => Some("burst"),
        "summarization-v1" => Some("summarization"),
        "vision-v1" => Some("vision"),
        _ => None,
    }
}

fn hint_role(model: &str) -> Option<&'static str> {
    match model.trim().strip_prefix("hint:")? {
        "chat" => Some("chat"),
        "reasoning" => Some("reasoning"),
        "agentic" => Some("agentic"),
        "coding" => Some("coding"),
        "burst" => Some("burst"),
        "summarization" => Some("summarization"),
        "vision" => Some("vision"),
        _ => None,
    }
}

pub(crate) fn role_for_model(model: &str) -> Option<String> {
    hint_role(model)
        .or_else(|| legacy_tier_role(model))
        .map(str::to_string)
}

pub(crate) fn is_alias(model: &str) -> bool {
    hint_role(model).is_some() || legacy_tier_role(model).is_some()
}

pub(crate) fn is_raw_model(model: &str) -> bool {
    let model = model.trim();
    !model.is_empty() && !model.starts_with("hint:") && !is_alias(model)
}

pub(crate) fn default_model() -> Option<String> {
    Some(MODEL_MANAGED_DEFAULT.to_string())
}

pub(crate) fn resolve_model(
    _role: &str,
    config: &Config,
    model_override: Option<&str>,
) -> Option<String> {
    let configured = config
        .default_model
        .as_deref()
        .map(str::trim)
        .filter(|model| !model.is_empty() && !is_alias(model))
        .unwrap_or(MODEL_MANAGED_DEFAULT)
        .to_string();
    Some(match model_override.map(str::trim).filter(|model| !model.is_empty()) {
        Some(model) if is_alias(model) => configured,
        Some(model) => model.to_string(),
        None => configured,
    })
}

pub(crate) fn supports_vision(model: &str) -> bool {
    let model = model.trim();
    model == MODEL_MANAGED_DEFAULT
        || MANAGED_MULTIMODAL_MODELS.contains(&model)
        || matches!(role_for_model(model).as_deref(), Some("vision" | "reasoning"))
}

pub(crate) fn context_window(model: &str) -> Option<u64> {
    let model = model.trim();
    if model == MODEL_MANAGED_DEFAULT || matches!(role_for_model(model).as_deref(), Some("chat" | "summarization")) {
        return Some(1_000_000);
    }
    match role_for_model(model).as_deref() {
        Some("reasoning") => Some(1_000_000),
        Some("agentic" | "coding") => Some(200_000),
        Some("burst") => Some(128_000),
        _ => None,
    }
}

pub(crate) fn default_pricing() -> Option<(String, f64, f64)> {
    Some((MODEL_MANAGED_DEFAULT.to_string(), 0.0886, 0.1772))
}

pub(crate) fn normalize_config(config: &mut Config) -> bool {
    let mut changed = false;
    let rewrite = |value: &mut String| {
        if legacy_tier_role(value).is_some() {
            *value = MODEL_MANAGED_DEFAULT.to_string();
            true
        } else {
            false
        }
    };
    if let Some(model) = config.default_model.as_mut() {
        changed |= rewrite(model);
    }
    if let Some(model) = config.orchestrator.model.as_mut() {
        changed |= rewrite(model);
    }
    for team in config.teams.values_mut() {
        if let Some(model) = team.lead_model.as_mut() {
            changed |= rewrite(model);
        }
        if let Some(model) = team.agent_model.as_mut() {
            changed |= rewrite(model);
        }
    }
    for delegate in config.agents.values_mut() {
        changed |= rewrite(&mut delegate.model);
    }
    for route in &mut config.model_routes {
        changed |= rewrite(&mut route.model);
    }
    if config.cloud_providers.is_empty() {
        if let Some(preset) = openhuman_embed::__host::config::schema::cloud_providers::builtin_cloud_provider("openhuman") {
            let id = openhuman_embed::__host::config::schema::cloud_providers::generate_provider_id("openhuman");
            config.cloud_providers.push(openhuman_embed::__host::config::schema::cloud_providers::CloudProviderCreds {
                id: id.clone(),
                slug: preset.slug.to_string(),
                label: preset.label.to_string(),
                endpoint: config.api_url.as_deref().filter(|url| !url.trim().is_empty()).unwrap_or(preset.endpoint).to_string(),
                auth_style: preset.auth_style,
                default_model: config.default_model.clone(),
            });
            if config.primary_cloud.is_none() && config.inference_url.as_deref().is_none_or(|url| url.trim().is_empty()) {
                config.primary_cloud = Some(id);
            }
            changed = true;
        }
    }
    changed
}

#[cfg(test)]
#[path = "tiers_tests.rs"]
mod tests;
