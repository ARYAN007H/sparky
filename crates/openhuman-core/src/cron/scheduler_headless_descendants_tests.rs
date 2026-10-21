use super::*;
use async_trait::async_trait;
use tinyinference_llm::model::{ChatModel, ModelProfile, ModelRequest, ModelResponse};

const CHILD_PROMPT: &str = "headless-cron-child-result-marker";

struct ChildInvocationDrop(std::sync::Arc<std::sync::atomic::AtomicBool>);

impl Drop for ChildInvocationDrop {
    fn drop(&mut self) {
        self.0.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

struct CronChildModel {
    spawn_calls: std::sync::atomic::AtomicUsize,
    child_calls: std::sync::atomic::AtomicUsize,
    child_invocation_dropped: std::sync::Arc<std::sync::atomic::AtomicBool>,
    accepted_followups: std::sync::atomic::AtomicUsize,
    child_started: std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    child_release: tokio::sync::Notify,
}

fn scheduled_agent_fixture(timeout_secs: u64) -> (
    TempDir,
    Config,
    std::sync::Arc<crate::core::runtime::CoreContext>,
    crate::core::runtime::DispatchCtx,
    tinyflows_bus::schedule::types::CronJob,
) {
    let temp = TempDir::new().expect("cron workspace");
    let mut config = Config::default();
    config.workspace_dir = temp.path().join("workspace");
    config.action_dir = config.workspace_dir.clone();
    config.default_model = Some("test-model".into());
    config.chat_provider = Some("omlx:test-model".into());
    config.agent.agent_timeout_secs = timeout_secs;
    config.reliability.scheduler_retries = 0;
    std::fs::create_dir_all(&config.workspace_dir).expect("workspace");

    let definitions = Arc::new(
        crate::agent_registry::host_definition::AgentDefinitionRegistry::builtins_only(),
    );
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
    let dispatch = crate::core::runtime::DispatchCtx::new(std::sync::Arc::clone(&core));
    let job = crate::cron::add_agent_job(
        &config,
        Some("headless child test".into()),
        tinyflows_bus::schedule::types::Schedule::Every {
            every_ms: 3_600_000,
        },
        "Delegate this scheduled task to its worker.",
        SessionTarget::Isolated,
        Some("test-model".into()),
        None,
        false,
    )
    .expect("persist scheduled agent job");
    crate::cron::store::force_next_run_for_test(
        &config,
        &job.id,
        chrono::Utc::now() - chrono::Duration::seconds(1),
    )
    .expect("make the job due");
    (temp, config, core, dispatch, job)
}

async fn dispatch_scheduled_agent(
    core: std::sync::Arc<crate::core::runtime::CoreContext>,
    dispatch: crate::core::runtime::DispatchCtx,
    config: Config,
) {
    let security = Arc::new(SecurityPolicy::from_config(
        &config.autonomy,
        &config.workspace_dir,
        &config.action_dir,
    ));
    let mut dispatcher = JobDispatcher::new(&dispatch, &config, &security);
    crate::core::runtime::CoreContext::scope(
        core,
        Box::pin(async move {
            let dispatched = dispatcher.tick(&config).await.expect("scheduler tick");
            assert_eq!(dispatched, 1, "the persisted due job is dispatched");
            dispatcher.drain().await;
        }),
    )
    .await;
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
            let _drop =
                ChildInvocationDrop(std::sync::Arc::clone(&self.child_invocation_dropped));
            self.child_release.notified().await;
            return Ok(ModelResponse::assistant("headless-cron-child-result"));
        }

        if request.messages.iter().any(|message| {
            message
                .text()
                .contains("Accepted async sub-agent `task_manager_agent`")
        }) {
            self.accepted_followups
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            return Ok(ModelResponse::assistant("headless-cron-root-result"));
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
        child_invocation_dropped: std::sync::Arc::new(
            std::sync::atomic::AtomicBool::new(false),
        ),
        accepted_followups: std::sync::atomic::AtomicUsize::new(0),
        child_started: std::sync::Mutex::new(None),
        child_release: tokio::sync::Notify::new(),
    });
    let _model =
        crate::inference::provider::factory::test_provider_override::install_model(model.clone());
    let (_temp, config, core, dispatch, job) = scheduled_agent_fixture(900);

    let (child_started_tx, child_started_rx) = tokio::sync::oneshot::channel();
    *model.child_started.lock().unwrap() = Some(child_started_tx);
    let scheduler_config = config.clone();
    let run = tokio::spawn(async move {
        dispatch_scheduled_agent(core, dispatch, scheduler_config).await
    });
    match tokio::time::timeout(std::time::Duration::from_secs(5), child_started_rx).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => panic!("child-start observation disconnected: {error}"),
        Err(_) => {
            model.child_release.notify_one();
            let result = tokio::time::timeout(std::time::Duration::from_secs(5), run).await;
            panic!(
            "scheduled child never reached its model (spawn calls: {}, followups: {}); run result: {result:?}",
            model.spawn_calls.load(std::sync::atomic::Ordering::SeqCst),
            model.accepted_followups.load(std::sync::atomic::Ordering::SeqCst),
            );
        }
    }
    model.child_release.notify_one();

    run.await.expect("cron scheduler task");
    let runs = crate::cron::list_runs(&config, &job.id, 10).expect("read persisted run history");
    assert_eq!(runs.len(), 1, "scheduler persisted one completed run");
    assert_eq!(runs[0].status, "ok", "agent job completed successfully");
    assert!(
        runs[0]
            .output
            .as_deref()
            .unwrap_or_default()
            .contains("headless-cron-child-result"),
        "the persisted cron result must include its spawned child output: {}",
        runs[0].output.as_deref().unwrap_or_default()
    );
    assert_eq!(
        model.spawn_calls.load(std::sync::atomic::Ordering::SeqCst),
        1
    );
    assert_eq!(model.child_calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(
        model.accepted_followups.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "the parent accepted the real spawn result before the child completed"
    );
}
