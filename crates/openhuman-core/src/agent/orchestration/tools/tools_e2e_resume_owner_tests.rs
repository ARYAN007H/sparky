use super::*;

struct PauseThenHoldResumeModel {
    calls: std::sync::atomic::AtomicUsize,
    resumed: tokio::sync::mpsc::UnboundedSender<()>,
    release: Arc<tokio::sync::Notify>,
}

#[async_trait]
impl ChatModel<()> for PauseThenHoldResumeModel {
    fn profile(&self) -> Option<&ModelProfile> {
        static PROFILE: std::sync::OnceLock<ModelProfile> = std::sync::OnceLock::new();
        Some(PROFILE.get_or_init(|| {
            let mut profile = ModelProfile::default();
            profile.tool_calling = true;
            profile
        }))
    }

    async fn invoke(
        &self,
        _state: &(),
        _request: ModelRequest,
    ) -> tinyinference_llm::Result<ModelResponse> {
        if self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
            let mut response = ModelResponse::assistant("");
            response.message.tool_calls = vec![tinyinference_llm::tool::ToolCall::new(
                "restart-pause-call",
                "ask_user_clarification",
                json!({"question": "Which file should I inspect?"}),
            )];
            response.finish_reason = Some("tool_calls".into());
            return Ok(response);
        }
        let _ = self.resumed.send(());
        self.release.notified().await;
        Ok(ModelResponse::assistant("resume completed"))
    }
}

#[tokio::test]
async fn resuming_after_owner_cache_loss_registers_live_task_before_launch() {
    use crate::agent::orchestration::subagent_sessions::DurableSubagentStatus;
    use crate::agent::orchestration::tools::SpawnSubagentTool;
    use tinyagents_harness_tools::subagents::SubAgentJobRegistry;
    use tinyagents_orchestration::subagent::SubAgentJobStatus;

    let _lock = crate::config::TEST_ENV_LOCK.lock().await;
    let _ = AgentDefinitionRegistry::init_global_builtins();
    let workspace = tempfile::TempDir::new().expect("workspace");
    let (resumed_tx, mut resumed_rx) = tokio::sync::mpsc::unbounded_channel();
    let release = Arc::new(tokio::sync::Notify::new());
    let provider = Arc::new(PauseThenHoldResumeModel {
        calls: std::sync::atomic::AtomicUsize::new(0),
        resumed: resumed_tx,
        release: Arc::clone(&release),
    });
    let ask_tool: Box<dyn tinytools::Tool> =
        Box::new(tinyagents_harness_tools::AskClarificationTool::new());
    let ask_spec = Arc::new(ask_tool.spec());
    let mut parent = parent_context(workspace.path(), provider.clone(), vec![]);
    parent.session_id = "restart-resume-owner-parent".into();
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel(32);
    parent.on_progress = Some(progress_tx);
    parent.all_tools = Arc::new(vec![ask_tool]);
    parent.all_tool_specs = Arc::new(vec![ask_spec.clone()]);
    parent.visible_tool_specs = Arc::new(vec![ask_spec]);
    parent
        .visible_tool_names
        .insert("ask_user_clarification".into());
    parent
        .subagent_tool_ceiling_names
        .insert("ask_user_clarification".into());
    let (live, core) = super::continuation_tests::core_backed_live_parent(
        workspace.path(),
        parent,
        "restart-resume-owner-parent",
        "restart-resume-owner-thread",
    );
    let spawn_tool = SpawnSubagentTool::new();
    let spawn = || {
        spawn_tool.execute_with_live_parent_context(
            json!({
                "agent_id": "task_manager_agent",
                "prompt": "Investigate the restart-resume owner fixture",
                "model": "test-model",
                "task_key": "restart-resume-owner-fixture",
                "blocking": true
            }),
            None,
            live.data.child(),
            crate::agent::subagent_host::detached_subagent_parent(&live, "restart-resume-child")
                .expect("detached parent"),
        )
    };
    let paused = crate::core::runtime::CoreContext::scope(Arc::clone(&core), spawn())
        .await
        .expect("spawn child that pauses");
    assert!(!paused.is_error, "{}", paused.output());
    let task_id = paused
        .output()
        .split("[async_subagent_ref]\n")
        .nth(1)
        .and_then(|value| value.split("\n[/async_subagent_ref]").next())
        .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok())
        .and_then(|value| value["task_id"].as_str().map(str::to_owned))
        .expect("accepted task id");
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if matches!(progress_rx.recv().await, Some(crate::progress::AgentProgress::SubagentAwaitingUser { task_id: ref id, .. }) if id == &task_id) {
                break;
            }
        }
    })
    .await
    .expect("initial child projects its pause");
    let dispatch = live.data.dispatch_ctx();
    let jobs = crate::core::runtime::slot_for::<SubAgentJobRegistry>(&dispatch);
    let paused_job = jobs
        .list()
        .into_iter()
        .find(|job| {
            job.extensions
                .get("task_id")
                .and_then(serde_json::Value::as_str)
                == Some(task_id.as_str())
        })
        .expect("paused job remains registered");
    assert_eq!(paused_job.status, SubAgentJobStatus::AwaitingInput);
    let durable = super::driver_sessions_for_parent(
        &dispatch,
        workspace.path(),
        "restart-resume-owner-parent",
        Some("restart-resume-owner-thread"),
    )
    .await
    .expect("read durable paused session");
    assert!(durable.iter().any(|row| {
        row.current_task_id.as_deref() == Some(task_id.as_str())
            && row.status == DurableSubagentStatus::AwaitingUser
    }));
    crate::agent::orchestration::completion_owners::clear_driver_task_owner_for_test(
        &dispatch, &task_id,
    );
    assert!(
        crate::agent::orchestration::completion_owners::driver_tasks_for_thread(
            &dispatch,
            workspace.path(),
            "restart-resume-owner-thread",
        )
        .is_empty(),
        "clearing scoped process-local ownership simulates a process restart"
    );

    let resumed = crate::core::runtime::CoreContext::scope(Arc::clone(&core), spawn())
        .await
        .expect("resume paused child after restart");
    assert!(!resumed.is_error, "{}", resumed.output());
    tokio::time::timeout(std::time::Duration::from_secs(5), resumed_rx.recv())
        .await
        .expect("resumed provider call starts")
        .expect("resumed provider notification");
    let owner_restored = crate::agent::orchestration::completion_owners::driver_tasks_for_thread(
        &dispatch,
        workspace.path(),
        "restart-resume-owner-thread",
    )
    .iter()
    .any(|(owner_task_id, _)| owner_task_id == &task_id);
    release.notify_one();
    assert!(
        owner_restored,
        "resume must register the active task owner before launching its provider"
    );
}

