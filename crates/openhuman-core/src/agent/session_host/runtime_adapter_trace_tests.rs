use std::sync::Arc;

use async_trait::async_trait;
use tinyinference_llm::model::{ChatModel, ModelProfile, ModelRequest, ModelResponse};

use super::*;

struct GatedRepairModel {
    calls: std::sync::atomic::AtomicUsize,
    repair_started: tokio::sync::Notify,
}

#[async_trait]
impl ChatModel<()> for GatedRepairModel {
    fn profile(&self) -> Option<&ModelProfile> {
        static PROFILE: std::sync::LazyLock<ModelProfile> =
            std::sync::LazyLock::new(|| ModelProfile {
                provider: Some("openai".into()),
                tool_calling: true,
                ..ModelProfile::default()
            });
        Some(&PROFILE)
    }

    async fn invoke(
        &self,
        _state: &(),
        _request: ModelRequest,
    ) -> tinyinference_llm::Result<ModelResponse> {
        if self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
            return Ok(ModelResponse::assistant(
                "The answer omits its required block.",
            ));
        }
        self.repair_started.notify_one();
        std::future::pending().await
    }
}

/// Cancellation after harness completion must finalize the durable run as
/// cancelled once Tiny has saved the partial transcript.
#[tokio::test]
async fn cancellation_during_required_output_repair_finalizes_native_run() {
    use crate::storage::sessions::ExecutionStatus;
    use tinyagents_harness::events::AgentEvent;
    use tinyagents_harness::terminal::TerminalReason;

    let scratch = tempfile::tempdir().expect("isolated workspace");
    let workspace = scratch.path().join("workspace");
    let mut config = crate::config::Config::default();
    config.workspace_dir = workspace.clone();
    config.agent.required_output = Some(crate::config::RequiredOutputContract::new("answer"));
    config.components.juice = false;
    let core = crate::core::runtime::CoreContext::for_test_with_config(
        crate::core::runtime::DomainSet::full(),
        config.clone(),
    );
    let model = Arc::new(GatedRepairModel {
        calls: std::sync::atomic::AtomicUsize::new(0),
        repair_started: tokio::sync::Notify::new(),
    });
    let chat_model: Arc<dyn ChatModel<()>> = model.clone();
    let mut host = crate::agent::SessionHostBuilder::new()
        .chat_model_with_config(chat_model, Arc::new(config.clone()))
        .config(config.agent.clone())
        .tools(Vec::new())
        .workspace_dir(workspace.clone())
        .action_dir(scratch.path().to_path_buf())
        .tool_dispatcher(Box::new(tinytools_agent::dialect::NativeDialect))
        .with_session_history_locator(
            crate::storage::sessions::splice_fixture_tests::memory_locator(),
        )
        .build()
        .expect("host session");
    let thread_id = "trace-terminal-cancel-repair";
    let request_id = uuid::Uuid::new_v4().to_string();
    host.set_thread_id(Some(thread_id));
    let cancellation = tinyagents_harness::cancel::CancellationToken::new();
    host.set_host_env(crate::agent::turn_env::HostTurnEnv {
        cancellation: Some(cancellation.clone()),
        core: Some(Arc::downgrade(&core)),
        ..Default::default()
    });

    let origin = crate::security::turn_origin::AgentTurnOrigin::WebChat {
        thread_id: thread_id.into(),
        client_id: "trace-terminal-test".into(),
        request_id: Some(request_id.clone()),
    };
    let turn =
        tokio::spawn(async move { host.run_single_with_origin("hello", Some(origin)).await });
    model.repair_started.notified().await;
    cancellation.cancel();
    assert!(turn.await.expect("turn task").is_err());

    let dispatch = crate::core::runtime::DispatchCtx::new(Arc::clone(&core));
    let run_id =
        crate::agent::tinyagents::journal::take_request_journal_run(&dispatch, &request_id)
            .expect("the web-chat request maps to its actual native run");
    let observations = crate::agent::tinyagents::journal::read_run_events(&dispatch, &run_id, 0)
        .await
        .expect("durable native event stream");
    let failure = observations
        .iter()
        .position(|record| {
            matches!(
                &record.event,
                AgentEvent::RunFailed { run_id: failed, outcome: Some(outcome), .. }
                    if failed.as_str() == run_id && outcome.reason == TerminalReason::Cancelled
            )
        })
        .expect("the durable run keeps its actual cancellation outcome");
    let finalized = observations
        .iter()
        .position(|record| {
            matches!(
                &record.event,
                AgentEvent::RunFinalized { run_id: finalized } if finalized.as_str() == run_id
            )
        })
        .expect("the native run is finalized");
    assert!(finalized > failure, "finalization follows cancellation");
    assert!(observations.iter().any(|record| matches!(
        &record.event,
        AgentEvent::RunCompleted { run_id: completed, .. } if completed.as_str() == run_id
    )));
    let status = crate::storage::sessions::status_store_for(&workspace)
        .get_status(&run_id)
        .await
        .expect("status lookup")
        .expect("run status persisted");
    assert_eq!(status.status, ExecutionStatus::Cancelled);
}
