use super::journal_projection::spans_from_observations;
use super::{SpanKind, SpanStatus, TraceContext};
use tinyagents_harness::events::AgentEvent;
use tinyagents_harness::ids::{CallId, EventId, RunId};
use tinyagents_harness::observability::AgentObservation;
use tinyinference_llm::usage::Usage;

/// Wraps an event as a journalled observation stamped at `ts`.
fn obs(offset: u64, ts: u64, event: AgentEvent) -> AgentObservation {
    AgentObservation {
        event_id: EventId::new(format!("run-1-evt-{offset}")),
        run_id: RunId::new("run-1"),
        parent_run_id: None,
        root_run_id: RunId::new("run-1"),
        offset,
        ts_ms: ts,
        event,
    }
}

fn tool_completed(call: &str, name: &str, error: Option<&str>) -> AgentEvent {
    AgentEvent::ToolCompleted {
        parent_call_id: None,
        call_id: CallId::new(call),
        tool_name: name.to_string(),
        started_at_ms: Some(1_020),
        input: None,
        output: None,
        duration_ms: Some(30),
        output_bytes: Some(12),
        error: error.map(str::to_string),
        metadata: None,
    }
}

fn single_turn(tool_error: Option<&str>) -> Vec<AgentObservation> {
    vec![
        obs(
            0,
            1_000,
            AgentEvent::RunStarted {
                run_id: RunId::new("run-1"),
                thread_id: None,
            },
        ),
        obs(
            1,
            1_010,
            AgentEvent::ModelStarted {
                call_id: CallId::new("c1"),
                model: "gpt-4".to_string(),
            },
        ),
        obs(
            2,
            1_020,
            AgentEvent::ToolStarted {
                parent_call_id: None,
                call_id: CallId::new("t1"),
                tool_name: "lookup".to_string(),
                input: None,
            },
        ),
        obs(3, 1_050, tool_completed("t1", "lookup", tool_error)),
        obs(
            4,
            1_060,
            AgentEvent::ModelCompleted {
                call_id: CallId::new("c1"),
                started_at_ms: Some(1_010),
                usage: Some(Usage::new(100, 20)),
                input: None,
                output: None,
            },
        ),
        obs(
            5,
            1_070,
            AgentEvent::RunCompleted {
                run_id: RunId::new("run-1"),
                outcome: None,
            },
        ),
    ]
}

fn subagent_turn() -> Vec<AgentObservation> {
    vec![
        obs(
            0,
            1_000,
            AgentEvent::RunStarted {
                run_id: RunId::new("run-1"),
                thread_id: None,
            },
        ),
        obs(
            1,
            1_010,
            AgentEvent::SubAgentStarted {
                name: "researcher".to_string(),
                depth: 1,
            },
        ),
        obs(
            2,
            1_020,
            AgentEvent::ModelStarted {
                call_id: CallId::new("scout-model"),
                model: "gpt-4".to_string(),
            },
        ),
        obs(
            3,
            1_090,
            AgentEvent::ToolStarted {
                parent_call_id: None,
                call_id: CallId::new("scout-tool"),
                tool_name: "read_file".to_string(),
                input: None,
            },
        ),
        obs(4, 1_060, tool_completed("scout-tool", "read_file", None)),
        obs(
            5,
            1_070,
            AgentEvent::ModelCompleted {
                call_id: CallId::new("scout-model"),
                started_at_ms: Some(1_020),
                usage: Some(Usage::new(11, 7)),
                input: None,
                output: None,
            },
        ),
        obs(
            6,
            1_090,
            AgentEvent::SubAgentCompleted {
                name: "researcher".to_string(),
                depth: 1,
            },
        ),
        obs(
            7,
            1_100,
            AgentEvent::RunCompleted {
                run_id: RunId::new("run-1"),
                outcome: None,
            },
        ),
    ]
}

fn ctx() -> TraceContext {
    TraceContext::new("session-1", Some("user-1".into()))
}

