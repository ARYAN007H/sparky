// --- harness fan-out concurrency ceiling ---

#[test]
fn harness_ceiling_defaults_when_unset_or_nonsense() {
    // A malformed override must never produce a zero-permit semaphore —
    // that would deadlock every flow agent node in the process.
    for raw in [None, Some(""), Some("0"), Some("-4"), Some("lots")] {
        assert_eq!(
            super::max_parallel_harness_agents(raw),
            super::DEFAULT_MAX_PARALLEL_HARNESS_AGENTS,
            "{raw:?} should fall back to the default"
        );
    }
}

#[test]
fn harness_ceiling_honours_a_valid_override() {
    assert_eq!(super::max_parallel_harness_agents(Some("3")), 3);
    assert_eq!(super::max_parallel_harness_agents(Some(" 16 ")), 16);
}

#[tokio::test]
async fn production_harness_ceiling_is_open_and_reusable() {
    let held = super::HARNESS_AGENT_SLOTS
        .acquire()
        .await
        .expect("the production limiter must remain open");
    drop(held);
    assert!(!super::HARNESS_AGENT_SLOTS.is_closed());
}

#[tokio::test]
async fn headless_flow_agent_includes_nested_grandchild_result() {
    use async_trait::async_trait;
    use tinyinference_llm::model::{ChatModel, ModelProfile, ModelRequest, ModelResponse};

    const CHILD_PROMPT: &str = "headless-flow-child-result-marker";
    const GRANDCHILD_PROMPT: &str = "headless-flow-grandchild-result-marker";

    struct FlowChildModel {
        spawn_calls: std::sync::atomic::AtomicUsize,
        child_calls: std::sync::atomic::AtomicUsize,
        child_schema_names: std::sync::Mutex<Vec<Vec<String>>>,
        grandchild_calls: std::sync::atomic::AtomicUsize,
        nested_spawn_calls: std::sync::atomic::AtomicUsize,
        accepted_followups: std::sync::atomic::AtomicUsize,
        child_started: std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
        child_release: tokio::sync::Notify,
        root_followup: tokio::sync::Notify,
    }

    #[async_trait]
    impl ChatModel<()> for FlowChildModel {
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
            let contains_child_prompt = request.messages.iter().any(|message| {
                matches!(message, tinyinference_llm::message::Message::User(_))
                    && message.text().contains(CHILD_PROMPT)
            });
            if !contains_child_prompt
                && request
                    .messages
                    .iter()
                    .any(|message| {
                        matches!(message, tinyinference_llm::message::Message::User(_))
                            && message.text().contains(GRANDCHILD_PROMPT)
                    })
            {
                self.grandchild_calls
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if let Some(started) = self.child_started.lock().unwrap().take() {
                    let _ = started.send(());
                }
                self.child_release.notified().await;
                return Ok(ModelResponse::assistant("headless-flow-grandchild-result"));
            }
            if contains_child_prompt {
                self.child_schema_names.lock().unwrap().push(
                    request
                        .tools
                        .iter()
                        .map(|tool| tool.name.clone())
                        .collect(),
                );
                let call = self
                    .child_calls
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if call > 0 {
                    return Ok(ModelResponse::assistant(
                        "headless-flow-worker-result-with-grandchild",
                    ));
                }
                self.nested_spawn_calls
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let mut response = ModelResponse::assistant("");
                response.message.tool_calls = vec![tinyinference_llm::tool::ToolCall::new(
                    "headless-flow-grandchild-spawn",
                    "spawn_subagent",
                    serde_json::json!({
                        "agent_id": "task_manager_agent",
                        "prompt": GRANDCHILD_PROMPT,
                        "task_key": "headless-flow-grandchild-task",
                        "context": "headless flow grandchild regression",
                        "model": "test-model"
                    }),
                )];
                response.finish_reason = Some("tool_calls".into());
                return Ok(response);
            }

            if request.messages.iter().any(|message| {
                message
                    .text()
                    .contains("Accepted async sub-agent `flow_worker`")
            }) {
                self.accepted_followups
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                self.root_followup.notify_one();
                return Ok(ModelResponse::assistant("headless-flow-root-result"));
            }
            if self
                .spawn_calls
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                > 0
            {
                return Ok(ModelResponse::assistant(
                    "headless-flow-root-finished-without-accepted-child",
                ));
            }
            let mut response = ModelResponse::assistant("");
            response.message.tool_calls = vec![tinyinference_llm::tool::ToolCall::new(
                "headless-flow-spawn",
                "spawn_subagent",
                serde_json::json!({
                    "agent_id": "flow_worker",
                    "prompt": CHILD_PROMPT,
                    "task_key": "headless-flow-child-task",
                    "context": "headless flow regression",
                    "model": "test-model"
                }),
            )];
            response.finish_reason = Some("tool_calls".into());
            Ok(response)
        }
    }

    let _inference = crate::inference::inference_test_guard_async().await;
    let model = std::sync::Arc::new(FlowChildModel {
        spawn_calls: std::sync::atomic::AtomicUsize::new(0),
        child_calls: std::sync::atomic::AtomicUsize::new(0),
        child_schema_names: std::sync::Mutex::new(Vec::new()),
        grandchild_calls: std::sync::atomic::AtomicUsize::new(0),
        nested_spawn_calls: std::sync::atomic::AtomicUsize::new(0),
        accepted_followups: std::sync::atomic::AtomicUsize::new(0),
        child_started: std::sync::Mutex::new(None),
        child_release: tokio::sync::Notify::new(),
        root_followup: tokio::sync::Notify::new(),
    });
    let _model =
        crate::inference::provider::factory::test_provider_override::install_model(model.clone());
    let workspace = tempfile::tempdir().expect("flow workspace");
    let mut config = crate::config::Config::default();
    config.workspace_dir = workspace.path().to_path_buf();
    config.action_dir = workspace.path().to_path_buf();
    config.default_model = Some("test-model".into());
    config.chat_provider = Some("omlx:test-model".into());

    let builtin_definitions =
        crate::agent_registry::host_definition::AgentDefinitionRegistry::builtins_only();
    let mut orchestrator = builtin_definitions
        .get("orchestrator")
        .expect("orchestrator definition")
        .clone();
    orchestrator.subagents = vec![tinyagents_definition::SubagentEntry::AgentId(
        "flow_worker".to_string(),
    )];
    let mut worker = builtin_definitions
        .get("task_manager_agent")
        .expect("worker definition")
        .clone();
    worker.id = "flow_worker".to_string();
    worker.agent_tier = tinyagents_definition::AgentTier::Reasoning;
    worker.extra_tools.push("spawn_subagent".to_string());
    worker.subagents = vec![tinyagents_definition::SubagentEntry::AgentId(
        "task_manager_agent".to_string(),
    )];
    let definitions =
        std::sync::Arc::new(builtin_definitions.with_definitions([orchestrator, worker]));
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

    let runner = super::OpenHumanAgentRunner {
        config: std::sync::Arc::new(config),
        ctx: crate::core::runtime::DispatchCtx::new(std::sync::Arc::clone(&core)),
        approval: Default::default(),
        progress: None,
    };
    let (child_started_tx, child_started_rx) = tokio::sync::oneshot::channel();
    *model.child_started.lock().unwrap() = Some(child_started_tx);
    let jobs_core = std::sync::Arc::clone(&core);
    let core_for_journal = std::sync::Arc::clone(&core);
    let run = tokio::spawn(async move {
        crate::core::runtime::CoreContext::scope(
            core,
            Box::pin(runner.run_agent(
                "orchestrator",
                serde_json::json!({"prompt": "Delegate this task to the child."}),
                None,
            )),
        )
        .await
    });
    let grandchild_reached_model = matches!(
        tokio::time::timeout(std::time::Duration::from_secs(5), child_started_rx).await,
        Ok(Ok(()))
    );
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        model.root_followup.notified(),
    )
    .await
    .expect("the parent receives spawn acceptance and produces its final response");
    tokio::task::yield_now().await;
    let returned_before_child = run.is_finished();
    model.child_release.notify_one();
    let output = run.await.expect("flow runner task");
    let output = output.expect("headless flow agent should settle its child");
    let snapshots = crate::core::runtime::slot_for::<
        tinyagents_harness_tools::subagents::SubAgentJobRegistry,
    >(&crate::core::runtime::DispatchCtx::new(jobs_core))
    .list();

    let child_runs = snapshots
        .iter()
        .filter(|job| job.agent == "flow_worker")
        .map(|job| (job.status.clone(), job.subagent_run_id.clone()))
        .collect::<Vec<_>>();
    let grandchild_jobs = snapshots
        .iter()
        .filter(|job| job.agent == "task_manager_agent")
        .collect::<Vec<_>>();
    let diagnostic_dispatch = crate::core::runtime::DispatchCtx::new(std::sync::Arc::clone(
        &core_for_journal,
    ));
    let worker_run_id = child_runs
        .first()
        .and_then(|(_, run_id)| run_id.as_deref())
        .expect("the worker job records its own run id");
    let worker_events = crate::agent::tinyagents::journal::read_run_events(
        &diagnostic_dispatch,
        worker_run_id,
        0,
    )
    .await
    .expect("read the worker journal for the nested-spawn evidence");
    let nested_spawn_completed = worker_events.iter().any(|observation| {
        matches!(
            &observation.event,
            tinyagents_harness::events::AgentEvent::ToolCompleted {
                tool_name,
                error: None,
                ..
            } if tool_name == "spawn_subagent"
        )
    });
    let nested_spawn_outcomes = worker_events
        .iter()
        .filter_map(|observation| match &observation.event {
            tinyagents_harness::events::AgentEvent::ToolCompleted {
                tool_name,
                error,
                ..
            } if tool_name == "spawn_subagent" => Some(error.clone()),
            tinyagents_harness::events::AgentEvent::UnknownToolCall {
                requested_name,
                recovery,
                ..
            } if requested_name == "spawn_subagent" => Some(Some(recovery.clone())),
            _ => None,
        })
        .collect::<Vec<_>>();
    eprintln!(
        "NESTED_FLOW_POLICY_DIAGNOSTIC worker_calls={} schema={:?} nested_spawn_calls={} worker_jobs={:?} grandchild_jobs={:?} spawn_events={:?}",
        model.child_calls.load(std::sync::atomic::Ordering::SeqCst),
        model.child_schema_names.lock().unwrap(),
        model.nested_spawn_calls.load(std::sync::atomic::Ordering::SeqCst),
        child_runs,
        grandchild_jobs
            .iter()
            .map(|job| (&job.status, job.output.as_deref().is_some_and(|s| s.contains("headless-flow-grandchild-result"))))
            .collect::<Vec<_>>(),
        nested_spawn_outcomes,
    );
    let grandchild_run_id = grandchild_jobs
        .first()
        .and_then(|job| job.subagent_run_id.as_deref())
        .expect("the grandchild job records its own run id");
    let grandchild_events = crate::agent::tinyagents::journal::read_run_events(
        &diagnostic_dispatch,
        grandchild_run_id,
        0,
    )
    .await
    .expect("read the grandchild's durable run events");
    let grandchild_completed = grandchild_events.iter().any(|observation| {
        matches!(
            &observation.event,
            tinyagents_harness::events::AgentEvent::RunCompleted { run_id, .. }
                if run_id.as_str() == grandchild_run_id
        )
    });
    let grandchild_output = grandchild_jobs[0].output.as_deref().unwrap_or_default();
    eprintln!(
        "NESTED_FLOW_TERMINAL_DIAGNOSTIC returned_before_child={returned_before_child} status={:?} error={:?} events={:?}",
        grandchild_jobs[0].status,
        grandchild_jobs[0].error,
        grandchild_events.iter().map(|o| &o.event).collect::<Vec<_>>(),
    );

    assert_eq!(
        model.spawn_calls.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "the parent model must issue the real spawn_subagent call"
    );
    assert_eq!(
        model.child_calls.load(std::sync::atomic::Ordering::SeqCst),
        2,
        "the worker must run, spawn its grandchild, and complete through the real dispatch"
    );
    assert_eq!(
        model
            .nested_spawn_calls
            .load(std::sync::atomic::Ordering::SeqCst),
        1,
        "the worker must issue a real nested spawn"
    );
    assert_eq!(
        model
            .grandchild_calls
            .load(std::sync::atomic::Ordering::SeqCst),
        1,
        "the grandchild must run through the real model dispatch"
    );
    assert!(grandchild_reached_model, "the grandchild must reach its model callback");
    assert_eq!(grandchild_jobs.len(), 1, "one grandchild job must be admitted");
    assert_eq!(
        grandchild_jobs[0].status,
        tinyagents_orchestration::subagent::SubAgentJobStatus::Completed,
        "the grandchild must reach a terminal successful state"
    );
    assert_ne!(
        worker_run_id, grandchild_run_id,
        "worker and grandchild must have distinct native run identities"
    );
    assert!(nested_spawn_completed, "the worker journal must record the successful nested spawn");
    assert!(grandchild_completed, "the grandchild journal must record terminal completion");
    assert!(grandchild_output.contains("headless-flow-grandchild-result"));
    assert_eq!(
        model
            .accepted_followups
            .load(std::sync::atomic::Ordering::SeqCst),
        1,
        "the parent must receive the actual async spawn acceptance"
    );
    assert!(
        !returned_before_child,
        "the headless flow must remain pending after root turn until its child settles; jobs: {snapshots:?}"
    );

    assert!(
        output
            .to_string()
            .contains("headless-flow-grandchild-result"),
        "the headless flow result must include its grandchild output: {output}"
    );
}

