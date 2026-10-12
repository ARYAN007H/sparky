//! OpenHuman-specific configuration for the TinyMCP module.
//!
//! This module keeps only host policy: converting product configuration to the
//! module contract, resolving the product proxy, and constructing the OAuth
//! redirect URL from the core's actual bound port. TinyMCP owns its service,
//! stores, transports, registries, and supervisor.

use std::sync::OnceLock;

use crate::config::Config;

const PROXY_SERVICE_KEY: &str = "tool.mcp_client";
const DEFAULT_CORE_PORT: u16 = 7788;
static DEFAULT_CONFIG: OnceLock<Config> = OnceLock::new();

/// Remembers the process config for ambient MCP calls without opening a module
/// service in the host.
pub fn set_default_config(config: &Config) {
    let _ = DEFAULT_CONFIG.set(config.clone());
}

/// The config named by the current scoped dispatch, or the process default
/// outside a tenant scope.
pub fn ambient_config() -> Option<Config> {
    match crate::core::runtime::current_tenant() {
        Ok(tenant) if tenant.is_scoped() => {
            crate::core::runtime::CoreContext::with_current_embedder_config(Clone::clone)
        }
        _ => DEFAULT_CONFIG.get().cloned(),
    }
}

/// Converts the MCP section of product configuration to the module contract.
pub fn client_config(config: &Config) -> tinymcp_bus::McpClientConfig {
    let mut client = tinymcp_bus::McpClientConfig {
        enabled: config.mcp_client.enabled,
        servers: config
            .mcp_client
            .servers
            .iter()
            .filter(|server| credentialed_endpoint_transport_allowed(server))
            .map(|server| server.server.clone())
            .collect(),
        proxy: proxy_for_mcp(),
        ..tinymcp_bus::McpClientConfig::default()
    };

    client.client_identity.name = config.mcp_client.client_identity.name.clone();
    client.client_identity.title = config.mcp_client.client_identity.title.clone();
    client.client_identity.version = config.mcp_client.client_identity.version.clone();
    client.registry_auth = config.mcp_client.registry_auth.clone();

    // GitBooks uses dedicated deferred tools and is not part of the configured
    // server set, so first-turn tool assembly never dials it for a catalog.
    client
}

/// Whether a configured credential may be sent to its endpoint. Credentials
/// are allowed over HTTPS, stdio, or loopback HTTP only.
fn credentialed_endpoint_transport_allowed(server: &crate::config::McpServerConfig) -> bool {
    use crate::config::McpAuthConfig;

    if matches!(server.auth, McpAuthConfig::None) {
        return true;
    }
    let endpoint = server.endpoint.trim();
    if endpoint.is_empty() {
        return true;
    }
    let Ok(url) = reqwest::Url::parse(endpoint) else {
        return true;
    };
    if url.scheme() == "https" || !matches!(url.scheme(), "http") {
        return true;
    }
    let loopback = url
        .host_str()
        .is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "::1"));
    if loopback {
        return true;
    }
    tracing::warn!(
        server = %server.name,
        "[mcp] refusing credentialed plaintext HTTP endpoint; use https:// instead"
    );
    false
}

/// The loopback address a browser sign-in redirects back to: the bound core
/// port, then the requested port, then the default.
pub fn oauth_redirect_uri() -> String {
    let port = std::env::var("OPENHUMAN_CORE_RPC_URL")
        .ok()
        .and_then(|url| explicit_port(&url))
        .or_else(|| {
            std::env::var("OPENHUMAN_CORE_PORT")
                .ok()
                .and_then(|value| value.trim().parse::<u16>().ok())
        })
        .unwrap_or(DEFAULT_CORE_PORT);

    format!("http://127.0.0.1:{port}/oauth/mcp/callback")
}

fn explicit_port(url: &str) -> Option<u16> {
    reqwest::Url::parse(url).ok().and_then(|url| url.port())
}

/// The proxy MCP traffic should use, or `None` to connect directly. The scope,
/// per-service list, and no-proxy list are OpenHuman policy.
pub fn proxy_for_mcp() -> Option<tinymcp_bus::McpProxyConfig> {
    let proxy = crate::config::runtime_proxy_config();
    if !proxy.should_apply_to_service(PROXY_SERVICE_KEY) {
        return None;
    }
    Some(tinymcp_bus::McpProxyConfig {
        http_proxy: proxy.http_proxy.clone(),
        https_proxy: proxy.https_proxy.clone(),
        all_proxy: proxy.all_proxy.clone(),
        no_proxy: proxy.no_proxy.clone(),
    })
}

#[cfg(test)]
#[path = "host_tests.rs"]
mod tests;
