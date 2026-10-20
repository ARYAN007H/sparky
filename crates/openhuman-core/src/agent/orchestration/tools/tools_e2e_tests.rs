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
