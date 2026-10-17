//! Bounded OTLP/HTTP JSON export through the authenticated backend proxy.
//!
//! The span-to-OTLP conversion is upstream
//! (`tinyagents_harness::observability::trace_export::otlp`); this module only
//! resolves the backend URL and credential and posts the requests.

use std::time::Duration;

use tinyagents_harness::observability::trace_export::otlp::otlp_requests;
use tinyagents_harness::observability::trace_export::TraceSpan;
use tinyagents_harness::observability::LangfuseClient;

use super::langfuse::{environment_for_base, ingestion_url, skip_push};
use crate::config::Config;
use crate::security::credentials::session_support::direct_backend_credential;

const PUSH_TIMEOUT: Duration = Duration::from_secs(10);

pub(super) async fn push_spans(config: &Config, spans: &[TraceSpan]) -> Result<(), String> {
    if spans.is_empty() {
        return Ok(());
    }
    let url = ingestion_url(config);
    let environment = environment_for_base(&url);
    if skip_push(environment) {
        return Ok(());
    }
    if !url.starts_with("http") {
        return Err("Langfuse backend proxy URL is unavailable".to_string());
    }
    // No TinyHumans connection, or no usable credential (signed out, offline
    // local session): a configured state, so skip quietly rather than failing
    // every turn's push.
    let token = match direct_backend_credential(config, "langfuse otlp push") {
        Some(crate::security::credentials::session_support::BackendCredential::Session(token)) => {
            token
        }
        _ => return Ok(()),
    };
    // Reuse the host's configured API transport for TLS, attribution headers,
    // and route policy. LangfuseClient owns the proxy route, bearer header,
    // OTLP v4 header, and response validation.
    let transport = crate::backend::resolve_backend_transport()
        .map_err(|err| format!("Langfuse OTLP push has no backend transport: {err}"))?;
    let client = LangfuseClient::proxy_with_transport(
        url,
        token,
        transport.http_client(crate::backend::TransportProfile::Api),
        transport.attribution_headers(),
    )
    .map_err(|_| "Langfuse OTLP proxy client could not be configured".to_string())?;
    for payload in otlp_requests(spans, environment, &super::export_brand()) {
        tokio::time::timeout(PUSH_TIMEOUT, client.send_otlp(payload))
            .await
            .map_err(|_| "Langfuse OTLP proxy request timed out".to_string())?
            .map_err(|_| "Langfuse OTLP proxy request failed".to_string())?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "otlp_tests.rs"]
mod tests;