// --- host-registered agents (agent::host_agents) ---

struct FlowsHost;

impl crate::agent::host_agents::HostAgentResolver for FlowsHost {
    fn resolve(&self, agent_id: &str) -> Option<crate::agent::host_agents::HostAgent> {
        if agent_id != "flows-host-agent" {
            return None;
        }
        let mut definition =
            crate::agent_registry::host_definition::AgentDefinitionRegistry::builtins_only()
                .get("orchestrator")
                .cloned()
                .unwrap();
        definition.id = agent_id.to_string();
        Some(crate::agent::host_agents::HostAgent {
            definition,
            config: crate::config::Config::default(),
            host_tools: None,
            hooks: Default::default(),
            context: crate::core::runtime::CoreContext::for_test(
                crate::core::runtime::DomainSet::full(),
                None,
            ),
        })
    }
}

#[test]
fn a_host_registered_agent_routes_ahead_of_the_registries() {
    let _slot = crate::agent::host_agents::tests::lock();
    let installed: std::sync::Arc<dyn crate::agent::host_agents::HostAgentResolver> =
        std::sync::Arc::new(FlowsHost);
    crate::agent::host_agents::install(std::sync::Arc::clone(&installed));
    let host = super::route_for_agent_ref("flows-host-agent");
    let other = super::route_for_agent_ref("flows-not-a-host-agent");
    crate::agent::host_agents::clear_if(&installed);
    assert_eq!(host, super::AgentRoute::HostAgent);
    assert_eq!(other, super::AgentRoute::RegistryFallback);
}
