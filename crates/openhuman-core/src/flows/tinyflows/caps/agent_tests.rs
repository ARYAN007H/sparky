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
async fn headless_flow_agent_includes_its_spawned_child_result() {
    use async_trait::async_trait;
    use tinyinference_llm::model::{ChatModel, ModelProfile, ModelRequest, ModelResponse};

    const CHILD_PROMPT: &str = "headless-flow-child-result-marker";

    struct FlowChildModel {
        spawn_calls: std::sync::atomic::AtomicUsize,
        child_calls: std::sync::atomic::AtomicUsize,
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
            if request
                .messages
                .iter()
                .any(|message| message.text().contains(CHILD_PROMPT))
            {
                self.child_calls
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                return Ok(ModelResponse::assistant("headless-flow-child-result"));
            }

            if request.messages.iter().any(|message| {
                message
                    .text()
                    .contains("Accepted async sub-agent `task_manager_agent`")
            }) {
                return Ok(ModelResponse::assistant("headless-flow-root-result"));
            }
            self.spawn_calls
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let mut response = ModelResponse::assistant("");
            response.message.tool_calls = vec![tinyinference_llm::tool::ToolCall::new(
                "headless-flow-spawn",
                "spawn_subagent",
                serde_json::json!({
                    "agent_id": "task_manager_agent",
                    "prompt": CHILD_PROMPT,
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
    });
    let _model = crate::inference::provider::factory::test_provider_override::install_model(
        model.clone(),
    );
    let workspace = tempfile::tempdir().expect("flow workspace");
    let mut config = crate::config::Config::default();
    config.workspace_dir = workspace.path().to_path_buf();
    config.action_dir = workspace.path().to_path_buf();
    config.default_model = Some("test-model".into());
    config.chat_provider = Some("omlx:test-model".into());

    let definitions = std::sync::Arc::new(
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

    let runner = super::OpenHumanAgentRunner {
        config: std::sync::Arc::new(config),
        ctx: crate::core::runtime::DispatchCtx::new(std::sync::Arc::clone(&core)),
        approval: Default::default(),
    };
    let output = crate::core::runtime::CoreContext::scope(
        core,
        Box::pin(tinyflows::caps::AgentRunner::run_agent(
            &runner,
            "orchestrator",
            serde_json::json!({"prompt": "Delegate this task to the child."}),
            None,
        )),
    )
    .await
    .expect("headless flow agent should settle its child");

    assert_eq!(
        model.spawn_calls.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "the parent model must issue the real spawn_subagent call"
    );
    assert_eq!(
        model.child_calls.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "the detached child must run through the real model dispatch"
    );

    assert!(
        output.to_string().contains("headless-flow-child-result"),
        "the headless flow result must include its spawned child output: {output}"
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
