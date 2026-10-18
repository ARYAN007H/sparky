//! Langfuse export for durable graph runs.
//!
//! This exporter consumes generic TinyAgents graph observations plus the
//! serde-only TinyFlows bus trigger value. The host that owns engine
//! observations adapts them before calling this module; this exporter owns the
//! graph trace identity, metadata, and OTLP projection.

use std::time::Duration;

use serde_json::{Map, Value, json};
use tinyagents_graph::{GraphLangfuseExporter, GraphObservation};
use tinyagents_harness::{LangfuseAuth, LangfuseClient, LangfuseTraceConfig};
use tinyflows_bus::runs::FlowRunTrigger;

use crate::config::Config;
use crate::security::credentials::session_support::direct_backend_credential;

const LOG_TARGET: &str = "flows::langfuse";
const PUSH_TIMEOUT: Duration = Duration::from_secs(10);
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

fn trigger_name(trigger: FlowRunTrigger) -> &'static str {
    match trigger {
        FlowRunTrigger::Rpc => "rpc",
        FlowRunTrigger::Schedule => "schedule",
        FlowRunTrigger::AppEvent => "app_event",
        FlowRunTrigger::Resume => "resume",
    }
}

fn build_flow_trace_config(
    flow_name: &str,
    flow_id: &str,
    thread_id: &str,
    status: &str,
    trigger: FlowRunTrigger,
) -> LangfuseTraceConfig {
    let trigger = trigger_name(trigger);
    LangfuseTraceConfig {
        trace_id: Some(thread_id.to_string()),
        name: Some(format!("flow.run:{flow_name}")),
        session_id: Some(format!("flow:{flow_id}")),
        release: Some(APP_VERSION.to_string()),
        tags: vec!["run:flow".to_string(), format!("trigger:{trigger}")],
        metadata: json!({
            "flow_id": flow_id,
            "status": status,
            "source": "flows",
            "run_type": "flow",
            "trigger": trigger,
            "app_version": APP_VERSION,
        }),
        ..Default::default()
    }
}

fn build_flow_exporter(client: LangfuseClient, flow_id: &str) -> GraphLangfuseExporter {
    let flow_id = flow_id.to_string();
    GraphLangfuseExporter::new(client).with_span_metadata_fn(move |_obs| {
        let mut extra = Map::new();
        extra.insert("flow_id".to_string(), json!(flow_id));
        Some(extra)
    })
}

fn flow_otlp_requests(
    exporter: &GraphLangfuseExporter,
    trace: LangfuseTraceConfig,
    observations: &[GraphObservation],
) -> tinyagents_harness::error::Result<Vec<Value>> {
    exporter.build_otlp_requests(
        trace,
        observations,
        crate::agent::progress_tracing::export_brand().product,
    )
}

#[cfg(test)]
fn graph_otlp_requests(
    flow_name: &str,
    flow_id: &str,
    thread_id: &str,
    status: &str,
    trigger: FlowRunTrigger,
    observations: &[GraphObservation],
) -> tinyagents_harness::error::Result<Vec<Value>> {
    let client = LangfuseClient::new(
        "https://backend.invalid/telemetry/langfuse/ingestion",
        LangfuseAuth::Bearer {
            token: String::new(),
        },
    )?;
    let exporter = build_flow_exporter(client, flow_id);
    flow_otlp_requests(
        &exporter,
        build_flow_trace_config(flow_name, flow_id, thread_id, status, trigger),
        observations,
    )
}

/// Exports one settled flow graph to Langfuse. Failures are logged and
/// swallowed so tracing cannot fail or delay-fail the flow run.
pub(crate) async fn export_graph_run_trace(
    config: &Config,
    flow_name: &str,
    flow_id: &str,
    thread_id: &str,
    status: &str,
    trigger: FlowRunTrigger,
    observations: &[GraphObservation],
) {
    if !config.observability.share_usage_data {
        tracing::debug!(target: LOG_TARGET, flow_id = %flow_id, "[flows] langfuse export skipped: observability.share_usage_data is off");
        return;
    }
    if observations.is_empty() {
        tracing::debug!(target: LOG_TARGET, flow_id = %flow_id, thread_id = %thread_id, "[flows] langfuse export skipped: run journal is empty");
        return;
    }

    let url = super::langfuse::ingestion_url(config);
    if !url.starts_with("http") {
        tracing::warn!(target: LOG_TARGET, flow_id = %flow_id, "[flows] langfuse export skipped: could not resolve ingestion URL from backend host (got {url:?})");
        return;
    }
    let token = match direct_backend_credential(config, "flows langfuse export") {
        Some(crate::security::credentials::session_support::BackendCredential::Session(token)) => {
            token
        }
        _ => {
            tracing::debug!(target: LOG_TARGET, flow_id = %flow_id, "[flows] langfuse export skipped: hosted backend not available");
            return;
        }
    };
    let client = match LangfuseClient::new(
        url.clone(),
        LangfuseAuth::Bearer {
            token: token.clone(),
        },
    ) {
        Ok(client) => client,
        Err(err) => {
            tracing::warn!(target: LOG_TARGET, flow_id = %flow_id, error = %err, "[flows] langfuse export skipped: could not build client for {url}");
            return;
        }
    };
    let exporter = build_flow_exporter(client, flow_id);
    let trace = build_flow_trace_config(flow_name, flow_id, thread_id, status, trigger);
    let requests = match flow_otlp_requests(&exporter, trace, observations) {
        Ok(requests) => requests,
        Err(err) => {
            tracing::warn!(target: LOG_TARGET, flow_id = %flow_id, thread_id = %thread_id, error = %err, "[flows] langfuse export skipped: could not build OTLP requests");
            return;
        }
    };
    let request_count = requests.len();
    let otlp_url = exporter.client().otlp_endpoint();
    let send =
        crate::agent::progress_tracing::otlp::push_otlp_json_requests(&otlp_url, &token, &requests);
    match tokio::time::timeout(PUSH_TIMEOUT, send).await {
        Ok(Ok(())) => {
            tracing::debug!(target: LOG_TARGET, flow_id = %flow_id, thread_id = %thread_id, observation_count = observations.len(), request_count, "[flows] pushed flow run trace to Langfuse")
        }
        Ok(Err(err)) => {
            tracing::warn!(target: LOG_TARGET, flow_id = %flow_id, thread_id = %thread_id, error = %err, "[flows] langfuse export failed (run unaffected)")
        }
        Err(_) => {
            tracing::warn!(target: LOG_TARGET, flow_id = %flow_id, thread_id = %thread_id, timeout_secs = PUSH_TIMEOUT.as_secs(), "[flows] langfuse export timed out (run unaffected)")
        }
    }
}

#[cfg(test)]
#[path = "flow_export_tests.rs"]
mod tests;
