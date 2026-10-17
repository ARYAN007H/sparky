//! Handing finished spans to their sink(s): the local NDJSON/log exporter and
//! the remote Langfuse usage-data push.

use crate::config::schema::AgentTracingConfig;
use crate::config::Config;

use super::langfuse;
use super::otlp;
use crate::config::schema::AgentTracingBackend;
use tinyagents_harness::observability::trace_export::serialize::{spans_to_ndjson, SpanEnvelope};
use tinyagents_harness::observability::trace_export::{TraceContext, TraceSpan};

/// Map the configured backend onto the upstream NDJSON envelope.
pub(crate) fn envelope_for(backend: AgentTracingBackend) -> SpanEnvelope {
    match backend {
        AgentTracingBackend::Otel => SpanEnvelope::Otel,
        AgentTracingBackend::Langfuse => SpanEnvelope::Langfuse,
    }
}

/// Export finished spans per the [`AgentTracingConfig`]: append NDJSON to the
/// configured file, or emit to the application log when no path is set.
/// Best-effort — a failed write is logged and swallowed so tracing never
/// breaks an agent run. A no-op when tracing is disabled or there are no spans.
pub(crate) fn export_spans(config: &AgentTracingConfig, spans: &[TraceSpan]) {
    if !config.enabled || spans.is_empty() {
        return;
    }
    let payload = spans_to_ndjson(envelope_for(config.backend), spans);
    match &config.export_path {
        Some(path) => {
            use std::io::Write as _;
            let opened = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path);
            match opened {
                Ok(mut file) => {
                    if let Err(err) = file.write_all(payload.as_bytes()) {
                        log::warn!(
                            "[agent-tracing] failed to append {} spans to {path}: {err}",
                            spans.len()
                        );
                    } else {
                        log::debug!("[agent-tracing] exported {} spans to {path}", spans.len());
                    }
                }
                Err(err) => log::warn!("[agent-tracing] failed to open {path}: {err}"),
            }
        }
        None => {
            // No path configured. Surface only metadata (count + trace id) at
            // `info` so the export is visible on read-only / sandboxed
            // deployments WITHOUT ever printing span content at `info` (#4454).
            // The NDJSON body — which may carry prompt/reply text when
            // `capture_content` is opted in — goes to `debug` only. With the
            // default gate off, the storage layer already strips content, so
            // `payload` is metadata-only regardless.
            log::info!(
                "[agent-tracing] {} spans (trace_id={}) — set observability.agent_tracing.export_path to persist",
                spans.len(),
                spans.first().map(|s| s.trace_id.as_str()).unwrap_or(""),
            );
            log::debug!(
                target: "agent-tracing",
                "[agent-tracing] span NDJSON ({} spans):\n{}",
                spans.len(),
                payload.trim_end()
            );
        }
    }
}

/// Hand a completed run's spans to the configured tracing sink(s).
///
/// Two independent paths, both best-effort and never fatal to a turn:
///
/// 1. **Usage-data sharing** (`observability.share_usage_data`, on by default):
///    push the run's spans to the backend Langfuse proxy — endpoint derived from
///    the current backend host, authed with the session bearer (see
///    [`otlp::push_spans`]). A failure (no live session, network, rejected
///    payload) just logs; there is no local fallback, since sharing and local
///    export are distinct opt-ins.
/// 2. **Local exporter** (`observability.agent_tracing.enabled`, opt-in): append
///    OTel/Langfuse-format NDJSON to the configured file or the app log via
///    [`export_spans`].
///
/// A no-op when there are no spans or both paths are off.
pub(crate) async fn export_run_trace(config: &Config, spans: &[TraceSpan]) {
    if spans.is_empty() {
        return;
    }
    let observability = &config.observability;

    if observability.share_usage_data {
        if let Err(err) = otlp::push_spans(config, spans).await {
            log::warn!("[agent-tracing] Langfuse usage-data push failed ({err})");
        }
    }

    if observability.agent_tracing.enabled {
        export_spans(&observability.agent_tracing, spans);
    }
}

/// Export a completed run when durable tinyagents observations are available.
/// Remote usage-data sharing exports the live span tree as OTLP. The journal
/// is still read for local parity checks and future recovery work.
pub(crate) async fn export_run_trace_from_journal(
    ctx: &crate::core::runtime::DispatchCtx,
    config: &Config,
    request_id: &str,
    trace_ctx: TraceContext,
) {
    let observability = &config.observability;
    if !observability.share_usage_data && !observability.agent_tracing.enabled {
        return;
    }
    let Some(root_run_id) =
        crate::agent::tinyagents::journal::take_request_journal_run(ctx, request_id)
    else {
        log::debug!("[agent-tracing] no durable root run registered request_id={request_id}");
        return;
    };
    let observations = match crate::agent::tinyagents::journal::read_run_tree_events(
        ctx,
        &root_run_id,
    )
    .await
    {
        Ok(observations) if !observations.is_empty() => observations,
        Ok(_) => return,
        Err(err) => {
            log::warn!("[agent-tracing] trace journal read failed run_id={root_run_id}: {err}");
            return;
        }
    };
    let Some(root) = observations.iter().find(|observation| {
        observation.run_id.as_str() == root_run_id && observation.parent_run_id.is_none()
    }) else {
        log::warn!("[agent-tracing] trace root observation missing run_id={root_run_id}");
        return;
    };
    let trace_ctx = trace_ctx.with_run_lineage(
        Some(root_run_id.clone()),
        None,
        Some(root.root_run_id.as_str().to_owned()),
    );
    let mut collector = tinyagents_harness::observability::trace_export::collector::SpanCollector::new(
        trace_ctx,
    );
    for observation in &observations {
        collector.record(observation);
    }
    collector.finish(crate::agent::progress_tracing::unix_epoch_ms());
    let spans = collector.spans().to_vec();
    if spans.is_empty() {
        return;
    }
    if observability.share_usage_data {
        if let Err(err) = otlp::push_spans(config, &spans).await {
            log::warn!("[agent-tracing] Langfuse OTLP push failed ({err})");
        }
    }
    if observability.agent_tracing.enabled {
        export_spans(&observability.agent_tracing, &spans);
    }
}
