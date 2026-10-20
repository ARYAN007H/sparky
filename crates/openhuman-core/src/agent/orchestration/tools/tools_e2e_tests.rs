use super::SpawnSubagentTool;
use crate::agent::harness::ParentExecutionContext;
use crate::agent_registry::host_definition::AgentDefinitionRegistry;
use crate::integrations::ConnectedIntegration;
use async_trait::async_trait;
use parking_lot::Mutex;
use serde_json::json;
use std::path::Path;
use std::sync::Arc;
use tinyinference_llm::message::Message;
use tinyinference_llm::model::{ChatModel, ModelProfile, ModelRequest, ModelResponse};
use tinytools_agent::dialect::ToolCallFormat;

const SPAWN_SUBAGENT_CANARY: &str = "tool-e2e-spawn-subagent-canary";

struct StopGateModel {
    started: tokio::sync::mpsc::Sender<()>,
    release: Arc<tokio::sync::Notify>,
}

#[async_trait]
impl ChatModel<()> for StopGateModel {
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
        let _ = self.started.send(()).await;
        self.release.notified().await;
        Ok(ModelResponse::assistant("released"))
    }
}

#[tokio::test]
async fn unscoped_web_stop_cancels_a_running_driver_subagent() {
    let workspace = tempfile::TempDir::new().expect("workspace");
    let thread_id = "stop-detached-subagent-thread";
    let (started_tx, mut started_rx) = tokio::sync::mpsc::channel(1);
    let release = Arc::new(tokio::sync::Notify::new());
    let model = Arc::new(StopGateModel {
        started: started_tx,
        release,
    });
    let mut parent = parent_context(workspace.path(), model, vec![]);
    parent.session_id = "stop-detached-subagent-session".into();
    let mut config = crate::config::Config::default();
    config.workspace_dir = workspace.path().to_path_buf();
    config.components.juice = false;
    let definitions = Arc::new(AgentDefinitionRegistry::builtins_only());
    let core = crate::core::runtime::CoreContext::for_test_with_config(
        crate::core::runtime::DomainSet::full(),
        config.clone(),
    )
    .derive_with(crate::core::runtime::ContextOverlay::new(
        config,
        crate::core::runtime::DomainSet::full(),
        crate::tools::toolpacks::ToolGroups::none(),
    )
    .definitions(definitions));
    let host = crate::agent::tinyagents::host::OpenHumanRunContext::new()
        .with_core(Some(Arc::clone(&core)))
        .with_parent(parent);
    let live = tinyagents_harness::context::RunContext::new(
        tinyagents_harness::context::RunConfig::new("stop-detached-subagent-session")
            .with_thread(thread_id),
        host,
    );
    let accepted = crate::core::runtime::CoreContext::scope(
        Arc::clone(&core),
        Box::pin(SpawnSubagentTool::new().execute_with_live_parent_context(
            json!({
                "agent_id": "task_manager_agent",
                "prompt": "wait for the thread stop",
                "model": "test-model",
                "task_key": "stop detached child"
            }),
            None,
            live.data.child(),
            crate::agent::subagent_host::detached_subagent_parent(&live, "stop-child")
                .expect("detached parent"),
        )),
    )
    .await
    .expect("spawn child");
    assert!(!accepted.is_error, "{}", accepted.output());
    let task_id = accepted
        .output()
        .split("[async_subagent_ref]\n")
        .nth(1)
        .and_then(|value| value.split("\n[/async_subagent_ref]").next())
        .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok())
        .and_then(|value| value["task_id"].as_str().map(str::to_owned))
        .expect("accepted child includes its task ID");
    tokio::time::timeout(std::time::Duration::from_secs(5), started_rx.recv())
        .await
        .expect("driver child reached provider gate")
        .expect("provider start");
    let dispatch = live.data.dispatch_ctx();
    assert!(dispatch.resolved().is_some(), "stop uses the live core context");
    let jobs = crate::core::runtime::slot_for::<
        tinyagents_harness_tools::subagents::SubAgentJobRegistry,
    >(&dispatch);
    let job = jobs
        .list()
        .into_iter()
        .find(|job| {
            job.extensions
                .get("task_id")
                .and_then(serde_json::Value::as_str)
                == Some(task_id.as_str())
        })
        .expect("driver owns a live child job");
    assert_eq!(job.status, tinyagents_orchestration::subagent::SubAgentJobStatus::Running);

    let scoped = crate::core::runtime::CoreContext::scope(
        Arc::clone(&core),
        crate::web_chat::channel_web_cancel(
            &dispatch,
            "stop-client",
            thread_id,
            Some("req-unrelated"),
        ),
    )
    .await
    .expect("scoped cancel");
    assert_eq!(scoped.value["cancelled"], serde_json::json!(false));
    assert_eq!(scoped.value["subagents_cancelled"], serde_json::json!(0));
    assert_eq!(jobs.get(job.id.as_str()).unwrap().status, tinyagents_orchestration::subagent::SubAgentJobStatus::Running);

    let stop = crate::core::runtime::CoreContext::scope(
        Arc::clone(&core),
        crate::web_chat::channel_web_cancel(&dispatch, "stop-client", thread_id, None),
    )
    .await
    .expect("unscoped stop");
    assert_eq!(stop.value["cancelled"], serde_json::json!(true));
    assert_eq!(stop.value["request_id"], serde_json::Value::Null);
    assert_eq!(stop.value["subagents_cancelled"], serde_json::json!(1));
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if jobs.get(job.id.as_str()).is_some_and(|job| {
                job.status == tinyagents_orchestration::subagent::SubAgentJobStatus::Cancelled
            }) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("driver child cancellation settles");
}

