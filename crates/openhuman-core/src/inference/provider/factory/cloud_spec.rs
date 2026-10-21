//! Host-resolved cloud provider inputs for TinyInference's provider registry.

use super::*;
use tinyinference_llm::providers::registry::CredentialResolver;
use tinyinference_llm::{ProviderKind, ProviderSpec};

use crate::config::schema::cloud_providers::CloudProviderCreds;

/// Maps a configured cloud entry and its effective model onto the registry's
/// spec without resolving credentials or changing the configured endpoint.
pub(crate) fn cloud_provider_spec(entry: &CloudProviderCreds, model: &str) -> ProviderSpec {
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

/// The bearer selected by OpenHuman's auth-profile and OAuth policy.
///
/// Resolve the host credential before creating a registry so errors and OAuth
/// token rotation retain the existing host semantics; the registry callback
/// then returns that exact selected credential for its provider spec.
pub(crate) struct CloudCredentialResolver<'a>(pub(crate) &'a str);

impl CredentialResolver for CloudCredentialResolver<'_> {
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

#[cfg(test)]
#[path = "cloud_spec_tests.rs"]
mod tests;