/// A stable projection snapshot for the collector migration. Keep this fixture
/// at the export boundary so the native owner collector can be compared with
/// the current host projection before that projection is removed.
#[test]
fn legacy_export_snapshot_for_root_and_child_with_reused_call_ids() {
    use tinyagents_harness::observability::trace_export::serialize::{
        SpanEnvelope, spans_to_ndjson,
    };

    let mut observations = vec![
        obs(
            0,
            1_000,
            AgentEvent::RunStarted {
                run_id: RunId::new("run-1"),
                thread_id: None,
            },
        ),
        obs(
            1,
            1_010,
            AgentEvent::ModelStarted {
                call_id: CallId::new("same-call"),
                model: "gpt-4".to_string(),
            },
        ),
        obs(
            2,
            1_020,
            AgentEvent::ToolStarted {
                parent_call_id: None,
                call_id: CallId::new("tool-1"),
                tool_name: "lookup".to_string(),
                input: Some(serde_json::json!({"query":"CONTENT_SENTINEL_INPUT"})),
            },
        ),
        obs(
            3,
            1_030,
            AgentEvent::SubAgentStarted {
                name: "researcher".to_string(),
                depth: 1,
            },
        ),
    ];
    observations.push(AgentObservation {
        event_id: EventId::new("child-evt-0"),
        run_id: RunId::new("child-run"),
        parent_run_id: Some(RunId::new("run-1")),
        root_run_id: RunId::new("run-1"),
        offset: 0,
        ts_ms: 1_040,
        event: AgentEvent::RunStarted {
            run_id: RunId::new("child-run"),
            thread_id: None,
        },
    });
    observations.push(AgentObservation {
        event_id: EventId::new("child-evt-1"),
        run_id: RunId::new("child-run"),
        parent_run_id: Some(RunId::new("run-1")),
        root_run_id: RunId::new("run-1"),
        offset: 1,
        ts_ms: 1_050,
        event: AgentEvent::ModelStarted {
            call_id: CallId::new("same-call"),
            model: "gpt-4-mini".to_string(),
        },
    });
    observations.push(AgentObservation {
        event_id: EventId::new("child-evt-2"),
        run_id: RunId::new("child-run"),
        parent_run_id: Some(RunId::new("run-1")),
        root_run_id: RunId::new("run-1"),
        offset: 2,
        ts_ms: 1_060,
        event: AgentEvent::ModelCompleted {
            call_id: CallId::new("same-call"),
            started_at_ms: Some(1_050),
            usage: Some(Usage::new(7, 3)),
            input: None,
            output: None,
        },
    });
    observations.push(AgentObservation {
        event_id: EventId::new("child-evt-3"),
        run_id: RunId::new("child-run"),
        parent_run_id: Some(RunId::new("run-1")),
        root_run_id: RunId::new("run-1"),
        offset: 3,
        ts_ms: 1_070,
        event: AgentEvent::RunCompleted {
            run_id: RunId::new("child-run"),
            outcome: None,
        },
    });
    observations.push(obs(
        4,
        1_080,
        AgentEvent::SubAgentCompleted {
            name: "researcher".to_string(),
            depth: 1,
        },
    ));
    observations.push(obs(
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
    ));
    observations.push(obs(
        6,
        1_100,
        AgentEvent::ModelCompleted {
            call_id: CallId::new("same-call"),
            started_at_ms: Some(1_010),
            usage: Some(Usage::new(100, 20)),
            input: Some(serde_json::json!(["CONTENT_SENTINEL_PROMPT"])),
            output: Some(serde_json::json!(["CONTENT_SENTINEL_REPLY"])),
        },
    ));
    observations.push(obs(
        7,
        1_110,
        AgentEvent::RunCompleted {
            run_id: RunId::new("run-1"),
            outcome: None,
        },
    ));
    observations.push(obs(
        6,
        1_080,
        AgentEvent::SubAgentCompleted {
            name: "researcher".to_string(),
            depth: 1,
        },
    ));
    observations.push(obs(
        7,
        1_110,
        AgentEvent::RunCompleted {
            run_id: RunId::new("run-1"),
            outcome: None,
        },
    ));

    let spans = spans_from_observations(ctx(), 10, &observations);
    let snapshot = spans_to_ndjson(SpanEnvelope::Otel, &spans);
    assert!(
        !snapshot.contains("CONTENT_SENTINEL"),
        "content-off export must not contain captured payloads"
    );
    let mut native = tinyagents_harness::observability::trace_export::SpanCollector::new(ctx());
    for observation in &observations {
        native.record(observation);
    }
    native.finish(1_110);
    let native_spans = native.spans();
    let native_by_id: std::collections::HashMap<_, _> = native_spans
        .iter()
        .map(|span| (span.span_id.as_str(), span))
        .collect();
    let native_normalized: Vec<_> = native_spans
        .iter()
        .map(|span| {
            serde_json::json!({
                "name": span.name,
                "kind": format!("{:?}", span.kind),
                "parent": span.parent_span_id.as_deref().and_then(|id| native_by_id.get(id)).map(|parent| parent.name.as_str()),
                "start_unix_ms": span.start_unix_ms,
                "end_unix_ms": span.end_unix_ms,
                "status": format!("{:?}", span.status),
                "attributes": span.attributes,
            })
        })
        .collect();
    println!(
        "NATIVE:{}",
        serde_json::to_string_pretty(&native_normalized).unwrap()
    );
    let spans_by_id: std::collections::HashMap<_, _> = spans
        .iter()
        .map(|span| (span.span_id.as_str(), span))
        .collect();
    let normalized: Vec<_> = spans
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
        .collect();
    println!("NORMALIZED:{}", serde_json::to_string_pretty(&normalized).unwrap());
    let root = spans.iter().find(|span| span.parent_span_id.is_none()).unwrap();
    assert_eq!(root.start_unix_ms, 1_000);
    assert_eq!(root.end_unix_ms, Some(1_110));
    let generation_names: Vec<_> = spans
        .iter()
        .filter(|span| span.kind == SpanKind::Generation)
        .map(|span| span.name.as_str())
        .collect();
    assert_eq!(generation_names, ["llm.gpt-4", "llm.gpt-4-mini"]);
    assert_eq!(
        spans.iter().find(|span| span.name == "llm.gpt-4").unwrap().attributes["gen_ai.usage.input_tokens"],
        serde_json::json!(100)
    );
    assert_eq!(
        spans.iter().find(|span| span.name == "llm.gpt-4-mini").unwrap().attributes["gen_ai.usage.input_tokens"],
        serde_json::json!(7),
        "the repeated call id in a child run must retain its own usage"
    );
    let captured = spans_from_observations(ctx().with_capture_content(true), 10, &observations);
    assert!(
        spans_to_ndjson(SpanEnvelope::Otel, &captured).contains("CONTENT_SENTINEL_PROMPT"),
        "content-on export retains opted-in prompt content"
    );
    println!("{snapshot}");
}

