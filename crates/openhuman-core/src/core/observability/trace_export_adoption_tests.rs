use super::{journal_projection::spans_from_observations, RunType, TraceContext};
use tinyagents_harness::events::AgentEvent;
use tinyagents_harness::ids::{CallId, EventId, RunId};
use tinyagents_harness::observability::trace_export::collector::SpanCollector as NativeSpanCollector;
use tinyagents_harness::observability::trace_export::serialize::{spans_to_ndjson, SpanEnvelope};
use tinyagents_harness::observability::trace_export::TraceSpan;
use tinyagents_harness::observability::AgentObservation;
use tinyinference_llm::usage::Usage;

fn root_observation(offset: u64, ts_ms: u64, event: AgentEvent) -> AgentObservation {
    AgentObservation {
        event_id: EventId::new(format!("root-evt-{offset}")),
        run_id: RunId::new("root-run"),
        parent_run_id: None,
        root_run_id: RunId::new("root-run"),
        offset,
        ts_ms,
        event,
    }
}

fn child_observation(offset: u64, ts_ms: u64, event: AgentEvent) -> AgentObservation {
    AgentObservation {
        event_id: EventId::new(format!("child-evt-{offset}")),
        run_id: RunId::new("child-run"),
        parent_run_id: Some(RunId::new("root-run")),
        root_run_id: RunId::new("root-run"),
        offset,
        ts_ms,
        event,
    }
}

fn parent_run() -> Vec<AgentObservation> {
    vec![
        root_observation(
            0,
            1_000,
            AgentEvent::RunStarted {
                run_id: RunId::new("root-run"),
                thread_id: None,
            },
        ),
        root_observation(
            1,
            1_010,
            AgentEvent::ModelStarted {
                call_id: CallId::new("same-call"),
                model: "gpt-4".to_string(),
            },
        ),
        root_observation(
            2,
            1_020,
            AgentEvent::ToolStarted {
                parent_call_id: None,
                call_id: CallId::new("tool-1"),
                tool_name: "lookup".to_string(),
                input: Some(serde_json::json!({"query":"CONTENT_SENTINEL_INPUT"})),
            },
        ),
        root_observation(
            3,
            1_030,
            AgentEvent::SubAgentStarted {
                name: "researcher".to_string(),
                depth: 1,
            },
        ),
        root_observation(
            4,
            1_080,
            AgentEvent::SubAgentCompleted {
                name: "researcher".to_string(),
                depth: 1,
            },
        ),
        root_observation(
            5,
            1_090,
            AgentEvent::ToolCompleted {
                parent_call_id: None,
                call_id: CallId::new("tool-1"),
                tool_name: "lookup".to_string(),
                started_at_ms: Some(1_020),
                input: Some(serde_json::json!({"query":"CONTENT_SENTINEL_INPUT"})),
                output: Some(serde_json::json!({"result":"CONTENT_SENTINEL_OUTPUT"})),
                duration_ms: Some(70),
                output_bytes: Some(33),
                error: None,
                metadata: None,
            },
        ),
        root_observation(
            6,
            1_100,
            AgentEvent::ModelCompleted {
                call_id: CallId::new("same-call"),
                started_at_ms: Some(1_010),
                usage: Some(Usage::new(100, 20)),
                input: Some(serde_json::json!(["CONTENT_SENTINEL_PROMPT"])),
                output: Some(serde_json::json!(["CONTENT_SENTINEL_REPLY"])),
            },
        ),
        root_observation(
            7,
            1_110,
            AgentEvent::RunCompleted {
                run_id: RunId::new("root-run"),
                outcome: None,
            },
        ),
    ]
}

fn child_run() -> Vec<AgentObservation> {
    vec![
        child_observation(
            0,
            1_040,
            AgentEvent::RunStarted {
                run_id: RunId::new("child-run"),
                thread_id: None,
            },
        ),
        child_observation(
            1,
            1_050,
            AgentEvent::ModelStarted {
                call_id: CallId::new("same-call"),
                model: "gpt-4-mini".to_string(),
            },
        ),
        child_observation(
            2,
            1_060,
            AgentEvent::ModelCompleted {
                call_id: CallId::new("same-call"),
                started_at_ms: Some(1_050),
                usage: Some(Usage::new(7, 3)),
                input: None,
                output: None,
            },
        ),
        child_observation(
            3,
            1_070,
            AgentEvent::RunCompleted {
                run_id: RunId::new("child-run"),
                outcome: None,
            },
        ),
    ]
}

