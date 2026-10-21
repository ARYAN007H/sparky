//! Registered inference providers, keyed by provider id.
//!
//! Three tables an outside crate fills to supply a provider the core does not
//! build itself, each consulted **before** the built-in construction:
//!
//! | Table | Consulted by | Registered value |
//! | --- | --- | --- |
//! | chat models | `provider::factory` (every managed-provider construction) | [`ChatModelEntry`] |
//! | embeddings | `embedding_host::factory` (the managed embedding model) | [`EmbeddingFactory`] |
//! | model catalogs | `provider::models::catalog_listing`, `context_window::model_limits_request_for` | [`CatalogEntry`] |
//!
//! The id is the provider id the core already routes on (the managed provider
//! is [`PROVIDER_OPENHUMAN`](crate::inference::provider::factory::PROVIDER_OPENHUMAN)).
//! The tables know nothing about what a provider talks to. With nothing
//! registered every consult point behaves exactly as before.

use std::sync::Arc;


use crate::config::Config;
use crate::core::slots::{Extensions, HookSlot};
use crate::security::approval::ApprovalChatContext;
use crate::security::egress::EgressLedger;

// ---------------------------------------------------------------------------
// Chat models
// ---------------------------------------------------------------------------

/// Host state supplied to a native TinyInference provider callback.
#[derive(Clone)]
pub struct ProviderContext {
    /// Configuration selected by the host for this provider call.
    pub config: Config,
    /// Egress ledger selected by the host for this provider call.
    pub egress: EgressLedger,
    /// Credential and disclosure data for an embedding factory call.
    pub embedding: Option<EmbeddingCallContext>,
    /// Request data and log sink for a catalog listing call.
    pub catalog: Option<CatalogCallContext>,
}

#[derive(Clone)]
pub struct EmbeddingCallContext {
    pub config: Option<Config>,
    pub chat: Option<ApprovalChatContext>,
    pub state_dir: std::path::PathBuf,
    pub secrets_encrypt: bool,
}

#[derive(Clone)]
pub struct CatalogCallContext {
    pub request: CatalogHostRequest,
    pub logs: Arc<std::sync::Mutex<Vec<String>>>,
}

#[derive(Clone)]
pub struct CatalogHostRequest {
    pub provider_key: String,
    pub client: reqwest::Client,
}

impl ProviderContext {
    pub(crate) fn new(config: &Config, egress: &EgressLedger) -> Self {
        Self { config: config.clone(), egress: egress.clone(), embedding: None, catalog: None }
    }

    pub(crate) fn for_embedding(
        config: Option<&Config>,
        chat: Option<ApprovalChatContext>,
        state_dir: std::path::PathBuf,
        secrets_encrypt: bool,
    ) -> Self {
        let base = config.cloned().unwrap_or_default();
        Self {
            config: base,
            egress: EgressLedger::none(),
            embedding: Some(EmbeddingCallContext { config: config.cloned(), chat, state_dir, secrets_encrypt }),
            catalog: None,
        }
    }

    pub(crate) fn for_catalog(config: &Config, request: CatalogHostRequest) -> Self {
        Self {
            config: config.clone(),
            egress: EgressLedger::none(),
            embedding: None,
            catalog: Some(CatalogCallContext { request, logs: Arc::new(std::sync::Mutex::new(Vec::new())) }),
        }
    }
}

pub use tinyinference_llm::providers::overrides::{
    BoxFuture, CatalogEntry, CatalogLimits, CatalogRequest, ChatModelEntry, ChatModelFactory,
    ChatModelRequest, EmbeddingFactory, EmbeddingRequest, ProviderModelMetadata,
    ProviderOverrides, ReadinessProbe,
};

pub(crate) fn provider_overrides(extensions: Option<Extensions>) -> ProviderOverrides<ProviderContext> {
    let chat_extensions = extensions.clone();
    let embedding_extensions = extensions.clone();
    ProviderOverrides::new(
        move |provider_id| CHAT_MODELS.get_for(chat_extensions.as_ref(), provider_id),
        move |provider_id| EMBEDDINGS.get_for(embedding_extensions.as_ref(), provider_id),
        move |provider_id| CATALOGS.get_for(extensions.as_ref(), provider_id),
    )
}

pub(crate) fn ambient_provider_overrides() -> ProviderOverrides<ProviderContext> {
    ProviderOverrides::new(
        |provider_id| CHAT_MODELS.get(provider_id),
        |provider_id| EMBEDDINGS.get(provider_id),
        |provider_id| CATALOGS.get(provider_id),
    )
}

