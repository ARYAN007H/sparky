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

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use tinyinference_embeddings::EmbeddingModel;
use tinyinference_llm::model::discover::DiscoveryRequest;

use crate::config::Config;
use crate::core::slots::{Extensions, HookSlot};
use crate::core::Outcome;
use crate::security::approval::ApprovalChatContext;
use crate::security::egress::EgressLedger;

/// A boxed `Send` future.
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

// ---------------------------------------------------------------------------
// Chat models
// ---------------------------------------------------------------------------

/// Host state supplied to a native TinyInference provider callback.
#[derive(Clone)]
pub struct ProviderContext {
    /// Configuration selected by the host for this call.
    pub config: Config,
    /// Egress ledger selected by the host for this call.
    pub egress: EgressLedger,
}

impl ProviderContext {
    fn new(config: &Config, egress: &EgressLedger) -> Self {
        Self { config: config.clone(), egress: egress.clone() }
    }
}

/// Native TinyInference chat registration types used by the host.
pub use tinyinference_llm::providers::overrides::{
    ChatModelEntry, ChatModelFactory, ChatModelRequest, ReadinessProbe,
};

/// Native provider metadata specialized to OpenHuman's host context.
pub type ProviderModelHooks =
    tinyinference_llm::providers::overrides::ProviderModelMetadata<ProviderContext>;

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

static CHAT_MODELS: HookSlot<ChatModelEntry> = HookSlot::new("inference::chat_models");

/// Registers the chat-model provider `provider_id` for the whole process.
pub fn register_chat_model(provider_id: impl Into<String>, entry: ChatModelEntry) {
    CHAT_MODELS.register(provider_id, entry);
}

/// Registers the chat-model provider `provider_id` for the context built from
/// `extensions`.
pub fn register_chat_model_in(
    extensions: &Extensions,
    provider_id: impl Into<String>,
    entry: ChatModelEntry,
) {
    CHAT_MODELS.register_in(extensions, provider_id, entry);
}

/// Removes the process-wide chat-model provider `provider_id`.
pub fn unregister_chat_model(provider_id: &str) -> bool {
    CHAT_MODELS.unregister(provider_id)
}

/// The registered chat-model provider, if any.
pub(crate) fn chat_model(provider_id: &str) -> Option<ChatModelEntry> {
    CHAT_MODELS.get(provider_id)
}

/// The chat-model provider selected by an explicit context, falling back only
/// to process registration when that context has no entry.
pub(crate) fn chat_model_for(
    extensions: Option<&Extensions>,
    provider_id: &str,
) -> Option<ChatModelEntry> {
    CHAT_MODELS.get_for(extensions, provider_id)
}

/// The registered provider's model hooks, if it has any.
pub(crate) fn provider_model_hooks(provider_id: &str) -> Option<ProviderModelHooks> {
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
    (hooks.resolve_model)(role, config, model_override)
}

pub(crate) fn resolve_provider_model_for(
    extensions: Option<&Extensions>,
    provider_id: &str,
    role: &str,
    config: &Config,
    model_override: Option<&str>,
) -> Option<String> {
    let hooks = chat_model_for(extensions, provider_id)?.models?;
    (hooks.resolve_model)(role, config, model_override)
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

/// What a registered embedding factory is asked to build.
pub struct EmbeddingRequest<'a> {
    /// The configuration in force, when the caller has one (the credential-only
    /// constructors do not).
    pub config: Option<&'a Config>,
    /// The embedding model id.
    pub model: &'a str,
    /// The vector dimensionality.
    pub dimensions: usize,
    /// The chat turn the call is made in, so disclosures reach that chat.
    pub chat: Option<ApprovalChatContext>,
    /// The directory the caller's credentials live in (the config's credential
    /// scope, or the process default when the caller has no config).
    pub state_dir: std::path::PathBuf,
    /// Whether that credential store is encrypted at rest.
    pub secrets_encrypt: bool,
}

/// Builds an embedding model.
pub type EmbeddingFactory = Arc<
    dyn for<'a> Fn(
            &EmbeddingRequest<'a>,
        ) -> tinyinference_embeddings::Result<Box<dyn EmbeddingModel>>
        + Send
        + Sync,
