//! The G5 config enricher: fills `Config.resolved_endpoints` from this
//! crate's URL, header and product modules, so the core reads the resolved
//! control-plane origin, socket and tracing URLs, attribution headers and
//! product identity from config instead of owning them.
//!
//! | `ResolvedEndpoints` field | Source |
//! | --- | --- |
//! | `control_plane_url` | [`effective_backend_api_url`]`(&config.api_url)` |
//! | `inference_url` | [`effective_api_url`]`(&config.api_url)` |
//! | `socket_url` | the control-plane origin as a ready-to-dial Socket.IO URL |
//! | `tracing_ingest_url` | `<control>/telemetry/langfuse/ingestion` |
//! | `attribution_headers` | [`attribution_headers`](crate::backend::headers::attribution_headers) |
//! | `product_identity` | [`product_identity`](crate::backend::product::product_identity) |
//!
//! `config.api_url` is honoured exactly as the core's `backend::base_url` /
//! `inference_base_url` honour it today: the transport's `base_url` is these
//! same two functions.

use openhuman_embed::__host::config::Config;
use openhuman_embed::__host::config_enrichers::{register_enricher, ResolvedEndpoints};
use openhuman_embed::__host::util::url::join_url;

use crate::backend::headers::attribution_headers;
use crate::backend::product::{product_identity, product_identity_headers};
use crate::backend::url::{effective_api_url, effective_backend_api_url};

/// The id this enricher registers under.
pub const ENRICHER_ID: &str = "tinyhumans";

/// The Langfuse ingestion proxy route, relative to the control-plane origin.
/// The backend authenticates the session JWT, injects the Langfuse project
/// keys and forwards; clients POST here, never to `/api/public/ingestion`.
pub const TRACING_INGEST_PATH: &str = "/telemetry/langfuse/ingestion";

/// The Socket.IO handshake URL for an HTTP(S) control-plane origin
/// (`https://host` -> `wss://host/socket.io/?EIO=4&transport=websocket`).
/// Returned unchanged when `base` does not parse.
pub fn socket_url_for(base: &str) -> String {
    let Ok(mut url) = url::Url::parse(base) else {
        return base.to_string();
    };
    let scheme = match url.scheme() {
        "https" => "wss",
        "http" => "ws",
        other => other,
    }
    .to_string();
    let _ = url.set_scheme(&scheme);
    url.set_path(&format!("{}/socket.io/", url.path().trim_end_matches('/')));
    url.set_query(Some("EIO=4&transport=websocket"));
    url.to_string()
}

fn header_pairs(headers: &reqwest::header::HeaderMap) -> Vec<(String, String)> {
    headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|v| (name.as_str().to_string(), v.to_string()))
        })
        .collect()
}

/// Resolve every field for `config`. Pure apart from the product identity and
/// environment overrides the URL module reads.
pub fn resolve(config: &Config) -> ResolvedEndpoints {
    let control = effective_backend_api_url(&config.api_url);
    // A malformed attribution header degrades to the identity header alone,
    // as the transport does.
    let headers = attribution_headers().unwrap_or_else(|_| product_identity_headers());
    ResolvedEndpoints {
        socket_url: Some(socket_url_for(&control)),
        tracing_ingest_url: Some(join_url(&control, TRACING_INGEST_PATH)),
        inference_url: Some(effective_api_url(&config.api_url)),
        attribution_headers: header_pairs(&headers),
        product_identity: Some(product_identity().as_str().to_owned()),
        control_plane_url: Some(control),
    }
}

/// The enricher body: write the resolved endpoints onto `config`.
pub fn enrich(config: &mut Config) {
    let resolved = resolve(config);
    log::trace!(
        "[tinyhumans] config enricher: control_plane_url={:?}",
        resolved.control_plane_url
    );
    config.resolved_endpoints = resolved;
}

/// Registers the enricher with the core. Idempotent (re-registering replaces).
pub(crate) fn register() {
    register_enricher(ENRICHER_ID, enrich);
}

#[cfg(test)]
#[path = "enricher_tests.rs"]
mod tests;
