use super::*;

#[tokio::test]
async fn spawn_subagent_with_same_task_key_resumes_paused_job_through_shared_driver() {
    use crate::agent::orchestration::subagent_sessions::{
        DurableSubagentStatus, SubagentSessionStore,
    };
    use crate::agent::orchestration::tools::SpawnSubagentTool;
    use tinyagents_harness_tools::subagents::SubAgentJobRegistry;
    use tinyagents_orchestration::subagent::SubAgentJobStatus;

    let _lock = crate::config::TEST_ENV_LOCK.lock().await;
    let _ = AgentDefinitionRegistry::init_global_builtins();
    let workspace = tempfile::TempDir::new().expect("workspace");
    let provider = Arc::new(PauseThenAnswerModel {
        calls: std::sync::atomic::AtomicUsize::new(0),
        seen: Mutex::new(Vec::new()),
    });
    let ask_tool: Box<dyn tinytools::Tool> =
        Box::new(tinyagents_harness_tools::AskClarificationTool::new());
    let ask_spec = Arc::new(ask_tool.spec());
    let mut parent = parent_context(workspace.path(), provider.clone(), vec![]);
    parent.session_id = "continue-paused-e2e".into();
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel(64);
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
    let (live, _core) = core_backed_live_parent(
        workspace.path(),
        parent,
        "continue-paused-e2e",
        "continue-paused-e2e-thread",
    );
    assert!(
        live.data.dispatch_ctx().resolved().is_some(),
        "the parent carrier retains its core context"
    );
    let paused = SpawnSubagentTool::new()
        .execute_with_live_parent_context(
            json!({
                "agent_id": "task_manager_agent",
                "prompt": "Investigate the pause-resume fixture",
                "model": "test-model",
                "task_key": "pause-resume-fixture",
                "blocking": true
            }),
            None,
            live.data.child(),
            crate::agent::subagent_host::detached_subagent_parent(&live, "pause-resume-child")
                .expect("detached parent"),
        )
        .await
        .expect("initial spawn");
    assert!(!paused.is_error, "{}", paused.output());
    let paused_ref = paused
        .output()
        .split("[async_subagent_ref]\n")
        .nth(1)
        .and_then(|value| value.split("\n[/async_subagent_ref]").next())
        .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok())
        .unwrap_or_else(|| {
            panic!(
                "accepted envelope carries task reference: {}",
                paused.output()
            )
        });
    let task_id = paused_ref["task_id"]
        .as_str()
        .expect("accepted task id")
        .to_owned();

    let jobs = crate::core::runtime::slot_for::<SubAgentJobRegistry>(&live.data.dispatch_ctx());
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            match progress_rx.recv().await {
                Some(crate::progress::AgentProgress::SubagentAwaitingUser {
                    task_id: event_task_id,
                    ..
                }) if event_task_id == task_id => break,
                Some(_) => {}
                None => panic!("parent progress stream closed before the pause was projected"),
            }
        }
    })
    .await
    .expect("host pause projection follows durable checkpoint persistence");
    let paused_job = jobs
        .list()
        .into_iter()
        .find(|job| {
            job.extensions
                .get("task_id")
                .and_then(serde_json::Value::as_str)
                == Some(task_id.as_str())
        })
        .expect("initial child is registered in shared Jobs");
    let paused_job = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        jobs.wait_owned(
            paused_job.id.as_str(),
            crate::agent::tinyagents::tool_registration::subagent_owner_key(
                workspace.path(),
                "continue-paused-e2e",
                Some("continue-paused-e2e-thread"),
            ),
        ),
    )
    .await
    .expect("child reaches its clarification checkpoint")
    .expect("registered child remains available");
    assert_eq!(paused_job.status, SubAgentJobStatus::AwaitingInput);
    assert_eq!(
        provider.calls.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "the initial child must reach and persist its clarification before resume"
    );

    let sessions = SubagentSessionStore::new(workspace.path().to_path_buf());
    let before_session = sessions
        .load()
        .expect("initial session persisted")
        .into_iter()
        .find(|session| session.current_task_id.as_deref() == Some(task_id.as_str()))
        .expect("initial durable session points at the paused child");
    assert_eq!(before_session.status, DurableSubagentStatus::AwaitingUser);
    let original_lifecycle_key = before_session
        .lifecycle_key
        .clone()
        .expect("paused session retains its original TinyAgents task key");
    assert_eq!(original_lifecycle_key.task_id, task_id);
    assert_eq!(
        original_lifecycle_key.thread_id.as_deref(),
        Some("continue-paused-e2e-thread")
    );
    let before_job_count = jobs.list().len();

    let resumed = SpawnSubagentTool::new()
        .execute_with_live_parent_context(
            json!({
                "agent_id": "task_manager_agent",
                "prompt": "Inspect the README and continue.",
                "model": "test-model",
                "task_key": "pause-resume-fixture"
            }),
            None,
            live.data.child(),
            crate::agent::subagent_host::detached_subagent_parent(&live, "pause-resume-child")
                .expect("detached parent"),
        )
        .await
        .expect("continue");
    assert!(!resumed.is_error, "{}", resumed.output());
    let resumed_ref = resumed
        .output()
        .split("[async_subagent_ref]\n")
        .nth(1)
        .and_then(|value| value.split("\n[/async_subagent_ref]").next())
        .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok())
        .unwrap_or_else(|| {
            panic!(
                "resumed result carries task reference: {}",
                resumed.output()
            )
        });
    assert_eq!(
        resumed_ref["task_id"].as_str(),
        Some(task_id.as_str()),
        "resuming a checkpoint must keep its registered task identity"
    );
    assert_eq!(
        jobs.list().len(),
        before_job_count,
        "resume must not spawn a second child"
    );
    assert_eq!(
        resumed_ref["subagent_session_id"].as_str(),
        Some(before_session.subagent_session_id.as_str()),
        "resume must keep the durable session identity"
    );
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            match progress_rx.recv().await {
                Some(crate::progress::AgentProgress::SubagentCompleted {
                    task_id: event_task_id,
                    ..
                }) if event_task_id == task_id => break,
                Some(_) => {}
                None => panic!("parent progress stream closed before child completion"),
            }
        }
    })
    .await
    .expect("resume completion projection follows durable settlement");
    assert!(provider.saw("Inspect the README and continue."));
    assert_eq!(provider.calls.load(std::sync::atomic::Ordering::SeqCst), 2);
    let requests = provider.seen.lock();
    assert_eq!(requests.len(), 2);
    let resumed_request = &requests[1];
    assert!(resumed_request.contains("Investigate the pause-resume fixture"));
    assert!(resumed_request.contains("which file should I inspect?"));
    assert_eq!(
        resumed_request
            .matches("Inspect the README and continue.")
            .count(),
        1,
        "the answer is appended to the resumed history exactly once"
    );
    drop(requests);

    let completed_job = jobs
        .get(paused_job.id.as_str())
        .expect("continuation reuses the same registered job");
    assert_eq!(completed_job.agent, "task_manager_agent");
    assert_eq!(completed_job.status, SubAgentJobStatus::Completed);
    assert_eq!(
        completed_job.output.as_deref(),
        Some("paused-child-resumed-answer")
    );
    let completed_session = sessions
        .load()
        .expect("completed session list")
        .into_iter()
        .find(|session| session.current_task_id.as_deref() == Some(task_id.as_str()))
        .expect("completed row still owns the original task");
    assert_eq!(
        completed_session.lifecycle_key.as_ref(),
        Some(&original_lifecycle_key),
        "resume preserves the original lifecycle identity"
    );
}