#[tokio::test]
async fn spawn_subagent_returns_async_reference_and_runs_child_e2e() {
    let workspace = tempfile::TempDir::new().expect("workspace");
    let provider = Arc::new(ScriptedModel::new(vec![(
        SPAWN_SUBAGENT_CANARY,
        "spawn-subagent-child-answer",
    )]));
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel(16);
    let mut parent = parent_context(workspace.path(), provider.clone(), vec![]);
    parent.on_progress = Some(progress_tx);
    let mut config = crate::config::Config::default();
    config.workspace_dir = workspace.path().to_path_buf();
    config.components.juice = false;
    let definitions = Arc::new(AgentDefinitionRegistry::builtins_only());
    let core = crate::core::runtime::CoreContext::for_test_with_config(
        crate::core::runtime::DomainSet::full(),
        config.clone(),
    )
    .derive_with(
        crate::core::runtime::ContextOverlay::new(
            config,
            crate::core::runtime::DomainSet::full(),
            crate::tools::toolpacks::ToolGroups::none(),
        )
        .definitions(definitions),
    );
    let host = crate::agent::tinyagents::host::OpenHumanRunContext::new()
        .with_core(Some(Arc::clone(&core)))
        .with_parent(parent);
    let live = tinyagents_harness::context::RunContext::new(
        tinyagents_harness::context::RunConfig::new("spawn-subagent-e2e"),
        host,
    );
    // Boxed: the inline spawn future is too large for a test thread's stack.
    let result = crate::core::runtime::CoreContext::scope(
        Arc::clone(&core),
        Box::pin(
            SpawnSubagentTool::new().execute_with_live_parent_context(
                json!({
                    "agent_id": "task_manager_agent",
                    "prompt": format!("Investigate {SPAWN_SUBAGENT_CANARY}"),
                    "context": "parent supplied context",
                    "model": "test-model",
                }),
                None,
                live.data.child(),
                crate::agent::subagent_host::detached_subagent_parent(
                    &live,
                    "spawn-subagent-child",
                )
                .expect("detached parent"),
            ),
        ),
    )
    .await
    .expect("tool execution");

    assert!(!result.is_error, "{}", result.output());
    assert!(
        result.output().contains("[async_subagent_ref]"),
        "spawn returns an async task reference before its child result: {}",
        result.output()
    );
    let task_ref = result
        .output()
        .split("[async_subagent_ref]\n")
        .nth(1)
        .and_then(|value| value.split("\n[/async_subagent_ref]").next())
        .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok())
        .expect("accepted async envelope contains the child task reference");
    let task_id = task_ref["task_id"].as_str().expect("public task id");
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            match progress_rx.recv().await {
                Some(crate::progress::AgentProgress::SubagentCompleted {
                    task_id: event_task_id,
                    ..
                }) if event_task_id == task_id => break,
                Some(crate::progress::AgentProgress::SubagentFailed {
                    task_id: event_task_id,
                    error,
                    ..
                }) if event_task_id == task_id => {
                    panic!("scripted child failed before completing: {error}")
                }
                Some(_) => {}
                None => panic!("parent progress stream closed before child completion"),
            }
        }
    })
    .await
    .expect("matching completion progress follows durable settlement");
    assert!(provider.saw(SPAWN_SUBAGENT_CANARY));
    assert!(provider.saw("parent supplied context"));
    let jobs = crate::core::runtime::slot_for::<
        tinyagents_harness_tools::subagents::SubAgentJobRegistry,
    >(&live.data.dispatch_ctx());
    let job = jobs
        .list()
        .into_iter()
        .find(|job| {
            job.extensions
                .get("task_id")
                .and_then(serde_json::Value::as_str)
                == Some(task_id)
        })
        .expect("spawn registers its accepted child in shared Jobs");
    let terminal = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        jobs.wait_owned(
            job.id.as_str(),
            crate::agent::tinyagents::tool_registration::subagent_owner_key(
                workspace.path(),
                "tools-e2e-session",
                None,
            ),
        ),
    )
    .await
    .expect("accepted child completes asynchronously")
    .expect("accepted job remains in the shared registry");
    assert_eq!(
        terminal.status,
        tinyagents_orchestration::subagent::SubAgentJobStatus::Completed
    );
    assert_eq!(
        terminal.output.as_deref(),
        Some("spawn-subagent-child-answer")
    );
}

