//! Host-resolved cloud provider inputs for TinyInference's provider registry.

use super::*;
use tinyinference_llm::providers::openai::codex::OpenAiCodexRouting;
use tinyinference_llm::providers::registry::{
    CredentialResolver, ProviderBuildOptions, ProviderRegistry,
};
use tinyinference_llm::{ProviderKind, ProviderSpec};

use crate::config::schema::cloud_providers::CloudProviderCreds;

/// Maps a configured cloud entry and its effective model onto the registry's
/// spec without resolving credentials or changing the configured endpoint.
pub(super) fn cloud_provider_spec(entry: &CloudProviderCreds, model: &str) -> ProviderSpec {
    ProviderSpec {
        kind: match entry.auth_style {
            AuthStyle::Anthropic => ProviderKind::Anthropic,
            _ => ProviderKind::Compatible,
        },
        provider: entry.slug.clone(),
        model: model.to_string(),
        base_url: entry.endpoint.clone(),
        api_key_env: None,
        requires_api_key: false,
    }
}

/// A credential already selected by OpenHuman's auth-profile and OAuth policy.
pub(super) struct ResolvedCloudCredential<'a>(pub(super) &'a str);

impl CredentialResolver for ResolvedCloudCredential<'_> {
    fn resolve(&self, _spec: &ProviderSpec) -> tinyinference_llm::Result<String> {
        Ok(self.0.to_owned())
    }
}

impl From<AuthStyle> for tinyinference_llm::providers::openai::AuthStyle {
    fn from(style: AuthStyle) -> Self {
        match style {
            AuthStyle::Bearer => Self::Bearer,
            AuthStyle::Anthropic => Self::Anthropic,
            AuthStyle::None | AuthStyle::OpenhumanJwt => Self::None,
        }
    }
}

/// Builds a cloud provider through the owner registry with host-resolved policy.
pub(super) fn build_cloud_model(
    spec: &ProviderSpec,
    key: &str,
    auth_style: AuthStyle,
    temperature_override: Option<f64>,
    temperature_unsupported_models: &[String],
    native_tool_calling: bool,
    extra_headers: &[(String, String)],
    extra_query_params: &[(String, String)],
    user_agent: Option<&str>,
    codex: Option<&OpenAiCodexRouting>,
    http: Option<reqwest::Client>,
) -> anyhow::Result<Arc<dyn ChatModel<()>>> {
    log::debug!(
        "[providers][cloud-spec] build slug={} kind={} auth={:?}",
        spec.provider,
        spec.kind.as_str(),
        auth_style
    );
    ProviderRegistry::new(ResolvedCloudCredential(key))
        .build_with(
            spec,
            ProviderBuildOptions {
                auth_style: Some(auth_style.into()),
                temperature_override,
                temperature_unsupported_models,
                native_tool_calling: Some(native_tool_calling),
                extra_headers,
                extra_query_params,
                user_agent,
                codex,
                http,
                ..ProviderBuildOptions::default()
            },
        )
        .map_err(|error| anyhow::anyhow!("[chat-factory] {error}"))
}

/// The registry's listing request for a resolved route, or `None` when the
/// route has no discoverable listing.
pub(super) fn cloud_discovery_request(
    spec: &ProviderSpec,
    key: &str,
    auth_style: AuthStyle,
    extra_headers: &[(String, String)],
    extra_query_params: &[(String, String)],
    user_agent: Option<&str>,
    codex: Option<&OpenAiCodexRouting>,
) -> Option<tinyinference_llm::model::discover::DiscoveryRequest> {
    match ProviderRegistry::new(ResolvedCloudCredential(key)).discovery_request(
        spec,
        &ProviderBuildOptions {
            auth_style: Some(auth_style.into()),
            extra_headers,
            extra_query_params,
            user_agent,
            codex,
            ..ProviderBuildOptions::default()
        },
    ) {
        Ok(request) => Some(request),
        Err(error) => {
            log::debug!(
                "[model_limits] registry discovery unavailable slug={}: {error}",
                spec.provider
            );
            None
        }
    }
}

#[cfg(test)]
#[path = "cloud_spec_tests.rs"]
mod tests;