/// A provider-neutral semantic category for a backend error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendErrorKind {
    RateLimited,
    InsufficientCredits,
    UpstreamUnavailable,
    ModelUnavailable,
    PayloadTooLarge,
    ContextLengthExceeded,
    BadRequest,
    Internal,
}

/// Provider-owned classification, with reporting ownership kept separate from
/// display semantics and the optional canonical backend token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendErrorClassification {
    /// Whether this classifier owns the provider identity itself.
    pub provider_owned: bool,
    /// Whether the backend owns the response for duplicate-report purposes.
    pub backend_owned: bool,
    /// Whether this client should still report the event (for known client bugs).
    pub reportable: bool,
    /// Canonical raw token, including unknown future tokens.
    pub token: Option<String>,
    /// Recognized provider-neutral display category.
    pub kind: Option<BackendErrorKind>,
    /// Whether a BAD_REQUEST was explicitly marked as malformed.
    pub malformed_bad_request: bool,
}

/// Classifies a provider's backend error. `provider` is present at a typed
/// provider boundary and absent for flattened, string-only host call sites.
pub type BackendErrorClassifier =
    Arc<dyn Fn(Option<&str>, &str) -> BackendErrorClassification + Send + Sync>;

static BACKEND_ERROR_CLASSIFIER: HookSlot<BackendErrorClassifier> =
    HookSlot::new("inference::backend_error_classifier");

/// Registers a backend error classifier process-wide.
pub fn register_backend_error_classifier(classifier: BackendErrorClassifier) {
    BACKEND_ERROR_CLASSIFIER.register("default", classifier);
}

/// Removes the process-wide backend error classifier.
pub fn unregister_backend_error_classifier() -> bool {
    BACKEND_ERROR_CLASSIFIER.unregister("default")
}

/// Registers a backend error classifier for a CoreContext.
pub fn register_backend_error_classifier_in(
    extensions: &Extensions,
    classifier: BackendErrorClassifier,
) {
    BACKEND_ERROR_CLASSIFIER.register_in(extensions, "default", classifier);
}

/// Classifies an error when a host has supplied a classifier.
pub fn classify_backend_error(
    provider: Option<&str>,
    error: &str,
) -> Option<BackendErrorClassification> {
    BACKEND_ERROR_CLASSIFIER
        .get("default")
        .map(|classifier| classifier(provider, error))
}

static CHAT_MODELS: HookSlot<ChatModelEntry<ProviderContext>> = HookSlot::new("inference::chat_models");

/// Registers the chat-model provider `provider_id` for the whole process.
pub fn register_chat_model(provider_id: impl Into<String>, entry: ChatModelEntry<ProviderContext>) {
    CHAT_MODELS.register(provider_id, entry);
}

/// Registers the chat-model provider `provider_id` for the context built from
/// `extensions`.
pub fn register_chat_model_in(
    extensions: &Extensions,
    provider_id: impl Into<String>,
    entry: ChatModelEntry<ProviderContext>,
) {
    CHAT_MODELS.register_in(extensions, provider_id, entry);
}

/// Removes the process-wide chat-model provider `provider_id`.
pub fn unregister_chat_model(provider_id: &str) -> bool {
    CHAT_MODELS.unregister(provider_id)
}

/// The registered chat-model provider, if any.
pub(crate) fn chat_model(provider_id: &str) -> Option<ChatModelEntry<ProviderContext>> {
    ambient_provider_overrides().chat_entry(provider_id)
}

/// The chat-model provider selected by an explicit context, falling back only
/// to process registration when that context has no entry.
pub(crate) fn chat_model_for(
    extensions: Option<&Extensions>,
    provider_id: &str,
) -> Option<ChatModelEntry<ProviderContext>> {
    provider_overrides(extensions.cloned()).chat_entry(provider_id)
}

/// The registered provider's model hooks, if it has any.
pub(crate) fn provider_model_hooks(provider_id: &str) -> Option<ProviderModelMetadata<ProviderContext>> {
    chat_model(provider_id)?.models
}

/// The registered provider's default model, if it declares one.
pub fn provider_default_model(provider_id: &str) -> Option<String> {
    let hooks = provider_model_hooks(provider_id)?;
    (hooks.default_model)()
}

