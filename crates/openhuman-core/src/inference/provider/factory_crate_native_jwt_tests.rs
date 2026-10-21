use super::*;

#[test]
fn unmanaged_openhuman_jwt_route_never_builds_a_direct_unauthenticated_model() {
    let _guard = crate::inference::inference_test_guard();
    let mut config = Config::default();
    config.cloud_providers.push(CloudProviderCreds {
        id: "p_managed".to_string(),
        slug: "managed".to_string(),
        label: "Managed".to_string(),
        endpoint: "https://api.example/v1".to_string(),
        auth_style: AuthStyle::OpenhumanJwt,
        ..Default::default()
    });

    let error = try_create_cloud_slug_chat_model_from_string_with_native_tools(
        "chat",
        "managed:model",
        &config,
        true,
        &EgressLedger::none(),
    )
    .expect("configured slug")
    .expect_err("unregistered managed credentials must not reach the direct registry");

    assert!(
        error
            .to_string()
            .contains("hosted cloud authentication is unavailable"),
        "unexpected error: {error}"
    );
}
