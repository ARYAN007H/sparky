use super::*;

#[tokio::test]
async fn continue_subagent_resumes_paused_job_through_shared_driver() {
    use crate::agent::orchestration::tools::{ContinueSubagentTool, SpawnSubagentTool};
    use tinyagents_orchestration::subagent::{SubAgentJobRegistry, SubAgentJobStatus};

    let _ = AgentDefinitionRegistry::init_global_builtins();
    let workspace = tempfile::TempDir::new().expect("workspace");
    let provider = Arc::new(PauseThenAnswerModel {
        calls: std::sync::atomic::AtomicUsize::new(0),
        seen: Mutex::new(Vec::new()),
    });
    let ask_tool: Box<dyn tinytools::Tool> =
        Box::new(tinyagents_harness::tools::AskClarificationTool::new());
    let ask_spec = Arc::new(ask_tool.spec());
    let mut parent = parent_context(workspace.path(), provider.clone(), vec![]);
    parent.all_tools = Arc::new(vec![ask_tool]);
    parent.all_tool_specs = Arc::new(vec![ask_spec.clone()]);
    parent.visible_tool_specs = Arc::new(vec![ask_spec]);
    parent
        .visible_tool_names
        .insert("ask_user_clarification".into());
    parent
        .subagent_tool_ceiling_names
        .insert("ask_user_clarification".into());
    let live =
        crate::agent::orchestration::tools::live_parent_run_context("continue-paused-e2e", parent);
    let paused = SpawnSubagentTool::new()
        .execute_with_live_parent_context(
            json!({
                "agent_id": "task_manager_agent",
                "prompt": "Investigate the pause-resume fixture",
                "model": "test-model",
                "blocking": true
            }),
            None,
            live.data.child(),
            Some(&live),
        )
        .await
        .expect("initial spawn");
    assert!(!paused.is_error, "{}", paused.output());
    assert_eq!(provider.calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    let task_id = paused
        .output()
        .lines()
        .find_map(|line| line.strip_prefix("task_id: "))
        .unwrap_or_else(|| panic!("pause envelope carries task id: {}", paused.output()))
        .to_owned();
    assert!(paused.output().contains("which file should I inspect?"));

    let jobs = crate::core::runtime::slot_for::<SubAgentJobRegistry>(&live.data.dispatch_ctx());
    let paused_job = jobs
        .list()
        .into_iter()
        .find(|job| {
            job.extensions
                .get("task_id")
                .and_then(serde_json::Value::as_str)
                == Some(task_id.as_str())
        })
        .expect("initial pause is registered in shared Jobs");
    assert_eq!(paused_job.status, SubAgentJobStatus::AwaitingInput);

    let resumed = ContinueSubagentTool::new()
        .execute_with_live_parent_context(
            json!({
                "task_id": task_id,
                "agent_id": "task_manager_agent",
                "message": "Inspect the README and continue."
            }),
            None,
            live.data.child(),
            Some(&live),
        )
        .await
        .expect("continue");
    assert!(!resumed.is_error, "{}", resumed.output());
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
}

#[tokio::test]
async fn delegate_graph_runs_every_stage_through_shared_driver_jobs() {
    use crate::agent::orchestration::tools::DelegateGraphTool;
    use crate::agent_registry::host_definition::AgentDefinitionRegistry;
    use crate::core::runtime::{ContextOverlay, CoreContext, DomainSet};
    use crate::tools::toolpacks::ToolGroups;
    use tinyagents_orchestration::subagent::{SubAgentJobRegistry, SubAgentJobStatus};

    let _ = AgentDefinitionRegistry::init_global_builtins();
    let workspace = tempfile::TempDir::new().expect("workspace");
    let provider = Arc::new(ScriptedModel::new(vec![
        ("Produce a short, concrete", "1. inspect the fixture"),
        ("Carry out the plan below", "graph-stage-execution-canary"),
        ("Review the result below", "APPROVE\nThe result is correct."),
    ]));
    let mut parent = parent_context(workspace.path(), provider.clone(), vec![]);
    parent.session_id = "delegate-graph-jobs-e2e".into();
    let mut test_definition = AgentDefinitionRegistry::builtins_only()
        .get("task_manager_agent")
        .expect("task manager definition")
        .clone();
    test_definition.model = tinyagents_definition::ModelSpec::Exact("test-model".into());
    let definitions =
        Arc::new(AgentDefinitionRegistry::builtins_only().with_definitions([test_definition]));
    let mut config = crate::config::Config::default();
    config.workspace_dir = workspace.path().to_path_buf();
    let core = CoreContext::for_test_with_config(DomainSet::full(), config.clone()).derive_with(
        ContextOverlay::new(config, DomainSet::full(), ToolGroups::none()).definitions(definitions),
    );
    // `OpenHumanRunContext::with_core` retains a weak handle; keep this strong
    // owner alive through the graph execution so the scoped definition
    // catalogue is the one the live parent carries.
    let live = Arc::new(
        crate::agent::tinyagents::host::OpenHumanRunContext::new()
            .with_core(Some(Arc::clone(&core)))
            .with_parent(parent)
            .into_tinyagents(tinyagents_harness::context::RunConfig::new(
                "delegate-graph-jobs-e2e",
            )),
    );
    let result = DelegateGraphTool::new()
        .execute_with_live_parent_context(
            json!({
                "agent_id": "task_manager_agent",
                "task": "Verify the driver-backed graph stage fixture",
                "max_revisions": 1
            }),
            None,
            live.data.child(),
            Some(Arc::clone(&live)),
        )
        .await
        .expect("graph delegation returns a tool result");

    assert!(!result.is_error, "{}", result.output());
    assert!(result.output().contains("graph-stage-execution-canary"));
    assert_eq!(provider.seen.lock().len(), 3);

    let jobs = crate::core::runtime::slot_for::<SubAgentJobRegistry>(&live.data.dispatch_ctx());
    let completed_stages = jobs
        .list()
        .into_iter()
        .filter(|job| {
            job.agent == "task_manager_agent"
                && job.status == SubAgentJobStatus::Completed
                && job.output.as_deref().is_some_and(|output| {
                    output.contains("1. inspect the fixture")
                        || output.contains("graph-stage-execution-canary")
                        || output.contains("APPROVE")
                })
        })
        .count();
    assert_eq!(
        completed_stages, 3,
        "each graph stage is registered in Jobs"
    );
}