/// The registered provider's default model selected by an explicit context,
/// falling back to process registration when that context has no entry.
pub(crate) fn provider_default_model_for(
    extensions: Option<&Extensions>,
    provider_id: &str,
) -> Option<String> {
    let hooks = chat_model_for(extensions, provider_id)?.models?;
    (hooks.default_model)()
}

/// Default pricing supplied by the registered provider, if any.
pub fn provider_default_pricing(provider_id: &str) -> Option<(String, f64, f64)> {
    let hooks = provider_model_hooks(provider_id)?;
    (hooks.default_pricing)()
}

pub(crate) fn provider_default_pricing_for(
    extensions: Option<&Extensions>,
    provider_id: &str,
) -> Option<(String, f64, f64)> {
    let hooks = chat_model_for(extensions, provider_id)?.models?;
    (hooks.default_pricing)()
}

/// The workload role assigned to a provider-specific model alias.
pub fn provider_model_role(provider_id: &str, model: &str) -> Option<String> {
    let hooks = provider_model_hooks(provider_id)?;
    (hooks.role_for_model)(model)
}

pub(crate) fn provider_model_role_for(
    extensions: Option<&Extensions>,
    provider_id: &str,
    model: &str,
) -> Option<String> {
    let hooks = chat_model_for(extensions, provider_id)?.models?;
    (hooks.role_for_model)(model)
}

/// Whether the provider recognizes `model` as an abstract alias.
pub fn provider_model_is_alias(provider_id: &str, model: &str) -> bool {
    provider_model_hooks(provider_id).is_some_and(|hooks| (hooks.is_alias)(model))
}

pub(crate) fn provider_model_is_alias_for(
    extensions: Option<&Extensions>,
    provider_id: &str,
    model: &str,
) -> bool {
    chat_model_for(extensions, provider_id)
        .and_then(|entry| entry.models)
        .is_some_and(|hooks| (hooks.is_alias)(model))
}

/// Whether the provider regards `model` as a concrete id for pass-through.
pub fn provider_model_is_raw(provider_id: &str, model: &str) -> bool {
    let model = model.trim();
    !model.is_empty()
        && !model.starts_with("hint:")
        && provider_model_hooks(provider_id).is_none_or(|hooks| (hooks.is_raw_model)(model))
}

pub(crate) fn provider_model_is_raw_for(
    extensions: Option<&Extensions>,
    provider_id: &str,
    model: &str,
) -> bool {
    let model = model.trim();
    !model.is_empty()
        && !model.starts_with("hint:")
        && chat_model_for(extensions, provider_id)
            .and_then(|entry| entry.models)
            .is_none_or(|hooks| (hooks.is_raw_model)(model))
}

/// Whether the provider says `model` supports image input.
pub fn provider_model_supports_vision(provider_id: &str, model: &str) -> bool {
    provider_model_hooks(provider_id).is_some_and(|hooks| (hooks.supports_vision)(model))
}

pub(crate) fn provider_model_supports_vision_for(
    extensions: Option<&Extensions>,
    provider_id: &str,
    model: &str,
) -> bool {
    chat_model_for(extensions, provider_id)
        .and_then(|entry| entry.models)
        .is_some_and(|hooks| (hooks.supports_vision)(model))
}

/// Provider-specific resolution of a role alias, configured default, or pin.
pub fn resolve_provider_model(
    provider_id: &str,
    role: &str,
    config: &Config,
    model_override: Option<&str>,
) -> Option<String> {
    let hooks = provider_model_hooks(provider_id)?;
    (hooks.resolve_model)(role, &ProviderContext::new(config, &EgressLedger::none()), model_override)
}

pub(crate) fn resolve_provider_model_for(
    extensions: Option<&Extensions>,
    provider_id: &str,
    role: &str,
    config: &Config,
    model_override: Option<&str>,
) -> Option<String> {
    let hooks = chat_model_for(extensions, provider_id)?.models?;
    (hooks.resolve_model)(role, &ProviderContext::new(config, &EgressLedger::none()), model_override)
}

/// The provider's model-specific context window, if known.
pub fn provider_model_context_window(provider_id: &str, model: &str) -> Option<u64> {
    let hooks = provider_model_hooks(provider_id)?;
    (hooks.context_window)(model)
}

pub(crate) fn provider_model_context_window_for(
    extensions: Option<&Extensions>,
    provider_id: &str,
    model: &str,
) -> Option<u64> {
    let hooks = chat_model_for(extensions, provider_id)?.models?;
    (hooks.context_window)(model)
}