#[test]
fn projects_single_agent_turn_span_tree() {
    let spans = spans_from_observations(ctx(), 10, &single_turn(None));

    // Turn + iteration + tool + generation spans are all present.
    let by_kind = |k: SpanKind| spans.iter().filter(|s| s.kind == k).count();
    assert_eq!(by_kind(SpanKind::Turn), 1, "one turn span");
    assert_eq!(by_kind(SpanKind::Iteration), 1, "one iteration span");
    assert_eq!(by_kind(SpanKind::Tool), 1, "one tool span");
    assert_eq!(by_kind(SpanKind::Generation), 1, "one generation span");

    let tool = spans.iter().find(|s| s.kind == SpanKind::Tool).unwrap();
    assert_eq!(tool.name, "tool.lookup");
    assert_eq!(tool.attributes["tool.success"], serde_json::json!(true));
    assert_eq!(tool.attributes["tool.output_chars"], serde_json::json!(12));
    assert_eq!(tool.attributes["tool.elapsed_ms"], serde_json::json!(30));

    let generation = spans
        .iter()
        .find(|s| s.kind == SpanKind::Generation)
        .unwrap();
    assert!(
        generation.name.contains("gpt-4"),
        "gen span names the model"
    );
}

#[test]
fn failed_tool_projects_error_outcome() {
    // With content capture on, a failed tool span carries the classified
    // cause reconstructed from the journalled error string, reproducing the
    // live path's `classify(error, false)` exactly.
    let spans = spans_from_observations(
        ctx().with_capture_content(true),
        10,
        &single_turn(Some("permission denied opening /etc/x")),
    );
    let tool = spans.iter().find(|s| s.kind == SpanKind::Tool).unwrap();
    assert_eq!(tool.attributes["tool.success"], serde_json::json!(false));
    assert!(
        tool.attributes.contains_key("error.message"),
        "failed tool span carries a classified error message"
    );
}