#[tokio::test]
async fn cancel_of_a_completed_driver_child_reports_completed_without_rewriting_its_session() {
    use crate::agent::orchestration::subagent_sessions::SubagentSessionStore;
    use crate::core::runtime::{ContextOverlay, CoreContext, DispatchCtx, DomainSet};

    let workspace = tempfile::TempDir::new().expect("workspace");
    let provider = Arc::new(ScriptedModel::new(vec![(
        "completed-cancel-canary",
        "child completed before late cancel",
    )]));
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel(16);
    let mut parent = parent_context(workspace.path(), provider, vec![]);
    parent.session_id = "completed-cancel-parent-session".into();
    parent.on_progress = Some(progress_tx);
    let mut config = crate::config::Config::default();
    config.workspace_dir = workspace.path().to_path_buf();
    config.components.juice = false;
    let definitions = Arc::new(AgentDefinitionRegistry::builtins_only());
    let core = CoreContext::for_test_with_config(DomainSet::full(), config.clone()).derive_with(
        ContextOverlay::new(config, DomainSet::full(), crate::tools::toolpacks::ToolGroups::none())
            .definitions(definitions),
    );
    let thread_id = "completed-cancel-parent-thread";
    let host = crate::agent::tinyagents::host::OpenHumanRunContext::new()
        .with_core(Some(Arc::clone(&core)))
        .with_parent(parent);
    let live = tinyagents_harness::context::RunContext::new(
        tinyagents_harness::context::RunConfig::new("completed-cancel-parent-session")
            .with_thread(thread_id),
        host,
    );
    let result = CoreContext::scope(
        Arc::clone(&core),
        Box::pin(SpawnSubagentTool::new().execute_with_live_parent_context(
            json!({
                "agent_id": "task_manager_agent",
                "prompt": "Investigate completed-cancel-canary",
                "model": "test-model",
                "task_key": "completed child late cancel"
            }),
            None,
            live.data.child(),
            crate::agent::subagent_host::detached_subagent_parent(&live, "completed-cancel-child")
                .expect("detached parent"),
        )),
    )
    .await
    .expect("spawn tool execution");
    assert!(!result.is_error, "{}", result.output());
    let task_id = result
        .output()
        .split("[async_subagent_ref]\n")
        .nth(1)
        .and_then(|value| value.split("\n[/async_subagent_ref]").next())
        .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok())
        .and_then(|value| value["task_id"].as_str().map(str::to_owned))
        .expect("accepted child task reference");
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            match progress_rx.recv().await {
                Some(crate::progress::AgentProgress::SubagentCompleted {
                    task_id: event_task_id,
                    ..
                }) if event_task_id == task_id => break,
                Some(crate::progress::AgentProgress::SubagentFailed {
                    task_id: event_task_id,
                    error,
                    ..
                }) if event_task_id == task_id => panic!("child failed: {error}"),
                Some(_) => {}
                None => panic!("child progress stream closed"),
            }
        }
    })
    .await
    .expect("child completed before the late cancel");

    let store = SubagentSessionStore::new(workspace.path().to_path_buf());
    let before = CoreContext::sync_scope(Arc::clone(&core), || store.load())
        .expect("load completed durable row");
    assert!(before.iter().any(|row| {
        row.current_task_id.as_deref() == Some(task_id.as_str())
            && row.parent_session == "completed-cancel-parent-session"
    }));
    let before_snapshot = serde_json::to_string(&before).expect("encode completed rows");
    let controller = crate::agent_runs::all_subagent_control_registered_controllers()
        .into_iter()
        .find(|controller| controller.schema.function == "cancel")
        .expect("subagent cancel RPC registered");
    let mut params = serde_json::Map::new();
    params.insert("taskId".into(), json!(task_id));
    let output = (controller.handler)(DispatchCtx::new(Arc::clone(&core)), params)
        .await
        .expect("late cancel handler");
    assert_eq!(output["cancelled"], serde_json::json!(false));
    assert_eq!(output["outcome"], serde_json::json!("completed"));
    let after_snapshot = CoreContext::sync_scope(Arc::clone(&core), || store.load())
        .map(|rows| serde_json::to_string(&rows).expect("encode rows after cancel"))
        .expect("load row after cancel");
    assert_eq!(after_snapshot, before_snapshot, "late cancel must not rewrite completed state");
}

