use super::*;

use crate::config::schema::cloud_providers::CloudProviderCreds;

fn provider(id: &str, slug: &str, endpoint: &str, auth_style: AuthStyle) -> CloudProviderCreds {
    CloudProviderCreds {
        id: id.to_string(),
        slug: slug.to_string(),
        label: slug.to_string(),
        endpoint: endpoint.to_string(),
        auth_style,
        default_model: Some("provider-model".to_string()),
    }
}

#[test]
fn explicit_managed_primary_yields_matching_legacy_custom_provider() {
    let mut config = Config::default();
    config.cloud_providers = vec![
        provider(
            "managed",
            "openhuman",
            "https://api.openhuman.ai/v1",
            AuthStyle::OpenhumanJwt,
        ),
        provider(
            "custom",
            "my-provider",
            "https://example.test/v1/",
            AuthStyle::Bearer,
        ),
    ];
    config.primary_cloud = Some("managed".to_string());
    config.inference_url = Some("https://EXAMPLE.test/v1".to_string());

    assert_eq!(
        resolve_primary_cloud_provider_string(&config),
        "my-provider:provider-model"
    );
}

#[test]
fn explicit_managed_primary_fails_closed_for_unmatched_custom_intent() {
    let mut config = Config::default();
    config.cloud_providers = vec![provider(
        "managed",
        "openhuman",
        "https://api.openhuman.ai/v1",
        AuthStyle::OpenhumanJwt,
    )];
    config.primary_cloud = Some("managed".to_string());
    config.inference_url = Some("https://unmatched.example.test/v1".to_string());

    assert_eq!(
        resolve_primary_cloud_provider_string(&config),
        crate::inference::provider::factory::BYOK_INCOMPLETE_SENTINEL
    );
}

#[test]
fn absent_primary_without_custom_intent_retains_managed_route() {
    let config = Config::default();
    assert_eq!(
        resolve_primary_cloud_provider_string(&config),
        crate::inference::provider::factory::PROVIDER_OPENHUMAN
    );
}

#[test]
fn resolved_managed_host_is_not_treated_as_byok_intent() {
    let mut config = Config::default();
    config.resolved_endpoints.managed_hosts = vec![".managed.test".to_string()];
    config.inference_url = Some("https://api.managed.test/v1".to_string());

    assert!(!has_custom_inference_intent(&config));
    assert_eq!(
        resolve_primary_cloud_provider_string(&config),
        crate::inference::provider::factory::PROVIDER_OPENHUMAN
    );
}
