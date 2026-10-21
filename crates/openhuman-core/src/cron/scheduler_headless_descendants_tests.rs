use super::*;
use async_trait::async_trait;
use tinyinference_llm::model::{ChatModel, ModelProfile, ModelRequest, ModelResponse};

const CHILD_PROMPT: &str = "headless-cron-child-result-marker";

struct CronChildModel {
    spawn_calls: std::sync::atomic::AtomicUsize,
    child_calls: std::sync::atomic::AtomicUsize,
    child_started: std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    child_release: tokio::sync::Notify,
}

#[async_trait]
impl ChatModel<()> for CronChildModel {
    fn profile(&self) -> Option<&ModelProfile> {
        static PROFILE: std::sync::OnceLock<ModelProfile> = std::sync::OnceLock::new();
        Some(PROFILE.get_or_init(|| ModelProfile {
            tool_calling: true,
            parallel_tool_calls: true,
            ..ModelProfile::default()
        }))
    }

    async fn invoke(
        &self,
        _state: &(),
        request: ModelRequest,
    ) -> tinyinference_llm::Result<ModelResponse> {
        if request
            .messages
            .iter()
            .any(|message| message.text().contains(CHILD_PROMPT))
        {
            self.child_calls
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if let Some(started) = self.child_started.lock().unwrap().take() {
                let _ = started.send(());
            }
            self.child_release.notified().await;
            return Ok(ModelResponse::assistant("headless-cron-child-result"));
        }

        if self
            .spawn_calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            > 0
        {
            return Ok(ModelResponse::assistant("headless-cron-root-result"));
        }
        let mut response = ModelResponse::assistant("");
        response.message.tool_calls = vec![tinyinference_llm::tool::ToolCall::new(
            "headless-cron-spawn",
            "spawn_subagent",
            serde_json::json!({
                "agent_id": "task_manager_agent",
                "prompt": CHILD_PROMPT,
                "task_key": "headless-cron-child-task",
                "context": "headless cron regression",
                "model": "test-model"
            }),
        )];
        response.finish_reason = Some("tool_calls".into());
        Ok(response)
    }
}

#[tokio::test]
async fn scheduled_agent_history_includes_its_spawned_child_result() {
    let _inference = crate::inference::inference_test_guard_async().await;
    let model = std::sync::Arc::new(CronChildModel {
        spawn_calls: std::sync::atomic::AtomicUsize::new(0),
        child_calls: std::sync::atomic::AtomicUsize::new(0),
        child_started: std::sync::Mutex::new(None),
        child_release: tokio::sync::Notify::new(),
    });
    let _model =
        crate::inference::provider::factory::test_provider_override::install_model(model.clone());
    let temp = TempDir::new().expect("cron workspace");
    let mut config = Config::default();
    config.workspace_dir = temp.path().join("workspace");
    config.action_dir = config.workspace_dir.clone();
    config.default_model = Some("test-model".into());
    config.chat_provider = Some("omlx:test-model".into());
    std::fs::create_dir_all(&config.workspace_dir).expect("workspace");

    let definitions =
        Arc::new(crate::agent_registry::host_definition::AgentDefinitionRegistry::builtins_only());
    let core = crate::core::runtime::CoreContext::for_test_with_config(
        crate::core::runtime::DomainSet::full(),
        config.clone(),
    )
    .derive_with(
        crate::core::runtime::ContextOverlay::new(
            config.clone(),
            crate::core::runtime::DomainSet::full(),
            crate::tools::toolpacks::ToolGroups::none(),
        )
        .definitions(definitions),
    );
    let dispatch = crate::core::runtime::DispatchCtx::new(core);
    let mut job = test_job("");
    job.job_type = JobType::Agent;
    job.agent_id = Some("orchestrator".into());
    job.prompt = Some("Delegate this scheduled task to its worker.".into());
    job.name = Some("headless child test".into());

    let (child_started_tx, child_started_rx) = tokio::sync::oneshot::channel();
    *model.child_started.lock().unwrap() = Some(child_started_tx);
    let run = tokio::spawn(async move {
        super::super::agent_run::run_agent_job_for_run(&dispatch, &config, &job, "cron-run-test")
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), child_started_rx)
        .await
        .expect("the scheduled child reaches its model")
        .expect("child-start observation remains connected");
    model.child_release.notify_one();

    let (success, output, error) = run.await.expect("cron runner task");
    assert!(success, "cron run should succeed: {error:?}");
    assert!(
        output.contains("headless-cron-child-result"),
        "the recorded cron result must include its spawned child output: {output}"
    );
    assert_eq!(
        model.spawn_calls.load(std::sync::atomic::Ordering::SeqCst),
        1
    );
    assert_eq!(
        model.child_calls.load(std::sync::atomic::Ordering::SeqCst),
        1
    );
}