#[tokio::test]
async fn spawn_subagent_reuses_idle_task_key_with_the_saved_history() {
    use crate::agent::orchestration::subagent_sessions::{
        DurableSubagentStatus, SubagentSessionStore,
    };
    use crate::agent::orchestration::tools::SpawnSubagentTool;
    use tinyagents_harness_tools::subagents::SubAgentJobRegistry;
    use tinyagents_orchestration::subagent::SubAgentJobStatus;

    let _lock = crate::config::TEST_ENV_LOCK.lock().await;
    let _ = AgentDefinitionRegistry::init_global_builtins();
    let workspace = tempfile::TempDir::new().expect("workspace");
    let provider = Arc::new(TwoTurnModel::default());
    let mut parent = parent_context(workspace.path(), provider.clone(), vec![]);
    parent.session_id = "idle-reuse-e2e".into();
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel(64);
    parent.on_progress = Some(progress_tx);
    let (live, _core) = core_backed_live_parent(
        workspace.path(),
        parent,
        "idle-reuse-e2e",
        "idle-reuse-e2e-thread",
    );
    assert!(live.data.dispatch_ctx().resolved().is_some());
    let tool = SpawnSubagentTool::new();
    let first = tool
        .execute_with_live_parent_context(
            json!({
                "agent_id": "task_manager_agent",
                "prompt": "idle-history-first-prompt",
                "model": "test-model",
                "task_key": "idle-history-fixture"
            }),
            None,
            live.data.child(),
            crate::agent::subagent_host::detached_subagent_parent(&live, "idle-reuse-child")
                .expect("detached parent"),
        )
        .await
        .expect("first spawn");
    assert!(!first.is_error, "{}", first.output());
    wait_completed(&mut progress_rx).await;

    let sessions = SubagentSessionStore::new(workspace.path().to_path_buf());
    let first_session = sessions
        .load()
        .expect("first session list")
        .into_iter()
        .find(|session| session.task_key == "idle-history-fixture")
        .expect("first durable session");
    assert_eq!(first_session.status, DurableSubagentStatus::Idle);
    let original_session_id = first_session.subagent_session_id.clone();
    let first_task_id = first_session.current_task_id.clone().expect("first task id");
    let first_job = crate::core::runtime::slot_for::<SubAgentJobRegistry>(
        &live.data.dispatch_ctx(),
    )
    .list()
    .into_iter()
    .find(|job| {
        job.extensions
            .get("task_id")
            .and_then(serde_json::Value::as_str)
            == Some(first_task_id.as_str())
    })
    .expect("first child is registered");
    assert_eq!(first_job.status, SubAgentJobStatus::Completed);

    let second = tool
        .execute_with_live_parent_context(
            json!({
                "agent_id": "task_manager_agent",
                "prompt": "idle-history-follow-up-prompt",
                "model": "test-model",
                "task_key": "idle-history-fixture"
            }),
            None,
            live.data.child(),
            crate::agent::subagent_host::detached_subagent_parent(&live, "idle-reuse-child")
                .expect("detached parent"),
        )
        .await
        .expect("follow-up spawn");
    assert!(!second.is_error, "{}", second.output());
    wait_completed(&mut progress_rx).await;
    let second_session = sessions
        .load()
        .expect("follow-up session list")
        .into_iter()
        .find(|session| session.subagent_session_id == original_session_id)
        .expect("same durable child session is reused");
    assert_eq!(second_session.status, DurableSubagentStatus::Idle);
    assert_ne!(second_session.current_task_id.as_deref(), Some(first_task_id.as_str()));
    assert!(provider.saw("idle-history-first-prompt"));
    assert!(provider.saw("idle-child-first-result"));
    assert!(provider.saw("idle-history-follow-up-prompt"));
    let requests = provider.seen.lock();
    assert_eq!(requests.len(), 2, "both spawns use the injected model");
    assert_eq!(requests[1].matches("idle-history-follow-up-prompt").count(), 1);
}

