//! The managed model catalog: the `list_configured_models` listing for the
//! managed provider and the model-limits discovery request, both against the
//! hosted backend's `/openai/v1/models?catalog=openrouter`.
//!
//! `?catalog=openrouter` is required: without it the backend returns only the
//! curated legacy list. The catalog listing is gated server-side and returns an
//! empty set when the passthrough is off, so this degrades to "no models"
//! rather than an error on a backend that has not enabled it.

use openhuman_embed::__host::config::Config;
use openhuman_embed::__host::core::Outcome;
use openhuman_embed::__host::inference_registry::{
    BoxFuture, CatalogEntry, CatalogLimits, CatalogListing,
};
use openhuman_embed::__host::security::credentials::session_support::{
    classify_session_token, load_app_session_profile, publish_local_session_expiry,
    resolve_backend_credential, BackendCredential, SessionTokenCheck,
};
use openhuman_embed::__host::util::url::join_url;
use std::sync::Arc;
use tinyinference_llm::catalog::{append_query_param, parse_models_response};
use tinyinference_llm::model::discover::DiscoveryRequest;

use crate::backend::headers::attribution_headers;
use crate::backend::url::{effective_api_url, effective_backend_api_url};
use crate::client::endpoint::{
    is_managed_endpoint_for_api_key, is_safe_endpoint_for_managed_bearer,
};

const MODELS_PATH: &str = "/openai/v1/models";

/// The registry entry for the managed catalog.
pub(super) fn entry() -> CatalogEntry {
    let list: CatalogListing = Arc::new(
        |config: Config| -> BoxFuture<Result<Outcome<serde_json::Value>, String>> {
            Box::pin(async move { list_models(&config).await })
        },
    );
    let limits: CatalogLimits =
        Arc::new(|model: &str, config: &Config| limits_request(model, config));
    CatalogEntry {
        list,
        limits: Some(limits),
    }
}

fn empty_catalog(reason: &str) -> Result<Outcome<serde_json::Value>, String> {
    Ok(Outcome::new(
        serde_json::json!({ "models": Vec::<serde_json::Value>::new() }),
        vec![format!("{reason}; managed catalog is empty")],
    ))
}

fn catalog_url(config: &Config) -> String {
    append_query_param(
        &join_url(&effective_backend_api_url(&config.api_url), MODELS_PATH),
        "catalog",
        "openrouter",
    )
}

/// Whether a non-2xx from the listing reads as "signed out": a 401 on a request
/// that actually carried the app session token. A 401 without one describes the
/// request, not the session.
pub(super) fn managed_401_means_signed_out(status: u16, session_attached: bool) -> bool {
    status == 401 && session_attached
}

fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let cut: String = text.chars().take(max_chars).collect();
    format!("{cut}...")
}

