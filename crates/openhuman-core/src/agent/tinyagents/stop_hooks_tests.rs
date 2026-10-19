use super::*;

struct StopImmediately;

#[async_trait::async_trait]
impl tinyagents_hooks::stop::StopHook for StopImmediately {
    fn name(&self) -> &str {
        "budget"
    }

    async fn check(
        &self,
        _state: &tinyagents_hooks::stop::TurnState<'_>,
    ) -> tinyagents_hooks::stop::StopDecision {
        tinyagents_hooks::stop::StopDecision::Stop {
            reason: "limit reached".into(),
        }
    }
}

#[tokio::test]
async fn a_poisoned_halt_summary_does_not_abort_stop_reporting() {
    use tinyagents_harness::middleware::Middleware as _;

    let slot: tinyagents_harness::turn_outcome::HaltSummarySlot =
        Arc::new(std::sync::Mutex::new(None));
    let poisoned = Arc::clone(&slot);
    assert!(std::thread::spawn(move || {
        let _guard = poisoned.lock().unwrap();
        panic!("synthetic prior writer failure");
    })
    .join()
    .is_err());

    let middleware = StopHookMiddleware::new("test-model", vec![Arc::new(StopImmediately)])
        .with_halt_summary(Arc::clone(&slot));
    let mut context = tinyagents_harness::context::RunContext::new(
        tinyagents_harness::context::RunConfig::new("stop-summary-test"),
        (),
    )
    .with_steering(tinyagents_harness::steering::SteeringHandle::allow_all());
    let mut response = tinyinference_llm::model::ModelResponse::assistant("done");
    middleware
        .after_model(&mut context, &(), &mut response)
        .await
        .unwrap();

    assert_eq!(
        slot.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_deref(),
        Some("Stopping after 1 model call(s): limit reached")
    );
}