>;

static EMBEDDINGS: HookSlot<EmbeddingFactory> = HookSlot::new("inference::embeddings");

/// Registers the embedding provider `provider_id` for the whole process.
pub fn register_embedding<F>(provider_id: impl Into<String>, factory: F)
where
    F: for<'a> Fn(
            &EmbeddingRequest<'a>,
        ) -> tinyinference_embeddings::Result<Box<dyn EmbeddingModel>>
        + Send
        + Sync
        + 'static,
{
    EMBEDDINGS.register(provider_id, Arc::new(factory));
}

/// Registers the embedding provider `provider_id` for the context built from
/// `extensions`.
pub fn register_embedding_in<F>(extensions: &Extensions, provider_id: impl Into<String>, factory: F)
where
    F: for<'a> Fn(
            &EmbeddingRequest<'a>,
        ) -> tinyinference_embeddings::Result<Box<dyn EmbeddingModel>>
        + Send
        + Sync
        + 'static,
{
    EMBEDDINGS.register_in(extensions, provider_id, Arc::new(factory));
}

/// Removes the process-wide embedding provider `provider_id`.
pub fn unregister_embedding(provider_id: &str) -> bool {
    EMBEDDINGS.unregister(provider_id)
}

/// The registered embedding factory, if any.
pub(crate) fn embedding(provider_id: &str) -> Option<EmbeddingFactory> {
    EMBEDDINGS.get(provider_id)
}

// ---------------------------------------------------------------------------
// Model catalogs
// ---------------------------------------------------------------------------

/// What the core resolved for the `cloud_providers` entry a listing serves,
/// handed to the catalog so it honours the same per-entry settings every
/// other provider listing does.
#[derive(Clone)]
pub struct CatalogRequest {
    /// The key stored for the entry's slug (empty when none): the credential
    /// a catalog falls back to when there is no live session.
    pub provider_key: String,
    /// The HTTP client for the entry: the entry's CA bundle when it has one,
    /// else the runtime proxy client, under the `providers.list_models`
    /// service key.
    pub client: reqwest::Client,
}

/// Lists a provider's models: the same envelope `list_configured_models`
/// returns (`{ "models": [...] }`).
pub type CatalogListing = Arc<
    dyn Fn(Config, CatalogRequest) -> BoxFuture<Result<Outcome<serde_json::Value>, String>>
        + Send
        + Sync,
>;

/// Where to ask for one model's limits: `(model, config)`; `None` when the
/// provider has no listing for it.
pub type CatalogLimits = Arc<dyn Fn(&str, &Config) -> Option<DiscoveryRequest> + Send + Sync>;

/// A registered model catalog.
#[derive(Clone)]
pub struct CatalogEntry {
    /// The `list_configured_models` listing.
    pub list: CatalogListing,
    /// The limits-discovery request, when the provider has one.
    pub limits: Option<CatalogLimits>,
}

static CATALOGS: HookSlot<CatalogEntry> = HookSlot::new("inference::catalogs");

/// Registers the model catalog for `provider_id` for the whole process.
pub fn register_catalog(provider_id: impl Into<String>, entry: CatalogEntry) {
    CATALOGS.register(provider_id, entry);
}

/// Registers the model catalog for `provider_id` for the context built from
/// `extensions`.
pub fn register_catalog_in(
    extensions: &Extensions,
    provider_id: impl Into<String>,
    entry: CatalogEntry,
) {
    CATALOGS.register_in(extensions, provider_id, entry);
}

/// Removes the process-wide model catalog for `provider_id`.
pub fn unregister_catalog(provider_id: &str) -> bool {
    CATALOGS.unregister(provider_id)
}

/// The registered catalog, if any.
pub(crate) fn catalog(provider_id: &str) -> Option<CatalogEntry> {
    CATALOGS.get(provider_id)
}

pub(crate) fn catalog_for(
    extensions: Option<&Extensions>,
    provider_id: &str,
) -> Option<CatalogEntry> {
    CATALOGS.get_for(extensions, provider_id)
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