/// The `list_configured_models` listing for the managed provider.
pub(super) async fn list_models(config: &Config) -> Result<Outcome<serde_json::Value>, String> {
    // Classify the session directly: a store error propagates (a recoverable
    // fault the picker should surface), only an absent or expired session
    // degrades to an empty list.
    let profile = load_app_session_profile(config)?;
    let token = match classify_session_token(profile.as_ref(), chrono::Utc::now()) {
        SessionTokenCheck::Live(token) => token,
        SessionTokenCheck::Expired => {
            // `require_live_session_token` announced this; without it an expired
            // token stays in the store with nothing prompting a re-auth.
            publish_local_session_expiry("list_configured_models");
            log::info!("[tinyhumans][inference] managed catalog unavailable: session expired");
            return empty_catalog("session expired");
        }
        SessionTokenCheck::Absent => {
            log::info!("[tinyhumans][inference] managed catalog unavailable: no session");
            return empty_catalog("no session");
        }
    };

    let url = catalog_url(config);
    log::debug!("[tinyhumans][inference] managed catalog url={url}");
    let client = openhuman_embed::__host::config::build_runtime_proxy_client_with_timeouts(
        "providers.list_models",
        30,
        10,
    );

    let attach = is_safe_endpoint_for_managed_bearer(&url);
    let mut request = client.get(&url);
    if attach {
        request = request.header("Authorization", format!("Bearer {token}"));
        if let Ok(headers) = attribution_headers() {
            request = request.headers(headers);
        }
    } else {
        log::warn!(
            "[tinyhumans][inference] refusing to send a bearer token to a non-https, non-loopback URL"
        );
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("[providers][list_models] HTTP request failed: {e}"))?;
    let status = response.status();
    if !status.is_success() {
        if managed_401_means_signed_out(status.as_u16(), attach) {
            log::info!(
                "[tinyhumans][inference] managed catalog unavailable: backend rejected the session token (401)"
            );
            return empty_catalog("session not accepted");
        }
        let body = response.text().await.unwrap_or_default();
        let sanitized = tinyinference_core::sanitize::sanitize_api_error(&body);
        return Err(format!(
            "provider returned {}: {}",
            status.as_u16(),
            truncate(&sanitized, 300)
        ));
    }

    let raw_body = response
        .text()
        .await
        .map_err(|e| format!("[providers][list_models] failed to read response body: {e}"))?;
    let body: serde_json::Value = serde_json::from_str(&raw_body).map_err(|e| {
        let snippet = truncate(
            &tinyinference_core::sanitize::sanitize_api_error(&raw_body),
            300,
        );
        format!("[providers][list_models] failed to parse JSON: {e} (body: {snippet})")
    })?;
    if let Some(err_field) = body.get("error") {
        let msg = err_field
            .as_str()
            .map(str::to_string)
            .or_else(|| {
                err_field
                    .get("message")
                    .and_then(|m| m.as_str())
                    .map(str::to_string)
            })
            .unwrap_or_else(|| err_field.to_string());
        return Err(format!(
            "provider returned error payload: {}",
            tinyinference_core::sanitize::sanitize_api_error(&msg)
        ));
    }
    let models = parse_models_response(&body).map_err(|error| error.to_string())?;
    log::info!(
        "[tinyhumans][inference] managed catalog fetched {} models",
        models.len()
    );
    Ok(Outcome::new(
        serde_json::json!({ "models": models }),
        vec![format!("fetched {} models", models.len())],
    ))
}

/// Where to ask the managed catalog for `model`'s limits, keyed by the managed
/// inference base so overflow corrections from the chat model land on the same
/// entry. Authenticated with the backend credential when the URL is safe to
/// carry it.
pub(super) fn limits_request(model: &str, config: &Config) -> Option<DiscoveryRequest> {
    let model = model.trim();
    if model.is_empty() {
        return None;
    }
    let inference_base = effective_api_url(&config.api_url);
    let endpoint = format!("{}/openai/v1", inference_base.trim_end_matches('/'));
    let listing_url = catalog_url(config);
    let mut request = DiscoveryRequest::new(endpoint, model)
        .with_listing_url(listing_url.clone())
        .with_single_model_probe(false);
    match resolve_backend_credential(config) {
        Ok(BackendCredential::ApiKey(key)) if is_managed_endpoint_for_api_key(&listing_url) => {
            request = request.with_header("Authorization", format!("Bearer {key}"));
        }
        Ok(BackendCredential::Session(token))
            if is_safe_endpoint_for_managed_bearer(&listing_url) =>
        {
            request = request.with_header("Authorization", format!("Bearer {token}"));
        }
        Ok(_) => {
            log::warn!(
                "[tinyhumans][inference] refusing to send the backend credential to an unsafe listing url"
            );
        }
        Err(_) => {
            log::debug!(
                "[tinyhumans][inference] no backend credential; managed listing probed anonymously"
            );
        }
    }
    if let Ok(headers) = attribution_headers() {
        for (name, value) in headers.iter() {
            if let Ok(value) = value.to_str() {
                request = request.with_header(name.as_str(), value);
            }
        }
    }
    Some(request)
}

#[cfg(test)]
#[path = "managed_catalog_tests.rs"]
mod tests;