fn normalized(spans: &[TraceSpan]) -> Vec<serde_json::Value> {
    let spans_by_id: std::collections::HashMap<_, _> = spans
        .iter()
        .map(|span| (span.span_id.as_str(), span))
        .collect();
    spans
        .iter()
        .map(|span| {
            serde_json::json!({
                "name": span.name,
                "kind": format!("{:?}", span.kind),
                "parent": span.parent_span_id.as_deref().and_then(|id| spans_by_id.get(id)).map(|parent| parent.name.as_str()),
                "start_unix_ms": span.start_unix_ms,
                "end_unix_ms": span.end_unix_ms,
                "status": format!("{:?}", span.status),
                "attributes": span.attributes,
            })
        })
        .collect()
}

#[test]
fn per_run_trace_snapshots_keep_reused_call_ids_scoped_and_content_gated() {
    let parent_ctx = TraceContext::new("trace-parent", Some("user-1".to_string()));
    let child_ctx = TraceContext::new("trace-child", Some("user-1".to_string()))
        .with_run_type(RunType::Subagent);
    let parent_observations = parent_run();
    let child_observations = child_run();

    let parent_legacy = spans_from_observations(parent_ctx.clone(), 10, &parent_observations);
    let rooted_child = tinyagents_harness::observability::trace_export::journal_export::root_subagent_observations(&child_observations);
    let child_legacy = spans_from_observations(child_ctx.clone(), 10, &rooted_child);
    let mut parent_native = NativeSpanCollector::new(parent_ctx.clone());
    for observation in &parent_observations {
        parent_native.record(observation);
    }
    parent_native.finish(1_110);
    let mut child_native = NativeSpanCollector::new(child_ctx.clone());
    for observation in &rooted_child {
        child_native.record(observation);
    }
    child_native.finish(1_070);

    let parent_legacy_snapshot = normalized(&parent_legacy);
    let child_legacy_snapshot = normalized(&child_legacy);
    let parent_native_snapshot = normalized(&parent_native.spans());
    let child_native_snapshot = normalized(&child_native.spans());
    println!("PARENT_LEGACY:{}", serde_json::to_string_pretty(&parent_legacy_snapshot).unwrap());
    println!("CHILD_LEGACY:{}", serde_json::to_string_pretty(&child_legacy_snapshot).unwrap());
    println!("PARENT_NATIVE:{}", serde_json::to_string_pretty(&parent_native_snapshot).unwrap());
    println!("CHILD_NATIVE:{}", serde_json::to_string_pretty(&child_native_snapshot).unwrap());

    assert_eq!(
        parent_legacy.iter().find(|span| span.kind == super::SpanKind::Generation).unwrap().name,
        "llm.gpt-4"
    );
    assert_eq!(
        child_legacy.iter().find(|span| span.kind == super::SpanKind::Generation).unwrap().name,
        "llm.gpt-4-mini"
    );
    assert!(
        !spans_to_ndjson(SpanEnvelope::Otel, &parent_native.spans()).contains("CONTENT_SENTINEL"),
        "content-off export must not contain parent payloads"
    );
    assert!(
        !spans_to_ndjson(SpanEnvelope::Otel, &child_native.spans()).contains("CONTENT_SENTINEL"),
        "content-off export must not contain child payloads"
    );
    let mut captured = NativeSpanCollector::new(parent_ctx.with_capture_content(true));
    for observation in &parent_observations {
        captured.record(observation);
    }
    captured.finish(1_110);
    assert!(
        spans_to_ndjson(SpanEnvelope::Otel, &captured.spans())
            .contains("CONTENT_SENTINEL_PROMPT"),
        "content-on export retains opted-in prompt content"
    );
    assert_eq!(
        parent_legacy.iter().find(|span| span.name == "llm.gpt-4").unwrap().attributes["gen_ai.usage.input_tokens"],
        serde_json::json!(100)
    );
    assert_eq!(
        child_legacy.iter().find(|span| span.name == "llm.gpt-4-mini").unwrap().attributes["gen_ai.usage.input_tokens"],
        serde_json::json!(7)
    );
}
