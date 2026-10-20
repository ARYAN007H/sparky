use super::SpawnSubagentTool;
use crate::agent::harness::ParentExecutionContext;
use crate::agent_registry::host_definition::AgentDefinitionRegistry;
use crate::integrations::ConnectedIntegration;
use async_trait::async_trait;
use parking_lot::Mutex;
use serde_json::json;
use std::path::Path;
use std::sync::Arc;
use tinyagents_harness::context::RunConfig;
use tinyagents_session::transcript::TranscriptMessage;
use tinyinference_llm::message::Message;
use tinyinference_llm::model::{ChatModel, ModelProfile, ModelRequest, ModelResponse};
use tinytools_agent::dialect::ToolCallFormat;

const SPAWN_SUBAGENT_CANARY: &str = "tool-e2e-spawn-subagent-canary";

#[tokio::test]
async fn spawn_subagent_returns_async_reference_and_runs_child_e2e() {
    let workspace = tempfile::TempDir::new().expect("workspace");
    let provider = Arc::new(ScriptedModel::new(vec![(
        SPAWN_SUBAGENT_CANARY,
        "spawn-subagent-child-answer",
    )]));

    let live = super::live_parent_run_context(
        "spawn-subagent-e2e",
        parent_context(workspace.path(), provider.clone(), vec![]),
    );
    // Boxed: the inline spawn future is too large for a test thread's stack.
    let result = Box::pin(
        SpawnSubagentTool::new().execute_with_live_parent_context(
            json!({
                "agent_id": "task_manager_agent",
                "prompt": format!("Investigate {SPAWN_SUBAGENT_CANARY}"),
                "context": "parent supplied context",
                "model": "test-model",
            }),
            None,
            live.data.child(),
            crate::agent::subagent_host::detached_subagent_parent(&live, "spawn-subagent-child")
                .expect("detached parent"),
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
                "spawn-subagent-e2e",
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
    assert_eq!(terminal.output.as_deref(), Some("spawn-subagent-child-answer"));
}

/// A child the harness stops early (failure breaker / iteration cap) is not a
/// finished delegation: the parent's tool result must be a failure, so its own
/// failure policy and its trace see it, and the `SubagentCompleted` progress
/// event must say how the child was stopped.
#[tokio::test(start_paused = true)]
async fn spawn_subagent_reports_a_stopped_child_as_a_failed_tool_result() {
    let _ = AgentDefinitionRegistry::init_global_builtins();
    let workspace = tempfile::TempDir::new().expect("workspace");
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel(512);
    let parent = parent_context(workspace.path(), Arc::new(LoopingToolModel), vec![]);
    // The live parent run a real turn hands the tool, with the turn's
    // progress channel on the child's run context.
    let mut live = super::live_parent_run_context("stopped-child-test", parent);
    // The shared driver derives its child context from the live parent, so
    // install the observer there as a real harness turn would.
    live.data.progress = Some(progress_tx.clone());
    let run_context = live.data.child();
    // Boxed: the inline spawn future is too large for a test thread's
    // stack (the trait `execute` path boxes it the same way).
    let tool = SpawnSubagentTool::new();
    let result = Box::pin(
        tool.execute_with_live_parent_context(
            json!({
                "agent_id": "task_manager_agent",
                "prompt": "List the task sources",
                "model": "test-model",
                "blocking": true
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
        result.is_error,
        "a stopped child must fail the parent's tool call: {}",
        result.output()
    );
    assert!(
        result.output().contains("[SUBAGENT_INCOMPLETE]"),
        "{}",
        result.output()
    );

    let mut stop = None;
    while let Ok(event) = progress_rx.try_recv() {
        if let crate::progress::AgentProgress::SubagentCompleted { stop: s, .. } = event {
            stop = Some(s);
        }
    }
    let stop = stop
        .expect("SubagentCompleted was emitted")
        .expect("SubagentCompleted carries the child's stop");
    assert!(matches!(
        stop.kind,
        tinyagents_harness::turn_stop::TurnStopKind::Breaker
            | tinyagents_harness::turn_stop::TurnStopKind::IterationCap
    ));
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
    assert!(!accepted.is_error, "child failure is delivered asynchronously");

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
    assert!(failure.contains("scripted provider outage"), "{failure}");
    assert!(!failure.contains("completed successfully"), "{failure}");
}

#[tokio::test]
async fn unexecuted_tool_call_stub_is_projected_as_incomplete_not_success() {
    const STUB_CANARY: &str = "unexecuted-tool-call-stub-canary";
    let _ = AgentDefinitionRegistry::init_global_builtins();
    let workspace = tempfile::TempDir::new().expect("workspace");
    let provider = Arc::new(ScriptedModel::new(vec![
        (
            STUB_CANARY,
            r#"{"tool_calls":[{"type":"function","function":{"name":"missing","arguments":"{}"}}]}"#,
        ),
    ]));
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
    assert!(completed.1.is_some(), "a stub is incomplete, not successful");
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

#[path = "tools_e2e_driver_tests.rs"]
mod driver_tests;

#[path = "tools_e2e_continuation_tests.rs"]
mod continuation_tests;