async fn wait_completed(
    progress: &mut tokio::sync::mpsc::Receiver<crate::progress::AgentProgress>,
) {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            match progress.recv().await {
                Some(crate::progress::AgentProgress::SubagentCompleted { agent_id, .. })
                    if agent_id == "task_manager_agent" => break,
                Some(crate::progress::AgentProgress::SubagentFailed { error, .. }) => {
                    panic!("child failed before completion: {error}")
                }
                Some(_) => {}
                None => panic!("progress stream closed before child completion"),
            }
        }
    })
    .await
    .expect("completion is projected after durable settlement");
}

#[derive(Default)]
struct TwoTurnModel {
    calls: std::sync::atomic::AtomicUsize,
    seen: Mutex<Vec<String>>,
}

impl TwoTurnModel {
    fn saw(&self, needle: &str) -> bool {
        self.seen.lock().iter().any(|payload| payload.contains(needle))
    }
}

#[async_trait::async_trait]
impl tinyinference_llm::model::ChatModel<()> for TwoTurnModel {
    fn profile(&self) -> Option<&tinyinference_llm::model::ModelProfile> {
        static PROFILE: std::sync::OnceLock<tinyinference_llm::model::ModelProfile> =
            std::sync::OnceLock::new();
        Some(PROFILE.get_or_init(|| {
            let mut profile = tinyinference_llm::model::ModelProfile::default();
            profile.tool_calling = true;
            profile.parallel_tool_calls = true;
            profile
        }))
    }