/// A child the harness stops early (failure breaker / iteration cap) is not a
/// successful result: asynchronous completion preserves the incomplete marker
/// for delivery and reports the child's stop through parent progress.
#[tokio::test(start_paused = true)]
async fn spawn_subagent_reports_a_stopped_child_as_incomplete_progress() {
    let _ = AgentDefinitionRegistry::init_global_builtins();
    let workspace = tempfile::TempDir::new().expect("workspace");
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel(512);
    let mut parent = parent_context(workspace.path(), Arc::new(LoopingToolModel), vec![]);
    parent.session_id = "stopped-child-test".into();
    parent.on_progress = Some(progress_tx);
    let thread_id = "stopped-child-test-thread";
    let (live, _core) = continuation_tests::core_backed_live_parent(
        workspace.path(),
        parent,
        "stopped-child-test",
        thread_id,
    );
    assert!(live.data.dispatch_ctx().resolved().is_some());
    let run_context = live.data.child();
    // Boxed: the inline spawn future is too large for a test thread's
    // stack (the trait `execute` path boxes it the same way).
    let tool = SpawnSubagentTool::new();
    let result = Box::pin(
        tool.execute_with_live_parent_context(
            json!({
                "agent_id": "task_manager_agent",
                "prompt": "List the task sources",
                "model": "test-model"
            }),
            None,
            run_context,
            crate::agent::subagent_host::detached_subagent_parent(&live, "stopped-child")
                .expect("detached parent"),
        ),
    )
    .await
    .expect("tool execution");

    assert!(
        !result.is_error,
        "the asynchronous spawn is accepted before the child settles: {}",
        result.output()
    );
    let task_id = result
        .output()
        .split("[async_subagent_ref]\n")
        .nth(1)
        .and_then(|value| value.split("\n[/async_subagent_ref]").next())
        .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok())
        .and_then(|payload| payload.get("task_id")?.as_str().map(str::to_owned))
        .expect("accepted result carries the child task id");

    let (_output, stop) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            match progress_rx.recv().await {
                Some(crate::progress::AgentProgress::SubagentCompleted {
                    agent_id,
                    task_id: event_task_id,
                    output,
                    stop,
                    ..
                }) if agent_id == "task_manager_agent" && event_task_id == task_id => {
                    break (output, stop)
                }
                Some(crate::progress::AgentProgress::SubagentFailed {
                    agent_id,
                    task_id: event_task_id,
                    error,
                }) if agent_id == "task_manager_agent" && event_task_id == task_id => {
                    panic!("stopped child was reported as a provider failure: {error}")
                }
                Some(_) => {}
                None => panic!("parent progress stream closed before child stop"),
            }
        }
    })
    .await
    .expect("the async completion reaches the parent");
    let stop = stop.expect("SubagentCompleted carries the child's stop");
    assert!(matches!(
        stop.kind,
        tinyagents_harness::turn_stop::TurnStopKind::Breaker
            | tinyagents_harness::turn_stop::TurnStopKind::IterationCap
    ));
    let incomplete_completion =
        crate::agent::orchestration::background_completions::pending_for_ctx(
            &live.data.dispatch_ctx(),
            workspace.path(),
            thread_id,
        )
        .into_iter()
        .find(|record| record.task_id == task_id)
        .expect("the asynchronous incomplete result is retained for delivery");
    assert!(
        incomplete_completion
            .result
            .text
            .contains("[SUBAGENT_INCOMPLETE]"),
        "the delivered result remains explicitly incomplete: {}",
        incomplete_completion.result.text
    );
}

