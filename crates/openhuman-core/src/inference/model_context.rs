//! Known model context-window sizes for pre-inference budgeting.
//!
//! Provider `/models` responses may include `context_length` / `context_window`,
//! but the agent harness must enforce limits **before** the first dispatch —
//! otherwise long histories produce upstream `400 Bad Request` errors when usage
//! metadata is not yet available.

/// Resolve the context window (in tokens) for a model id or registered provider alias.
///
/// Returns the static model estimate. Call [`context_window_for_route`] when
/// the provider route is known; provider-discovered values are stored per
/// route and model. Thread usage prefers its persisted per-turn value before
/// consulting either lookup.
///
/// Returns `None` when the model is unknown — callers should skip pre-dispatch
/// trimming rather than guess.
pub fn context_window_for_model(model: &str) -> Option<u64> {
    static_context_window_for_model(model)
}

/// Resolve the synchronous window for a model on a specific provider route.
pub(crate) fn context_window_for_route(
    provider: &str,
    model: &str,
    config: &crate::config::Config,
) -> Option<u64> {
    crate::inference::context_window::config_override(model, config)
        .or_else(|| crate::inference::context_window::remembered_window(provider, model))
        .or_else(|| static_context_window_for_model(model))
}

/// The static guess for a model's window: provider metadata, then the cost catalog,
/// then the generic id-pattern hints. These never override a provider-reported
/// window; [`crate::inference::context_window`] consults them last.
pub(crate) fn static_context_window_for_model(model: &str) -> Option<u64> {
    let normalized = model.trim();
    if normalized.is_empty() {
        return None;
    }

    if let Some(window) = crate::inference::registry::provider_model_context_window(
        crate::inference::provider::factory::PROVIDER_OPENHUMAN,
        normalized,
    ) {
        return Some(window);
    }

    // Keep conservative estimates for generic small-model ids.
    if normalized.starts_with("gemma") || normalized.contains(":1b") || normalized.contains("270m")
    {
        return Some(8_192);
    }

    if let Some(price) = tinyagents_registry::pricing::lookup(normalized) {
        tracing::debug!(
            model = normalized,
            catalog_model = price.model_id,
            context_window = price.context_window,
            "[model_context] matched cost catalog row"
        );
        return Some(u64::from(price.context_window));
    }

    if let Some(window) = tinyinference_llm::model::context_window_for_model_id(normalized) {
        tracing::debug!(
            model = normalized,
            context_window = window,
            "[model_context] matched tinyagents model context hint"
        );
        return Some(window);
    }

    None
}

/// Whether the model resolved for a chat hint/agent/profile accepts image input
/// according to the **user-configured** vision flag in `config.model_registry`.
///
/// This is the per-model override that lets a user mark a **custom / BYOK** model
/// as vision-capable (Settings → Advanced LLM → custom model → "Supports
/// vision"). Managed-backend models already advertise vision via
/// [`crate::inference::provider::Provider::supports_vision`]; this flag
/// covers OpenAI-compatible providers the backend can't introspect per-model.
/// Returns `false` for models the user has not flagged.
pub fn model_vision_enabled(model: &str, config: &crate::config::Config) -> bool {
    let normalized = model.trim();
    if normalized.is_empty() {
        return false;
    }
    let enabled = config
        .model_registry
        .iter()
        .any(|entry| entry.id == normalized && entry.vision);
    tracing::debug!(
        model = normalized,
        vision_enabled = enabled,
        "[model_context] resolved user-configured vision flag"
    );
    enabled
}

/// Whether a resolved model accepts image input. The single predicate shared by
/// the chat UI resolve and the server-side session/sub-agent gates.
///
/// - **Hosted models and role aliases** consult the registered provider's
///   capability hook; the backend does not advertise per-model capability.
/// - **Custom/BYOK models** consult the user-set `model_registry.vision` flag
///   ([`model_vision_enabled`]).
pub fn model_supports_vision(model: &str, config: &crate::config::Config) -> bool {
    crate::inference::registry::provider_model_supports_vision(
        crate::inference::provider::factory::PROVIDER_OPENHUMAN,
        model,
    ) || model_vision_enabled(model, config)
}

#[cfg(test)]
#[path = "model_context_tests.rs"]
mod tests;
