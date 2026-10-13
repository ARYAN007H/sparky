//! A memory budget refusal is the host's own limit, not the tool failing:
//! a parallel batch of memory calls past the cap must not halt the run
//! (#7717).

use super::*;
use crate::memory::test_fixtures::{bind_reference, config_in};
use crate::memory::tools::{is_budget_refusal, MemoryTool};
use tinyagents_harness::ids::CallId;
use tinyagents_harness::tool::ToolExecutionContext;
use tinytools::ToolCallOptions;

/// Spends one run's read budget, then returns the refusals of `count`
/// further reads, as the memory tool answers them.
async fn budget_refusals(count: usize) -> Vec<TaToolResult> {
    let tmp = tempfile::tempdir().unwrap();
    let config = config_in(&tmp);
    let _engine = bind_reference(&config);
    let tool = MemoryTool::new(Arc::new(config));
    let run: RunContext = RunContext::new(RunConfig::new("budget-run"), ());
    let mut refusals = Vec::new();
    for i in 0..8 + count {
        let context = ToolExecutionContext::from_run_context(&run, CallId::new(format!("r{i}")));
        let result = tool
            .execute_with_context(
                json!({"action":"fetch","query":"plan"}),
                ToolCallOptions::default(),
                Some(&context),
            )
            .await
            .unwrap();
        if is_budget_refusal(&result) {
            refusals.push(result);
        }
    }
    assert_eq!(refusals.len(), count, "only the reads past the cap refuse");
    refusals
}

/// Feeds `results` to a fresh breaker as one batch of `memory` calls and
/// returns how many times it paused the run.
async fn pauses_for(results: Vec<TaToolResult>) -> usize {
    let handle = SteeringHandle::allow_all();
    let breaker = RepeatedToolFailureMiddleware::new(
        handle.clone(),
        3,
        std::sync::Arc::new(std::sync::Mutex::new(None)),
    );
    let mut ctx = ctx();
    for (i, mut result) in results.into_iter().enumerate() {
        breaker
            .after_tool(
                &mut ctx,
                &(),
                &invocation(format!("memory-{i}"), "memory"),
                &mut result,
            )
            .await
            .unwrap();
    }
    drain_pause_count(&handle)
}

#[tokio::test]
async fn memory_budget_refusals_do_not_halt_the_run() {
    let refusals = budget_refusals(9).await;
    assert_eq!(pauses_for(refusals).await, 0);
}

/// The control: the same refusals without the budget's marker are ordinary
/// failures and do halt, so the skip above is what keeps the run going.
#[tokio::test]
async fn the_same_refusals_unmarked_still_halt_the_run() {
    let unmarked = budget_refusals(9)
        .await
        .into_iter()
        .map(|mut result| {
            result.metadata = None;
            assert!(!is_budget_refusal(&result));
            result
        })
        .collect();
    assert!(pauses_for(unmarked).await > 0);
}

fn response_with(calls: &[TaToolCall]) -> tinyinference_llm::model::ModelResponse {
    use tinyinference_llm::message::{AssistantMessage, ContentBlock};
    let mut response = tinyinference_llm::model::ModelResponse::assistant("saving");
    response.message = AssistantMessage {
        id: None,
        content: vec![ContentBlock::Text("saving".to_string())],
        tool_calls: calls.to_vec(),
        usage: None,
        origin: None,
    };
    response.finish_reason = Some("tool_calls".to_string());
    response
}

/// The write churn #7293 capped (one save re-issued round after round) no
/// longer spends the memory budget, so the repeat guard installed by
/// `harness_assembly` is what stops it: identical learns acknowledge
/// identically, and the guard halts the run.
#[tokio::test]
async fn an_identical_learn_loop_is_halted_by_the_repeat_guard() {
    use tinyagents_harness::middleware::library::RepeatProgressMiddleware;
    let tmp = tempfile::tempdir().unwrap();
    let config = config_in(&tmp);
    let _engine = bind_reference(&config);
    let tool = MemoryTool::new(Arc::new(config));
    let run: RunContext = RunContext::new(RunConfig::new("churn-run"), ());
    let handle = SteeringHandle::allow_all();
    let guard = RepeatProgressMiddleware::new(
        handle.clone(),
        std::sync::Arc::new(std::sync::Mutex::new(None)),
        Arc::new(is_repeat_call_exempt),
    );
    let mut ctx = ctx();
    for round in 0..12 {
        let mut call = TaToolCall::new(
            format!("learn-{round}"),
            "memory",
            json!({"action":"learn","text":"The plan is saved"}),
        );
        guard
            .after_model(
                &mut ctx,
                &(),
                &mut response_with(std::slice::from_ref(&call)),
            )
            .await
            .unwrap();
        let mut result = match guard.before_tool(&mut ctx, &(), &mut call).await {
            Err(refusal) => TaToolResult::error(refusal.to_string()),
            Ok(()) => {
                let context = ToolExecutionContext::from_run_context(
                    &run,
                    CallId::new(format!("learn-{round}")),
                );
                let result = tool
                    .execute_with_context(
                        call.arguments.clone(),
                        ToolCallOptions::default(),
                        Some(&context),
                    )
                    .await
                    .unwrap();
                assert!(!is_budget_refusal(&result), "writes spend no budget");
                result
            }
        };
        guard
            .after_tool(&mut ctx, &(), &invocation(&call.id, "memory"), &mut result)
            .await
            .unwrap();
        if drain_pause_count(&handle) > 0 {
            return;
        }
    }
    panic!("an identical learn loop never stopped the run");
}
