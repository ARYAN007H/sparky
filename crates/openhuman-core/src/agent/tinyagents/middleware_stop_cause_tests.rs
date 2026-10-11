//! The stop summary a user reads must name the breaker that actually stopped
//! the turn. These drive real tool-result sequences through the breakers the
//! harness installs, then render the summary from whatever stop note they
//! left, the way the session driver does.

use super::*;
use crate::agent::session_host::stop_summary::{render_stop_summary, StopReasonKind};
use crate::agent::session_host::turn_checkpoint::CheckpointToolResult;
use tinyagents_harness::middleware::library::{
    RepeatProgressMiddleware, REPEAT_GUARD_BLOCKED, REPEAT_GUARD_HALTED, REPEAT_GUARD_METADATA_KEY,
};
use tinyinference_llm::message::{AssistantMessage, ContentBlock};
use tinyinference_llm::model::ModelResponse;

const READ_ONLY_REFUSAL: &str = "[policy-blocked] Tool 'shell' was blocked by the security \
     policy. Reason: Security policy: read-only mode — only read commands are allowed";

const MISTYPED_PATH: &str = "Failed to resolve path '/work/src/lbi.rs': No such file or \
     directory (os error 2)";

/// The stop note the breakers left, rendered as the user-facing summary.
fn summary_from(
    slot: &std::sync::Arc<std::sync::Mutex<Option<String>>>,
    records: &[CheckpointToolResult],
) -> (StopReasonKind, String) {
    let note = slot
        .lock()
        .unwrap()
        .clone()
        .expect("a breaker stopped the turn");
    (
        StopReasonKind::from_stop_note(&note),
        render_stop_summary(records, &note),
    )
}

fn record(name: &str, result: &TaToolResult) -> CheckpointToolResult {
    CheckpointToolResult {
        name: name.to_string(),
        success: !result.is_error,
        content: result_text(result),
    }
}

/// A read-only seat: two refused `shell` calls among 35 successful reads.
/// The refusals' own ledger stops the turn, and the summary must say the
/// security settings blocked a step, not that the tools got nowhere.
#[tokio::test]
async fn a_read_only_refusal_halt_is_summarised_as_a_security_block() {
    let handle = SteeringHandle::allow_all();
    let slot = std::sync::Arc::new(std::sync::Mutex::new(None));
    let mw = RepeatedToolFailureMiddleware::new(handle.clone(), 3, slot.clone());
    let mut ctx = ctx();
    let mut records = Vec::new();
    for i in 0..37 {
        let (tool, args, result) = match i {
            12 => (
                "shell",
                json!({"command": "cargo build"}),
                failing_result("shell", READ_ONLY_REFUSAL),
            ),
            30 => (
                "shell",
                json!({"command": "touch notes.md"}),
                failing_result("shell", READ_ONLY_REFUSAL),
            ),
            i if i % 2 == 0 => (
                "file_read",
                json!({"path": format!("src/module_{i}.rs")}),
                tool_result("file_read", &format!("contents of module {i}")),
            ),
            i => (
                "grep",
                json!({"pattern": format!("fn handler_{i}")}),
                tool_result("grep", &format!("src/module_{i}.rs:3: fn handler_{i}()")),
            ),
        };
        let id = format!("call-{i}");
        let mut call = TaToolCall::new(&id, tool, args);
        mw.before_tool(&mut ctx, &(), &mut call).await.unwrap();
        let mut result = result;
        mw.after_tool(&mut ctx, &(), &invocation(&id, tool), &mut result)
            .await
            .unwrap();
        records.push(record(tool, &result));
    }
    assert_eq!(drain_pause_count(&handle), 1, "the second refusal stops");

    let (kind, summary) = summary_from(&slot, &records);
    assert_eq!(kind, StopReasonKind::Policy, "{summary}");
    assert!(
        summary.contains("blocked by your security settings"),
        "{summary}"
    );
    assert!(!summary.contains("not getting anywhere"), "{summary}");
}

/// One model response asking for `calls`, as the harness hands it to
/// `after_model`.
fn response_with(calls: &[TaToolCall]) -> ModelResponse {
    let mut response = ModelResponse::assistant("re-reading");
    response.message = AssistantMessage {
        id: None,
        content: vec![ContentBlock::Text("re-reading".to_string())],
        tool_calls: calls.to_vec(),
        usage: None,
        origin: None,
    };
    response.finish_reason = Some("tool_calls".to_string());
    response
}