/// Applies a provider-specific no-reasoning hint when the provider is installed.
pub fn provider_request_without_reasoning(
    provider_id: &str,
    request: tinyinference_llm::model::ModelRequest,
) -> tinyinference_llm::model::ModelRequest {
    match provider_model_hooks(provider_id) {
        Some(hooks) => (hooks.without_reasoning)(request),
        None => request,
    }
}

pub(crate) fn provider_request_without_reasoning_for(
    extensions: Option<&Extensions>,
    provider_id: &str,
    request: tinyinference_llm::model::ModelRequest,
) -> tinyinference_llm::model::ModelRequest {
    match chat_model_for(extensions, provider_id).and_then(|entry| entry.models) {
        Some(hooks) => (hooks.without_reasoning)(request),
        None => request,
    }
}

/// Registers a generic config normalizer, such as a provider-owned migration
/// that must also run after the config schema reaches its current version.
pub type ConfigNormalizer = Arc<dyn Fn(&mut Config) -> bool + Send + Sync>;

static CONFIG_NORMALIZERS: HookSlot<ConfigNormalizer> = HookSlot::new("config::normalizers");

/// Registers a config normalizer for the process.
pub fn register_config_normalizer(id: impl Into<String>, normalizer: ConfigNormalizer) {
    CONFIG_NORMALIZERS.register(id, normalizer);
}

/// Registers a config normalizer for a CoreContext's extensions.
pub fn register_config_normalizer_in(
    extensions: &Extensions,
    id: impl Into<String>,
    normalizer: ConfigNormalizer,
) {
    CONFIG_NORMALIZERS.register_in(extensions, id, normalizer);
}

/// Runs the currently registered config normalizers and reports whether any
/// changed the config.
pub(crate) fn normalize_config(config: &mut Config) -> bool {
    CONFIG_NORMALIZERS
        .entries_context_first()
        .into_iter()
        .fold(false, |changed, (_, normalize)| {
            normalize(config) || changed
        })
}

// ---------------------------------------------------------------------------
// Embeddings
// ---------------------------------------------------------------------------

static EMBEDDINGS: HookSlot<EmbeddingFactory<ProviderContext>> = HookSlot::new("inference::embeddings");

/// Registers the embedding provider `provider_id` for the whole process.
pub fn register_embedding(provider_id: impl Into<String>, factory: EmbeddingFactory<ProviderContext>) {
    EMBEDDINGS.register(provider_id, factory);
}

/// Registers the embedding provider `provider_id` for the context built from `extensions`.
pub fn register_embedding_in(extensions: &Extensions, provider_id: impl Into<String>, factory: EmbeddingFactory<ProviderContext>) {
    EMBEDDINGS.register_in(extensions, provider_id, factory);
}

/// Removes the process-wide embedding provider `provider_id`.
pub fn unregister_embedding(provider_id: &str) -> bool {
    EMBEDDINGS.unregister(provider_id)
}

/// The registered embedding factory, if any.
pub(crate) fn embedding(provider_id: &str) -> Option<EmbeddingFactory<ProviderContext>> {
    ambient_provider_overrides().embedding_factory(provider_id)
}

// ---------------------------------------------------------------------------
// Model catalogs
// ---------------------------------------------------------------------------

static CATALOGS: HookSlot<CatalogEntry<ProviderContext>> = HookSlot::new("inference::catalogs");

/// Registers the model catalog for `provider_id` for the whole process.
pub fn register_catalog(provider_id: impl Into<String>, entry: CatalogEntry<ProviderContext>) {
    CATALOGS.register(provider_id, entry);
}

/// Registers the model catalog for `provider_id` for the context built from
/// `extensions`.
pub fn register_catalog_in(
    extensions: &Extensions,
    provider_id: impl Into<String>,
    entry: CatalogEntry<ProviderContext>,
) {
    CATALOGS.register_in(extensions, provider_id, entry);
}

/// Removes the process-wide model catalog for `provider_id`.
pub fn unregister_catalog(provider_id: &str) -> bool {
    CATALOGS.unregister(provider_id)
}

/// The registered catalog, if any.
pub(crate) fn catalog(provider_id: &str) -> Option<CatalogEntry<ProviderContext>> {
    ambient_provider_overrides().catalog_entry(provider_id)
}

pub(crate) fn catalog_for(
    extensions: Option<&Extensions>,
    provider_id: &str,
) -> Option<CatalogEntry<ProviderContext>> {
    provider_overrides(extensions.cloned()).catalog_entry(provider_id)
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