#[tokio::test]
async fn failed_async_child_is_reported_to_the_parent_without_a_success_result() {
    let _ = AgentDefinitionRegistry::init_global_builtins();
    let workspace = tempfile::TempDir::new().expect("workspace");
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel(32);
    let mut parent = parent_context(workspace.path(), Arc::new(FailingModel), vec![]);
    parent.on_progress = Some(progress_tx);
    let live = super::live_parent_run_context("failed-async-child", parent);
    let accepted = SpawnSubagentTool::new()
        .execute_with_live_parent_context(
            json!({
                "agent_id": "task_manager_agent",
                "prompt": "exercise provider failure",
                "model": "test-model"
            }),
            None,
            live.data.child(),
            crate::agent::subagent_host::detached_subagent_parent(&live, "failed-child")
                .expect("detached parent"),
        )
        .await
        .expect("the asynchronous spawn request is accepted");
    assert!(
        !accepted.is_error,
        "child failure is delivered asynchronously"
    );

    let failure = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            match progress_rx.recv().await {
                Some(crate::progress::AgentProgress::SubagentFailed {
                    agent_id, error, ..
                }) if agent_id == "task_manager_agent" => break error,
                Some(_) => {}
                None => panic!("parent progress stream closed before child failure"),
            }
        }
    })
    .await
    .expect("asynchronous child failure reaches the parent");
    assert!(
        failure.starts_with("spawn_subagent failed and did not complete."),
        "failure copy remains recognizable to the parent: {failure}"
    );
    assert!(failure.contains("scripted provider outage"), "{failure}");
    assert!(!failure.contains("completed successfully"), "{failure}");
}