    async fn invoke(
        &self,
        _state: &(),
        request: tinyinference_llm::model::ModelRequest,
    ) -> tinyinference_llm::Result<tinyinference_llm::model::ModelResponse> {
        let seen = request
            .messages
            .iter()
            .map(tinyinference_llm::message::Message::text)
            .collect::<Vec<_>>()
            .join("\n");
        self.seen.lock().push(seen);
        let answer = match self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst) {
            0 => "idle-child-first-result",
            1 => "idle-child-follow-up-result",
            call => return Err(tinyinference_llm::Error::Model(format!("unexpected call {call}"))),
        };
        Ok(tinyinference_llm::model::ModelResponse::assistant(answer))
    }
}

fn core_backed_live_parent(
    workspace: &std::path::Path,
    parent: crate::agent::harness::ParentExecutionContext,
    session_id: &str,
    thread_id: &str,
) -> (
    tinyagents_harness::context::RunContext<crate::agent::tinyagents::host::OpenHumanRunContext>,
    Arc<crate::core::runtime::CoreContext>,
) {
    use crate::core::runtime::{ContextOverlay, CoreContext, DomainSet};
    use crate::tools::toolpacks::ToolGroups;

    let mut definition = AgentDefinitionRegistry::builtins_only()
        .get("task_manager_agent")
        .expect("task manager definition")
        .clone();
    definition.model = tinyagents_definition::ModelSpec::Inherit;
    let definitions =
        Arc::new(AgentDefinitionRegistry::builtins_only().with_definitions([definition]));
    let mut config = crate::config::Config::default();
    config.workspace_dir = workspace.to_path_buf();
    config.components.juice = false;
    let core = CoreContext::for_test_with_config(DomainSet::full(), config.clone()).derive_with(
        ContextOverlay::new(config, DomainSet::full(), ToolGroups::none()).definitions(definitions),
    );
    let mut parent = parent;
    parent.workspace_dir = workspace.to_path_buf();
    parent.session_id = session_id.to_owned();
    let mut host_context = crate::agent::tinyagents::host::OpenHumanRunContext::new()
        .with_core(Some(Arc::clone(&core)))
        .with_parent(parent);
    host_context.thread_id = Some(thread_id.to_owned());
    (
        tinyagents_harness::context::RunContext::new(
            tinyagents_harness::context::RunConfig::new(session_id).with_thread(thread_id),
            host_context,
        ),
        core,
    )
}