#[test]
fn projects_subagent_scope_from_lifecycle_brackets() {
    let spans = spans_from_observations(ctx(), 10, &subagent_turn());

    let by_kind = |k: SpanKind| spans.iter().filter(|s| s.kind == k).count();
    assert_eq!(by_kind(SpanKind::Subagent), 1, "one subagent span");
    assert_eq!(
        by_kind(SpanKind::SubagentIteration),
        1,
        "one child iteration span"
    );
    assert_eq!(by_kind(SpanKind::Tool), 1, "one child tool span");
    assert_eq!(by_kind(SpanKind::Generation), 1, "one child generation");

    let subagent = spans.iter().find(|s| s.kind == SpanKind::Subagent).unwrap();
    assert_eq!(
        subagent.attributes["subagent.agent_id"],
        serde_json::json!("researcher")
    );
    assert_eq!(
        subagent.attributes["subagent.iterations"],
        serde_json::json!(1)
    );

    let child_iteration = spans
        .iter()
        .find(|s| s.kind == SpanKind::SubagentIteration)
        .unwrap();
    assert_eq!(
        child_iteration.parent_span_id.as_deref(),
        Some(subagent.span_id.as_str())
    );
}

#[test]
fn projects_failed_subagent_from_child_run_failed() {
    let observations = vec![
        obs(
            0,
            1_000,
            AgentEvent::RunStarted {
                run_id: RunId::new("run-1"),
                thread_id: None,
            },
        ),
        obs(
            1,
            1_010,
            AgentEvent::SubAgentStarted {
                name: "researcher".to_string(),
                depth: 1,
            },
        ),
        obs(
            2,
            1_020,
            AgentEvent::RunFailed {
                run_id: RunId::new("run-1"),
                error: "provider unavailable".to_string(),
                outcome: None,
            },
        ),
        obs(
            3,
            1_030,
            AgentEvent::RunCompleted {
                run_id: RunId::new("run-1"),
                outcome: None,
            },
        ),
    ];

    let spans = spans_from_observations(ctx(), 10, &observations);
    let subagent = spans.iter().find(|s| s.kind == SpanKind::Subagent).unwrap();

    assert_eq!(subagent.status, SpanStatus::Error);
    assert_eq!(subagent.attributes["error"], serde_json::json!(true));
    assert!(
        subagent.attributes.contains_key("error.length"),
        "failed subagent span carries redacted error metadata"
    );
}

#[test]
fn projects_turn_content_from_root_model_io() {
    let observations = vec![
        obs(
            0,
            1_000,
            AgentEvent::RunStarted {
                run_id: RunId::new("run-1"),
                thread_id: None,
            },
        ),
        obs(
            1,
            1_010,
            AgentEvent::ModelStarted {
                call_id: CallId::new("m1"),
                model: "gpt-4".to_string(),
            },
        ),
        obs(
            2,
            1_020,
            AgentEvent::ModelCompleted {
                call_id: CallId::new("m1"),
                started_at_ms: Some(1_010),
                usage: Some(Usage::new(5, 3)),
                input: Some(serde_json::json!([
                    {"role": "user", "content": "summarize this"}
                ])),
                output: Some(serde_json::json!({
                    "role": "assistant",
                    "content": "short summary"
                })),
            },
        ),
        obs(
            3,
            1_030,
            AgentEvent::RunCompleted {
                run_id: RunId::new("run-1"),
                outcome: None,
            },
        ),
    ];
    let spans = spans_from_observations(ctx().with_capture_content(true), 10, &observations);
    let turn = spans.iter().find(|s| s.kind == SpanKind::Turn).unwrap();

    assert!(
        turn.input
            .as_ref()
            .unwrap()
            .to_string()
            .contains("summarize this"),
        "root model input is attached through TurnContent"
    );
    assert_eq!(
        turn.output.as_ref().unwrap(),
        &serde_json::json!("short summary")
    );
}

#[path = "journal_projection_accuracy_tests.rs"]
mod accuracy_tests;
#[path = "journal_projection_cost_rollup_tests.rs"]
mod cost_rollup_tests;