/// A seat that keeps re-reading the same files, one of them through a
/// mistyped path. The repeat guard blocks the identical reads and then stops
/// the run while it admits a batch; the mistyped read in that same batch
/// fails again afterwards. The guard stopped the turn, so the summary must
/// say the agent kept repeating a step, not that something was missing.
#[tokio::test]
async fn a_repeat_guard_halt_is_not_overwritten_by_a_later_failure_in_its_batch() {
    let handle = SteeringHandle::allow_all();
    let slot = std::sync::Arc::new(std::sync::Mutex::new(None));
    // Assembled as `harness_assembly` does: the guard is registered first
    // (outermost), the failure breaker after it, both on one stop-note slot.
    let guard = RepeatProgressMiddleware::new(
        handle.clone(),
        slot.clone(),
        Arc::new(is_repeat_call_exempt),
    )
    .with_read_only(Arc::new(|name: &str| name == "file_read"))
    .with_cleared_placeholder(CLEARED_PLACEHOLDER);
    let breaker = RepeatedToolFailureMiddleware::new(handle.clone(), 3, slot.clone());
    let mut ctx = ctx();
    let mut records = Vec::new();
    let paths = ["src/lib.rs", "src/main.rs", "src/lbi.rs"];
    let mut halted = false;
    for round in 0..12 {
        let mut calls: Vec<TaToolCall> = paths
            .iter()
            .enumerate()
            .map(|(n, path)| {
                TaToolCall::new(
                    format!("r{round}-{n}"),
                    "file_read",
                    json!({ "path": path }),
                )
            })
            .collect();
        guard
            .after_model(&mut ctx, &(), &mut response_with(&calls))
            .await
            .unwrap();
        // Admission: the whole read-only batch is admitted before any result
        // comes back (concurrent execution). A call the guard refuses never
        // runs and is answered with the refusal, stamped with its marker.
        let mut results = Vec::new();
        for call in &mut calls {
            let result = match guard.before_tool(&mut ctx, &(), call).await {
                Err(refusal) => {
                    let text = refusal.to_string();
                    let marker = if text.contains("Stopping") {
                        REPEAT_GUARD_HALTED
                    } else {
                        REPEAT_GUARD_BLOCKED
                    };
                    let mut result = TaToolResult::error(text);
                    result.metadata = Some(json!({ REPEAT_GUARD_METADATA_KEY: marker }));
                    result
                }
                Ok(()) => {
                    breaker.before_tool(&mut ctx, &(), call).await.unwrap();
                    let path = call.arguments["path"].as_str().unwrap();
                    if path == "src/lbi.rs" {
                        failing_result("file_read", MISTYPED_PATH)
                    } else {
                        tool_result("file_read", &format!("contents of {path}"))
                    }
                }
            };
            results.push(result);
        }
        // Results fold back in call order; `after_tool` runs innermost first.
        for (call, mut result) in calls.iter().zip(results) {
            let id = invocation(&call.id, "file_read");
            breaker
                .after_tool(&mut ctx, &(), &id, &mut result)
                .await
                .unwrap();
            guard
                .after_tool(&mut ctx, &(), &id, &mut result)
                .await
                .unwrap();
            records.push(record("file_read", &result));
        }
        if drain_pause_count(&handle) > 0 {
            halted = true;
            break;
        }
    }
    assert!(halted, "the repeated reads stop the run");

    let (kind, summary) = summary_from(&slot, &records);
    assert_eq!(kind, StopReasonKind::RepeatingStep, "{summary}");
    assert!(
        summary.contains("kept repeating the same step"),
        "{summary}"
    );
    assert!(!summary.contains("could not be found"), "{summary}");
}

/// The failure breaker owns its stop when it fired first: a repeat-guard
/// halt later in the same batch must not replace its note.
#[tokio::test]
async fn a_failure_breaker_halt_survives_a_later_halt_in_its_batch() {
    let handle = SteeringHandle::allow_all();
    let slot = std::sync::Arc::new(std::sync::Mutex::new(None));
    let breaker = RepeatedToolFailureMiddleware::new(handle.clone(), 3, slot.clone());
    let mut ctx = ctx();
    for (i, command) in ["cargo build", "touch notes.md"].into_iter().enumerate() {
        let id = format!("refused-{i}");
        let mut call = TaToolCall::new(&id, "shell", json!({ "command": command }));
        breaker.before_tool(&mut ctx, &(), &mut call).await.unwrap();
        let mut result = failing_result("shell", READ_ONLY_REFUSAL);
        breaker
            .after_tool(&mut ctx, &(), &invocation(&id, "shell"), &mut result)
            .await
            .unwrap();
    }
    // A sibling result in the same batch trips the repeat guard, which
    // writes its own note into the shared slot.
    *slot.lock().unwrap() = Some(
        "Stopping: the same successful tool call returned the identical result 4 times in this \
         run; re-running steps whose results are already in the conversation adds no new \
         information, so the run is cycling without making progress."
            .to_string(),
    );
    breaker
        .after_agent(&mut ctx, &(), &mut AgentRun::new())
        .await
        .unwrap();
    let note = slot.lock().unwrap().clone().unwrap();
    assert_eq!(
        StopReasonKind::from_stop_note(&note),
        StopReasonKind::Policy,
        "{note}"
    );
}