#[tokio::test]
async fn cancelling_a_paused_driver_settles_once_with_its_reason() {
    use crate::agent::orchestration::background_completions::{
        forget_workspace_for_test, pending_for_ctx,
    };
    use crate::agent::orchestration::subagent_sessions::{
        DurableSubagentStatus, SubagentSessionStore,
    };
    use crate::agent::orchestration::tools::SpawnSubagentTool;
    use tinyagents_harness_tools::subagents::SubAgentJobRegistry;
    use tinyagents_orchestration::subagent::SubAgentJobStatus;
    use tinyagents_tasks::CompletionStatus;

    let _lock = crate::config::TEST_ENV_LOCK.lock().await;
    let _ = AgentDefinitionRegistry::init_global_builtins();
    let workspace = tempfile::tempdir().expect("workspace");
    let provider = Arc::new(PauseThenAnswerModel {
        calls: std::sync::atomic::AtomicUsize::new(0),
        seen: Mutex::new(Vec::new()),
    });
    let ask_tool: Box<dyn tinytools::Tool> =
        Box::new(tinyagents_harness_tools::AskClarificationTool::new());
    let ask_spec = Arc::new(ask_tool.spec());
    let mut parent = parent_context(workspace.path(), provider.clone(), vec![]);
    parent.session_id = "paused-cancel-rpc-parent".into();
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel(64);
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

    let (live, _core) = core_backed_live_parent(
        workspace.path(),
        parent,
        "paused-cancel-rpc-parent",
        "paused-cancel-rpc-thread",
    );
    assert!(
        live.data.dispatch_ctx().resolved().is_some(),
        "the parent carrier retains its core context"
    );
    let parent_cancellation = live.data.child().cancellation;
    let jobs =
        (*crate::core::runtime::slot_for::<SubAgentJobRegistry>(&live.data.dispatch_ctx())).clone();
    let paused = SpawnSubagentTool::new()
        .execute_with_live_parent_context(
            json!({
                "agent_id": "task_manager_agent",
                "prompt": "Investigate the paused RPC cancellation fixture",
                "model": "test-model",
                "blocking": true
            }),
            None,
            live.data.child(),
            crate::agent::subagent_host::detached_subagent_parent(&live, "paused-cancel-child")
                .expect("detached parent"),
        )
        .await
        .expect("initial spawn");
    assert!(!paused.is_error, "{}", paused.output());
    let paused_ref = paused
        .output()
        .split("[async_subagent_ref]\n")
        .nth(1)
        .and_then(|value| value.split("\n[/async_subagent_ref]").next())
        .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok())
        .unwrap_or_else(|| {
            panic!(
                "accepted envelope carries task reference: {}",
                paused.output()
            )
        });
    let task_id = paused_ref["task_id"]
        .as_str()
        .expect("accepted task id")
        .to_owned();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            match progress_rx.recv().await {
                Some(crate::progress::AgentProgress::SubagentAwaitingUser {
                    task_id: event_task_id,
                    ..
                }) if event_task_id == task_id => break,
                Some(_) => {}
                None => panic!("parent progress stream closed before the pause was projected"),
            }
        }
    })
    .await
    .expect("host pause projection follows durable checkpoint persistence");
    assert_eq!(provider.calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    let paused_job = jobs
        .list()
        .into_iter()
        .find(|job| {
            job.extensions
                .get("task_id")
                .and_then(serde_json::Value::as_str)
                == Some(task_id.as_str())
        })
        .expect("the paused child is registered in shared Jobs");
    assert_eq!(paused_job.status, SubAgentJobStatus::AwaitingInput);
    let queued = pending_for_ctx(
        &live.data.dispatch_ctx(),
        workspace.path(),
        "paused-cancel-rpc-thread",
    );
    assert!(
        queued.iter().any(|record| {
            record.label.as_deref()
                == Some(crate::agent::orchestration::background_completions::AWAITING_INPUT_LABEL)
        }),
        "the asynchronous pause notice is queued independently"
    );
    let sessions = SubagentSessionStore::new(workspace.path().to_path_buf());
    let parent_session = live
        .data
        .parent
        .as_ref()
        .expect("live parent execution context")
        .session_id
        .clone();
    let before_row = sessions
        .load()
        .expect("paused durable session list")
        .into_iter()
        .find(|row| row.current_task_id.as_deref() == Some(task_id.as_str()))
        .expect("actual paused durable session remains available");
    assert_eq!(before_row.status, DurableSubagentStatus::AwaitingUser);
    assert_eq!(before_row.parent_session, parent_session);
    assert_eq!(
        before_row.parent_thread_id.as_deref(),
        Some("paused-cancel-rpc-thread")
    );
    assert!(before_row.lifecycle_key.is_some());
    assert_eq!(
        live.data
            .dispatch_ctx()
            .resolved()
            .expect("RPC uses the spawned parent's core")
            .workspace_dir()
            .expect("RPC resolves the parent's workspace"),
        workspace.path()
    );

    let controller = crate::agent_runs::subagent_control::all_registered_controllers()
        .into_iter()
        .find(|controller| {
            controller.schema.namespace == "subagent" && controller.schema.function == "cancel"
        })
        .expect("subagent cancel RPC is registered");
    let mut params = serde_json::Map::new();
    params.insert("taskId".into(), json!(task_id));
    params.insert("reason".into(), json!("user requested stop"));
    let cancelled = (controller.handler)(live.data.dispatch_ctx(), params)
        .await
        .expect("cancel RPC result");
    let response_field = |value: &serde_json::Value, name: &str| {
        value
            .get("data")
            .and_then(|data| data.get(name))
            .or_else(|| value.get(name))
            .cloned()
    };
    assert_eq!(response_field(&cancelled, "cancelled"), Some(json!(true)));
    assert_eq!(provider.calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert!(matches!(
        jobs.get(paused_job.id.as_str())
            .expect("paused job remains inspectable")
            .status,
        SubAgentJobStatus::Cancelled
    ));
    let row = SubagentSessionStore::new(workspace.path().to_path_buf())
        .load()
        .expect("durable session")
        .into_iter()
        .find(|row| row.current_task_id.as_deref() == Some(task_id.as_str()))
        .expect("cancelled durable session remains available");
    assert_eq!(row.status, DurableSubagentStatus::Failed);
    assert!(row
        .latest_error
        .as_deref()
        .is_some_and(|error| error.contains("user requested stop")));
    let deliveries = pending_for_ctx(
        &live.data.dispatch_ctx(),
        workspace.path(),
        "paused-cancel-rpc-thread",
    );
    let failures: Vec<_> = deliveries
        .iter()
        .filter(|record| record.task_id == task_id && record.status == CompletionStatus::Failed)
        .collect();
    assert_eq!(failures.len(), 1, "cancellation settles once for this task");
    assert!(failures[0].result.text.contains("[SUBAGENT_FAILED]"));
    assert!(failures[0].result.text.contains("user requested stop"));

    let mut repeated_params = serde_json::Map::new();
    repeated_params.insert("taskId".into(), json!(task_id));
    let repeated = (controller.handler)(live.data.dispatch_ctx(), repeated_params)
        .await
        .expect("repeated cancel RPC result");
    assert_eq!(response_field(&repeated, "cancelled"), Some(json!(false)));
    assert_eq!(response_field(&repeated, "outcome"), Some(json!("unknown")));
    let deliveries_after_repeat = pending_for_ctx(
        &live.data.dispatch_ctx(),
        workspace.path(),
        "paused-cancel-rpc-thread",
    );
    assert_eq!(
        deliveries_after_repeat
            .iter()
            .filter(|record| record.task_id == task_id && record.status == CompletionStatus::Failed)
            .count(),
        1,
        "repeated cancellation does not enqueue another terminal failure"
    );
    forget_workspace_for_test(workspace.path());
    assert!(!parent_cancellation.is_cancelled());
}
