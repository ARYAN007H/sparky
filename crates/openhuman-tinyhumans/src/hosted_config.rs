use openhuman_embed::__host::config::schema::cloud_providers::{
    register_hosted_cloud_provider_policy, register_hosted_cloud_preset, AuthStyle,
    BuiltinCloudProvider,
};
use std::sync::Arc;

pub(crate) fn register() {
    register_hosted_cloud_preset(
        "openhuman",
        BuiltinCloudProvider {
            slug: "openhuman",
            label: "OpenHuman",
            endpoint: "https://api.openhuman.ai/v1",
            auth_style: AuthStyle::OpenhumanJwt,
        },
    );
    register_hosted_cloud_provider_policy(Arc::new(|style| style == AuthStyle::OpenhumanJwt));
}