#[tokio::test]
async fn unexecuted_tool_call_stub_is_projected_as_incomplete_not_success() {
    const STUB_CANARY: &str = "unexecuted-tool-call-stub-canary";
    let _ = AgentDefinitionRegistry::init_global_builtins();
    let workspace = tempfile::TempDir::new().expect("workspace");
    let provider = Arc::new(ScriptedModel::new(vec![(
        STUB_CANARY,
        r#"{"tool_calls":[{"type":"function","function":{"name":"missing","arguments":"{}"}}]}"#,
    )]));
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel(32);
    let mut parent = parent_context(workspace.path(), provider.clone(), vec![]);
    parent.on_progress = Some(progress_tx);
    let live = super::live_parent_run_context("stub-async-child", parent);
    let accepted = SpawnSubagentTool::new()
        .execute_with_live_parent_context(
            json!({
                "agent_id": "task_manager_agent",
                "prompt": format!("return {STUB_CANARY}"),
                "model": "test-model"
            }),
            None,
            live.data.child(),
            crate::agent::subagent_host::detached_subagent_parent(&live, "stub-child")
                .expect("detached parent"),
        )
        .await
        .expect("the asynchronous spawn request is accepted");
    assert!(!accepted.is_error, "completion is asynchronous");

    let completed = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            match progress_rx.recv().await {
                Some(crate::progress::AgentProgress::SubagentCompleted {
                    agent_id,
                    output,
                    stop,
                    ..
                }) if agent_id == "task_manager_agent" => break (output, stop),
                Some(crate::progress::AgentProgress::SubagentFailed {
                    agent_id, error, ..
                }) if agent_id == "task_manager_agent" => {
                    panic!("stub should be incomplete, not an inference failure: {error}")
                }
                Some(_) => {}
                None => panic!("parent progress stream closed before stub settlement"),
            }
        }
    })
    .await
    .expect("the completion guard projects the stub to the parent");
    assert!(provider.saw(STUB_CANARY));
    assert!(completed.0.contains("tool_calls"), "{}", completed.0);
    assert!(
        completed.1.is_some(),
        "a stub is incomplete, not successful"
    );
}

struct FailingModel;

#[async_trait]
impl ChatModel<()> for FailingModel {
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
        Err(tinyinference_llm::Error::Model(
            "scripted provider outage".into(),
        ))
    }
}

fn parent_context(
    workspace_dir: &Path,
    model: Arc<dyn ChatModel<()>>,
    connected_integrations: Vec<ConnectedIntegration>,
) -> ParentExecutionContext {
    ParentExecutionContext {
        workspace_descriptor: None,
        agent_definition_id: "orchestrator".into(),
        allowed_subagent_ids: ["task_manager_agent".to_string()].into_iter().collect(),
        turn_model_source: crate::agent::tinyagents::TurnModelSource::from_model(model),
        all_tools: Arc::new(Vec::new()),
        all_tool_specs: Arc::new(Vec::new()),
        visible_tool_specs: Arc::new(Vec::new()),
        visible_tool_names: std::collections::HashSet::new(),
        subagent_tool_ceiling_names: std::collections::HashSet::new(),
        model_name: "test-model".into(),
        temperature: 0.2,
        workspace_dir: workspace_dir.to_path_buf(),
        agent_config: Default::default(),
        workflows: Arc::new(Vec::new()),
        memory_context: Arc::new(None),
        session_id: "tools-e2e-session".into(),
        channel: "test".into(),
        connected_integrations,
        tool_call_format: ToolCallFormat::Native,
        session_key: "tools-e2e".into(),
        session_parent_prefix: None,
        on_progress: None,
        run_queue: None,
        host_env: Default::default(),
    }
}

struct ScriptedModel {
    responses: Vec<(&'static str, &'static str)>,
    seen: Mutex<Vec<String>>,
}

struct PauseThenAnswerModel {
    calls: std::sync::atomic::AtomicUsize,
    seen: Mutex<Vec<String>>,
}

impl PauseThenAnswerModel {
    fn saw(&self, needle: &str) -> bool {
        self.seen
            .lock()
            .iter()
            .any(|payload| payload.contains(needle))
    }
}

#[async_trait]
impl ChatModel<()> for PauseThenAnswerModel {
    fn profile(&self) -> Option<&ModelProfile> {
        static PROFILE: std::sync::OnceLock<ModelProfile> = std::sync::OnceLock::new();
        Some(PROFILE.get_or_init(|| {
            let mut profile = ModelProfile::default();
            profile.tool_calling = true;
            profile.parallel_tool_calls = true;
            profile
        }))
    }

    async fn invoke(
        &self,
        _state: &(),
        request: ModelRequest,
    ) -> tinyinference_llm::Result<ModelResponse> {
        use std::sync::atomic::Ordering;
        let flattened = flatten_messages(&request.messages);
        self.seen.lock().push(flattened);
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            let mut response = ModelResponse::assistant("");
            response.message.tool_calls = vec![tinyinference_llm::tool::ToolCall::new(
                "pause-call",
                "ask_user_clarification",
                json!({"question": "which file should I inspect?"}),
            )];
            response.finish_reason = Some("tool_calls".into());
            Ok(response)
        } else {
            Ok(ModelResponse::assistant("paused-child-resumed-answer"))
        }
    }
}

impl ScriptedModel {
    fn new(responses: Vec<(&'static str, &'static str)>) -> Self {
        Self {
            responses,
            seen: Mutex::new(Vec::new()),
        }
    }

    fn saw(&self, needle: &str) -> bool {
        self.seen
            .lock()
            .iter()
            .any(|payload| payload.contains(needle))
    }
}

#[async_trait]
impl ChatModel<()> for ScriptedModel {
    fn profile(&self) -> Option<&ModelProfile> {
        static PROFILE: std::sync::OnceLock<ModelProfile> = std::sync::OnceLock::new();
        Some(PROFILE.get_or_init(|| {
            let mut profile = ModelProfile::default();
            profile.tool_calling = true;
            profile.parallel_tool_calls = true;
            profile
        }))
    }

    async fn invoke(
        &self,
        _state: &(),
        request: ModelRequest,
    ) -> tinyinference_llm::Result<ModelResponse> {
        let flattened = flatten_messages(&request.messages);
        self.seen.lock().push(flattened.clone());
        for (needle, answer) in &self.responses {
            if flattened.contains(needle) {
                return Ok(ModelResponse::assistant(*answer));
            }
        }
        Err(tinyinference_llm::Error::Model(format!(
            "unexpected model request: {flattened}"
        )))
    }
}

fn flatten_messages(messages: &[Message]) -> String {
    messages
        .iter()
        .map(Message::text)
        .collect::<Vec<_>>()
        .join("\n")
}

/// A model that never stops asking for the same unavailable tool, so the child
/// run can only end by being stopped.
struct LoopingToolModel;

#[async_trait]
impl ChatModel<()> for LoopingToolModel {
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
        let mut response = ModelResponse::assistant("");
        response.message.tool_calls = vec![tinyinference_llm::tool::ToolCall::new(
            "call-loop",
            "task_source_list",
            json!({}),
        )];
        response.finish_reason = Some("tool_calls".to_string());
        Ok(response)
    }
}

#[path = "tools_e2e_driver_reuse_tests.rs"]
mod driver_reuse_tests;
#[path = "tools_e2e_driver_tests.rs"]
mod driver_tests;
#[path = "tools_e2e_profile_cancel_tests.rs"]
mod profile_cancel_tests;

#[path = "tools_e2e_continuation_tests.rs"]
mod continuation_tests;

#[path = "tools_e2e_paused_cancel_tests.rs"]
mod paused_cancel_tests;
