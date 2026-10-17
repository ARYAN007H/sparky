use super::*;

#[tokio::test]
async fn force_fresh_cancels_old_driver_before_creating_a_new_session() {
    use crate::agent::orchestration::subagent_sessions::{
        DurableSubagentStatus, SubagentSessionStore,
    };
    use crate::core::runtime::{ContextOverlay, CoreContext, DomainSet};
    use crate::tools::toolpacks::ToolGroups;
    use tinyagents_harness::tool::ToolDispatch;
    use tinyagents_orchestration::subagent::{
        SubAgentJobRegistry, SubAgentJobStatus, SubAgentJobsTool,
    };

    struct GatedModel {
        calls: std::sync::atomic::AtomicUsize,
        first_started: tokio::sync::mpsc::Sender<()>,
        release_first: Arc<tokio::sync::Notify>,
        second_started: tokio::sync::mpsc::Sender<()>,
        release_second: Arc<tokio::sync::Notify>,
    }

    #[async_trait]
    impl ChatModel<()> for GatedModel {
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
            _request: ModelRequest,
        ) -> tinyinference_llm::Result<ModelResponse> {
            use std::sync::atomic::Ordering;
            match self.calls.fetch_add(1, Ordering::SeqCst) {
                0 => {
                    let _ = self.first_started.send(()).await;
                    self.release_first.notified().await;
                    Ok(ModelResponse::assistant("old child released"))
                }
                1 => {
                    let _ = self.second_started.send(()).await;
                    self.release_second.notified().await;
                    Ok(ModelResponse::assistant("fresh child completed"))
                }
                other => panic!("unexpected child model call {other}"),
            }
        }
    }

    let scratch = tempfile::tempdir().expect("isolated force-fresh fixture");
    let workspace = scratch.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    let (first_started_tx, mut first_started_rx) = tokio::sync::mpsc::channel(1);
    let (second_started_tx, mut second_started_rx) = tokio::sync::mpsc::channel(1);
    let release_first = Arc::new(tokio::sync::Notify::new());
    let release_second = Arc::new(tokio::sync::Notify::new());
    let provider = Arc::new(GatedModel {
        calls: std::sync::atomic::AtomicUsize::new(0),
        first_started: first_started_tx,
        release_first: release_first.clone(),
        second_started: second_started_tx,
        release_second: release_second.clone(),
    });
    let mut parent = parent_context(workspace.as_path(), provider.clone(), vec![]);
    parent.workspace_dir = workspace.clone();
    parent.session_id = "force-fresh-parent-session".into();

    let definitions = Arc::new(AgentDefinitionRegistry::builtins_only());
    let mut config = crate::config::Config::default();
    config.workspace_dir = workspace.clone();
    config.components.juice = false;
    let core = CoreContext::for_test_with_config(DomainSet::full(), config.clone()).derive_with(
        ContextOverlay::new(config, DomainSet::full(), ToolGroups::none()).definitions(definitions),
    );
    let mut host_context = OpenHumanRunContext::new()
        .with_core(Some(Arc::clone(&core)))
        .with_parent(parent);
    host_context.thread_id = Some("force-fresh-parent-thread".into());
    let live = RunContext::<OpenHumanRunContext>::new(
        tinyagents_harness::context::RunConfig::new("force-fresh-parent")
            .with_thread("force-fresh-parent-thread"),
        host_context,
    );
    async fn spawn(
        live: &RunContext<OpenHumanRunContext>,
        args: serde_json::Value,
    ) -> tinytools::ToolResult {
        let detached_data = live.data.detached_child();
        let cancellation = detached_data.cancellation.clone();
        let detached = live
            .child(
                tinyagents_harness::context::RunConfig::new(format!(
                    "force-fresh-child-{}",
                    uuid::Uuid::new_v4()
                )),
                detached_data,
            )
            .expect("detached child context")
            .with_cancellation(cancellation);
        crate::agent::orchestration::tools::SpawnAsyncSubagentTool::new()
            .execute_with_live_parent_context(args, None, live.data.child(), detached)
            .await
            .expect("spawn tool executes")
    }

    let first = spawn(
        &live,
        json!({
            "agent_id": "task_manager_agent",
            "prompt": "start the old lifecycle canary",
            "model": "test-model",
            "task_key": "force-fresh-canary"
        }),
    )
    .await;
    assert!(!first.is_error, "{}", first.output());
    let accepted_ref = first
        .output()
        .split("[async_subagent_ref]\n")
        .nth(1)
        .and_then(|value| value.split("\n[/async_subagent_ref]").next())
        .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok())
        .expect("first accepted envelope contains its task ID");
    let old_task_id = accepted_ref["task_id"]
        .as_str()
        .expect("accepted task ID")
        .to_owned();
    tokio::time::timeout(std::time::Duration::from_secs(5), first_started_rx.recv())
        .await
        .expect("old child reaches the deterministic provider gate")
        .expect("old child start notification");
    let jobs = crate::core::runtime::slot_for::<SubAgentJobRegistry>(&live.data.dispatch_ctx());
    let old_job = jobs
        .list()
        .into_iter()
        .find(|job| job.agent == "task_manager_agent" && job.status == SubAgentJobStatus::Running)
        .expect("first detached child is registered in shared Jobs");
    let store = SubagentSessionStore::new(workspace.clone());
    let before = store
        .load()
        .expect("first child session persisted")
        .into_iter()
        .find(|session| session.current_task_id.as_deref() == Some(old_task_id.as_str()))
        .expect("durable row links to the live child");
    assert_eq!(before.status, DurableSubagentStatus::Running);

    let fresh = spawn(
        &live,
        json!({
            "agent_id": "task_manager_agent",
            "prompt": "replace the old lifecycle canary",
            "model": "test-model",
            "task_key": "force-fresh-canary",
            "fresh": true
        }),
    )
    .await;
    assert!(!fresh.is_error, "{}", fresh.output());
    let fresh_ref = fresh
        .output()
        .split("[async_subagent_ref]\n")
        .nth(1)
        .and_then(|value| value.split("\n[/async_subagent_ref]").next())
        .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok())
        .expect("fresh accepted envelope contains its task ID");
    let fresh_task_id = fresh_ref["task_id"]
        .as_str()
        .expect("fresh task ID")
        .to_owned();
    tokio::time::timeout(std::time::Duration::from_secs(5), second_started_rx.recv())
        .await
        .expect("replacement child is actually dispatched")
        .expect("replacement child start notification");
    let rows = store.load().expect("replacement session persisted");
    let closed_old = rows
        .iter()
        .find(|session| session.subagent_session_id == before.subagent_session_id)
        .expect("fresh launch retains the old durable session");
    assert_eq!(closed_old.status, DurableSubagentStatus::Closed);
    let replacement = rows
        .iter()
        .find(|session| session.current_task_id.as_deref() == Some(fresh_task_id.as_str()))
        .expect("fresh launch writes a distinct durable session and task");
    assert_eq!(replacement.status, DurableSubagentStatus::Running);
    assert_ne!(replacement.subagent_session_id, before.subagent_session_id);
    assert!(
        !live.data.cancellation.is_cancelled(),
        "fresh replacement must not cancel the parent turn"
    );

    release_first.notify_one();
    release_second.notify_one();
    let session_port =
        crate::agent::orchestration::subagent_sessions::OpenHumanTinySubagentSessionStore::new(
            workspace.clone(),
            "force-fresh-parent-session",
            Some("force-fresh-parent-thread".into()),
        );
    let wait = SubAgentJobsTool::new((*jobs).clone())
        .with_fixed_action(tinyagents_orchestration::subagent::SubAgentJobAction::Await)
        .with_identifier_fields("task_id", "subagent_session_id")
        .with_sessions(Arc::new(session_port), |_| {
            "force-fresh-parent-session".to_owned()
        })
        .with_owner_scope(Arc::new({
            let owner = crate::agent::tinyagents::tool_registration::subagent_owner_key(
                &workspace,
                "force-fresh-parent-session",
                Some("force-fresh-parent-thread"),
            );
            move |_| owner.clone()
        }));
    let settled = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        ToolDispatch::execute(
            &wait,
            &(),
            tinyagents_harness::CallId::new("wait-force-fresh-old-child"),
            json!({"task_id": old_task_id}),
            tinytools::ToolCallOptions::default(),
            &live,
        ),
    )
    .await
    .expect("old child settles after its provider future is released")
    .expect("shared Jobs await succeeds");
    assert!(!settled.is_error, "{}", settled.output());
    let old_terminal: serde_json::Value =
        serde_json::from_str(&settled.output()).expect("wait returns job JSON");
    assert_eq!(old_terminal["status"], "cancelled");
    assert_eq!(
        jobs.get(&old_job.id.to_string())
            .expect("old child remains registered")
            .status,
        SubAgentJobStatus::Cancelled
    );
    let new_settled = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        ToolDispatch::execute(
            &wait,
            &(),
            tinyagents_harness::CallId::new("wait-force-fresh-new-child"),
            json!({"task_id": fresh_task_id}),
            tinytools::ToolCallOptions::default(),
            &live,
        ),
    )
    .await
    .expect("replacement child settles")
    .expect("replacement shared Jobs await succeeds");
    assert!(!new_settled.is_error, "{}", new_settled.output());
    let new_terminal: serde_json::Value =
        serde_json::from_str(&new_settled.output()).expect("replacement wait returns job JSON");
    assert_eq!(new_terminal["status"], "completed");
    assert_eq!(new_terminal["extensions"]["task_id"], fresh_task_id);
    let new_job_id = new_terminal["id"].as_str().expect("generic job ID");
    assert_eq!(
        jobs.get(new_job_id)
            .expect("replacement remains registered")
            .status,
        SubAgentJobStatus::Completed
    );
    assert_eq!(provider.calls.load(std::sync::atomic::Ordering::SeqCst), 2);
    let after_rows = store.load().expect("final durable state");
    let old_after = after_rows
        .iter()
        .find(|session| session.subagent_session_id == before.subagent_session_id)
        .expect("closed old durable row remains present");
    assert_eq!(old_after.status, DurableSubagentStatus::Closed);
    assert_eq!(old_after.current_task_id, before.current_task_id);
    let new_after = after_rows
        .iter()
        .find(|session| session.subagent_session_id == replacement.subagent_session_id)
        .expect("replacement durable row remains present");
    assert_eq!(new_after.status, DurableSubagentStatus::Idle);
    assert_eq!(
        new_after.current_task_id.as_deref(),
        Some(fresh_ref["task_id"].as_str().unwrap())
    );
}

#[tokio::test]
async fn driver_job_controls_are_isolated_by_workspace_with_shared_runtime_state() {
    use crate::agent::subagent_host::{run_subagent_via_driver_tool, SubagentRunOptions};
    use crate::agent::tinyagents::host::OpenHumanRunContext;
    use crate::core::runtime::{ContextOverlay, CoreContext, DomainSet};
    use crate::tools::toolpacks::ToolGroups;
    use tinyagents_harness::context::RunContext;
    use tinyagents_harness::tool::ToolDispatch;
    use tinyagents_orchestration::subagent::{
        SubAgentJobAction, SubAgentJobRegistry, SubAgentJobsTool,
    };

    struct PausedModel {
        started: tokio::sync::mpsc::UnboundedSender<()>,
        release: Arc<tokio::sync::Notify>,
    }

    #[async_trait]
    impl ChatModel<()> for PausedModel {
        async fn invoke(
            &self,
            _state: &(),
            _request: ModelRequest,
        ) -> tinyinference_llm::Result<ModelResponse> {
            let _ = self.started.send(());
            self.release.notified().await;
            Ok(ModelResponse::assistant("workspace child finished"))
        }
    }

    let scratch = tempfile::tempdir().expect("workspace owner isolation fixture");
    let workspace_a = scratch.path().join("workspace-a");
    let workspace_b = scratch.path().join("workspace-b");
    std::fs::create_dir_all(&workspace_a).expect("create workspace A");
    std::fs::create_dir_all(&workspace_b).expect("create workspace B");
    let mut base_config = crate::config::Config::default();
    base_config.workspace_dir = workspace_a.clone();
    base_config.components.juice = false;
    let base = CoreContext::for_test_with_config(DomainSet::full(), base_config.clone());
    let definitions = Arc::new(AgentDefinitionRegistry::builtins_only());
    let mut config_a = base_config.clone();
    config_a.workspace_dir = workspace_a.clone();
    let mut config_b = base_config;
    config_b.workspace_dir = workspace_b.clone();
    let core_a = base.derive_with(
        ContextOverlay::new(config_a, DomainSet::full(), ToolGroups::none())
            .definitions(definitions.clone()),
    );
    let core_b = base.derive_with(
        ContextOverlay::new(config_b, DomainSet::full(), ToolGroups::none())
            .definitions(definitions),
    );

    let (started_tx, mut started_rx) = tokio::sync::mpsc::unbounded_channel();
    let release = Arc::new(tokio::sync::Notify::new());
    let model: Arc<dyn ChatModel<()>> = Arc::new(PausedModel {
        started: started_tx,
        release: release.clone(),
    });
    let build_parent = |workspace: &std::path::Path, core: &Arc<CoreContext>| {
        let mut product_parent = parent_context(workspace, model.clone(), vec![]);
        product_parent.workspace_dir = workspace.to_path_buf();
        product_parent.session_id = "same-parent-session".into();
        let mut options = SubagentRunOptions::default();
        options.run_context.parent = Some(product_parent);
        options.task_id = Some("same-raw-task-id".into());
        options.thread_id = Some("same-parent-thread".into());
        options.run_context = options
            .run_context
            .with_core(Some(Arc::clone(core)));
        let run_id = workspace
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("workspace");
        let run = options.run_context.clone().into_tinyagents(
            tinyagents_harness::context::RunConfig::new(format!("parent-{run_id}"))
                .with_thread("same-parent-thread"),
        );
        (options, run)
    };
    let (options_a, parent_a) = build_parent(&workspace_a, &core_a);
    let (options_b, parent_b) = build_parent(&workspace_b, &core_b);
    let jobs_a = crate::core::runtime::slot_for::<SubAgentJobRegistry>(&parent_a.data.dispatch_ctx());
    let jobs_b = crate::core::runtime::slot_for::<SubAgentJobRegistry>(&parent_b.data.dispatch_ctx());
    assert!(
        Arc::ptr_eq(&jobs_a, &jobs_b),
        "the two workspace overlays must exercise one shared registry"
    );

    let definition_a = AgentDefinitionRegistry::builtins_only()
        .get("task_manager_agent")
        .expect("task manager definition")
        .clone();
    let mut definition_b = definition_a.clone();
    let child_a = tokio::spawn(run_subagent_via_driver_tool(
        &parent_a,
        definition_a,
        "run in workspace A".into(),
        options_a,
    ));
    let child_b = tokio::spawn(run_subagent_via_driver_tool(
        &parent_b,
        definition_b,
        "run in workspace B".into(),
        options_b,
    ));
    for _ in 0..2 {
        tokio::time::timeout(std::time::Duration::from_secs(5), started_rx.recv())
            .await
            .expect("both driver children enter the gated model")
            .expect("model start notification");
    }

    let owner_a = crate::agent::tinyagents::tool_registration::subagent_owner_key(
        &workspace_a,
        "same-parent-session",
        Some("same-parent-thread"),
    );
    let owner_b = crate::agent::tinyagents::tool_registration::subagent_owner_key(
        &workspace_b,
        "same-parent-session",
        Some("same-parent-thread"),
    );
    let query = |owner: String| {
        SubAgentJobsTool::new((*jobs_a).clone())
            .with_fixed_action(SubAgentJobAction::Query)
            .with_owner_scope(Arc::new(move |_| owner.clone()))
    };
    let list_a = ToolDispatch::execute(
        &query(owner_a.clone()),
        &(),
        tinyagents_harness::CallId::new("workspace-a-list"),
        serde_json::json!({}),
        tinytools::ToolCallOptions::default(),
        &parent_a,
    )
    .await
    .expect("workspace A list dispatch");
    let list_a: serde_json::Value = serde_json::from_str(&list_a.output()).unwrap();
    let list_b = ToolDispatch::execute(
        &query(owner_b.clone()),
        &(),
        tinyagents_harness::CallId::new("workspace-b-list"),
        serde_json::json!({}),
        tinytools::ToolCallOptions::default(),
        &parent_b,
    )
    .await
    .expect("workspace B list dispatch");
    let list_b: serde_json::Value = serde_json::from_str(&list_b.output()).unwrap();
    let jobs_a_rows = list_a.as_array().expect("A query returns rows");
    let jobs_b_rows = list_b.as_array().expect("B query returns rows");
    assert_eq!(jobs_a_rows.len(), 1, "A sees only its child: {list_a}");
    assert_eq!(jobs_b_rows.len(), 1, "B sees only its child: {list_b}");
    let id_a = jobs_a_rows[0]["id"].as_str().unwrap().to_owned();
    let id_b = jobs_b_rows[0]["id"].as_str().unwrap().to_owned();
    assert_ne!(id_a, id_b, "full lifecycle scope keeps raw-ID reuse distinct");

    let cancel_as_a = SubAgentJobsTool::new((*jobs_a).clone())
        .recover_selector_errors()
        .with_fixed_action(SubAgentJobAction::Cancel)
        .with_owner_scope(Arc::new(move |_| owner_a.clone()));
    let denied = ToolDispatch::execute(
        &cancel_as_a,
        &(),
        tinyagents_harness::CallId::new("workspace-a-cancel-b"),
        serde_json::json!({"job_id":id_b}),
        tinytools::ToolCallOptions::default(),
        &parent_a,
    )
    .await;
    assert!(denied.is_err(), "A must not acquire B's workspace child");
    let job_b = jobs_b.get(&id_b).expect("B's child remains registered");
    assert_eq!(job_b.status, tinyagents_orchestration::subagent::SubAgentJobStatus::Running);

    let cancel_as_b = SubAgentJobsTool::new((*jobs_b).clone())
        .with_fixed_action(SubAgentJobAction::Cancel)
        .with_owner_scope(Arc::new(move |_| owner_b.clone()));
    let cancelled = ToolDispatch::execute(
        &cancel_as_b,
        &(),
        tinyagents_harness::CallId::new("workspace-b-cancel-own"),
        serde_json::json!({"job_id":id_b}),
        tinytools::ToolCallOptions::default(),
        &parent_b,
    )
    .await
    .expect("B can control its own child");
    assert!(!cancelled.is_error, "{}", cancelled.output());
    release.notify_waiters();
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), child_a)
        .await
        .expect("A child settles");
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), child_b)
        .await
        .expect("B child settles");
}

#[tokio::test]
async fn stopped_async_spawn_is_refused_before_progress_or_session_effects() {
    use crate::agent::tinyagents::host::OpenHumanRunContext;
    use crate::core::runtime::{ContextOverlay, CoreContext, DomainSet};
    use crate::tools::toolpacks::ToolGroups;
    use tinyagents_harness::context::RunContext;

    let scratch = tempfile::tempdir().expect("isolated stopped-spawn fixture");
    let workspace = scratch.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    let provider = Arc::new(ScriptedModel::new(vec![]));
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel(8);
    let mut parent = parent_context(workspace.as_path(), provider.clone(), vec![]);
    parent.workspace_dir = workspace.clone();
    parent.session_id = "stopped-async-parent-session".into();
    parent.on_progress = Some(progress_tx);

    let definitions = Arc::new(AgentDefinitionRegistry::builtins_only());
    let mut config = crate::config::Config::default();
    config.workspace_dir = workspace.clone();
    config.components.juice = false;
    let core = CoreContext::for_test_with_config(DomainSet::full(), config.clone()).derive_with(
        ContextOverlay::new(config, DomainSet::full(), ToolGroups::none()).definitions(definitions),
    );
    let thread_id = "stopped-async-parent-thread";
    let mut host_context = OpenHumanRunContext::new()
        .with_core(Some(Arc::clone(&core)))
        .with_parent(parent);
    host_context.thread_id = Some(thread_id.into());
    let live = RunContext::<OpenHumanRunContext>::new(
        tinyagents_harness::context::RunConfig::new("stopped-async-parent").with_thread(thread_id),
        host_context,
    );
    let dispatch = live.data.dispatch_ctx();
    crate::agent::orchestration::background_completions::discard_pending_for_thread(
        &dispatch, thread_id,
    );
    let detached_data = live.data.detached_child();
    let detached = live
        .child(
            tinyagents_harness::context::RunConfig::new("stopped-async-child"),
            detached_data.clone(),
        )
        .expect("detached child context")
        .with_cancellation(detached_data.cancellation.clone());

    let result = crate::agent::orchestration::tools::SpawnAsyncSubagentTool::new()
        .execute_with_live_parent_context(
            json!({
                "agent_id": "task_manager_agent",
                "prompt": "must be refused before dispatch",
                "model": "test-model"
            }),
            None,
            live.data.child(),
            detached,
        )
        .await
        .expect("tool execution");

    assert!(
        result.is_error,
        "a stopped thread must reject new child work"
    );
    assert!(
        result.output().contains("stopped or deleted"),
        "{}",
        result.output()
    );
    assert!(
        provider.seen.lock().is_empty(),
        "refusal must not dispatch the model"
    );
    assert!(
        progress_rx.try_recv().is_err(),
        "refusal must not publish spawn progress"
    );
    let jobs = crate::core::runtime::slot_for::<
        tinyagents_orchestration::subagent::SubAgentJobRegistry,
    >(&dispatch);
    assert!(
        jobs.list().is_empty(),
        "refusal must not register a shared job"
    );
    assert!(
        crate::agent::orchestration::subagent_sessions::SubagentSessionStore::new(
            workspace.clone()
        )
        .load()
        .expect("read isolated session store")
        .is_empty(),
        "refusal must not persist an accepted session"
    );
    crate::agent::orchestration::background_completions::resume_stopped_thread(
        &dispatch, thread_id,
    );
}

#[tokio::test]
async fn deleted_thread_marker_survives_router_restart_and_refuses_async_spawn() {
    use crate::agent::orchestration::background_completions::{
        self, BackgroundAgentOutcome, TestWorkspace,
    };
    use crate::agent::tinyagents::host::OpenHumanRunContext;
    use crate::core::runtime::{ContextOverlay, CoreContext, DomainSet};
    use crate::tools::toolpacks::ToolGroups;
    use tinyagents_harness::context::RunContext;

    let workspace = TestWorkspace::new();
    let provider = Arc::new(ScriptedModel::new(vec![]));
    let thread_id = "deleted-async-parent-thread";
    let parent_session = "deleted-async-parent-session";
    let make_live = |progress| {
        let mut parent = parent_context(workspace.path(), provider.clone(), vec![]);
        parent.workspace_dir = workspace.path().to_path_buf();
        parent.session_id = parent_session.into();
        parent.on_progress = progress;

        let definitions = Arc::new(AgentDefinitionRegistry::builtins_only());
        let mut config = crate::config::Config::default();
        config.workspace_dir = workspace.path().to_path_buf();
        config.components.juice = false;
        let core = CoreContext::for_test_with_config(DomainSet::full(), config.clone())
            .derive_with(
                ContextOverlay::new(config, DomainSet::full(), ToolGroups::none())
                    .definitions(definitions),
            );
        let mut host_context = OpenHumanRunContext::new()
            .with_core(Some(Arc::clone(&core)))
            .with_parent(parent);
        host_context.thread_id = Some(thread_id.into());
        (
            core,
            RunContext::<OpenHumanRunContext>::new(
                tinyagents_harness::context::RunConfig::new("deleted-async-parent")
                    .with_thread(thread_id),
                host_context,
            ),
        )
    };

    let completion_store: Arc<dyn tinyagents_tasks::CompletionStore> =
        Arc::new(tinyagents_tasks::InMemoryCompletionStore::new());
    background_completions::install_store_for_test(workspace.path(), completion_store.clone());

    let (_before_delete_core, before_delete) = make_live(None);
    let original_dispatch = before_delete.data.dispatch_ctx();
    background_completions::record_outcome(
        &original_dispatch,
        workspace.path(),
        parent_session,
        "pre-delete-child",
        "task_manager_agent",
        "completion queued before deletion",
        Some(thread_id.into()),
        BackgroundAgentOutcome::Completed,
    )
    .await;
    assert_eq!(
        background_completions::pending_for(workspace.path(), thread_id).len(),
        1,
        "the fixture starts with a durable pending completion"
    );
    background_completions::discard_for_thread(&original_dispatch, workspace.path(), thread_id);
    assert!(background_completions::pending_for(workspace.path(), thread_id).is_empty());

    // Drop process-local router and thread gates, then reopen the same backing
    // store as a fresh process would. The in-memory store stands in for the
    // durable store injected by the host; only process-local state is reset.
    background_completions::forget_workspace_for_test(workspace.path());
    background_completions::install_store_for_test(workspace.path(), completion_store.clone());

    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel(8);
    let (_restarted_core, restarted) = make_live(Some(progress_tx));
    let detached_data = restarted.data.detached_child();
    let detached = restarted
        .child(
            tinyagents_harness::context::RunConfig::new("deleted-async-child"),
            detached_data.clone(),
        )
        .expect("detached child context")
        .with_cancellation(detached_data.cancellation.clone());
    let result = crate::agent::orchestration::tools::SpawnAsyncSubagentTool::new()
        .execute_with_live_parent_context(
            json!({
                "agent_id": "task_manager_agent",
                "prompt": "must remain refused after restart",
                "model": "test-model"
            }),
            None,
            restarted.data.child(),
            detached,
        )
        .await
        .expect("spawn tool execution");

    assert!(result.is_error, "deleted-thread spawn must be refused");
    assert!(
        result.output().contains("stopped or deleted"),
        "{}",
        result.output()
    );
    assert!(
        provider.seen.lock().is_empty(),
        "refusal never calls the model"
    );
    assert!(
        progress_rx.try_recv().is_err(),
        "refusal emits no spawn card"
    );
    assert!(
        crate::agent::orchestration::subagent_sessions::SubagentSessionStore::new(
            workspace.path().to_path_buf()
        )
        .load()
        .expect("session store readable")
        .is_empty(),
        "refusal writes no durable child session"
    );
    let jobs = crate::core::runtime::slot_for::<
        tinyagents_orchestration::subagent::SubAgentJobRegistry,
    >(&restarted.data.dispatch_ctx());
    assert!(jobs.list().is_empty(), "refusal registers no shared job");

    // Clear the process-local deletion gate hydrated during refused spawn so
    // the late-result check exercises the persisted marker by itself.
    background_completions::forget_workspace_for_test(workspace.path());
    background_completions::install_store_for_test(workspace.path(), completion_store);
    background_completions::record_outcome(
        &restarted.data.dispatch_ctx(),
        workspace.path(),
        parent_session,
        "late-child-after-restart",
        "task_manager_agent",
        "must stay suppressed",
        Some(thread_id.into()),
        BackgroundAgentOutcome::Completed,
    )
    .await;
    assert!(background_completions::pending_for(workspace.path(), thread_id).is_empty());
}

#[tokio::test]
async fn detached_registration_failure_finishes_the_announced_product_card() {
    use crate::agent::orchestration::subagent_sessions::{
        DurableSubagentStatus, SubagentSessionStore,
    };
    use crate::agent::tinyagents::host::OpenHumanRunContext;
    use crate::core::runtime::{ContextOverlay, CoreContext, DomainSet};
    use crate::tools::toolpacks::ToolGroups;
    use tinyagents_harness::context::RunContext;

    let scratch = tempfile::tempdir().expect("isolated registration-failure fixture");
    let workspace = scratch.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    let provider = Arc::new(ScriptedModel::new(vec![]));
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel(8);
    let mut parent = parent_context(workspace.as_path(), provider.clone(), vec![]);
    parent.workspace_dir = workspace.clone();
    parent.session_id = "registration-failure-parent-session".into();
    parent.on_progress = Some(progress_tx);

    let definitions = Arc::new(AgentDefinitionRegistry::builtins_only());
    let mut config = crate::config::Config::default();
    config.workspace_dir = workspace.clone();
    config.components.juice = false;
    let core = CoreContext::for_test_with_config(DomainSet::full(), config.clone()).derive_with(
        ContextOverlay::new(config, DomainSet::full(), ToolGroups::none()).definitions(definitions),
    );
    let thread_id = "registration-failure-parent-thread";
    let mut host_context = OpenHumanRunContext::new()
        .with_core(Some(Arc::clone(&core)))
        .with_parent(parent);
    host_context.thread_id = Some(thread_id.into());
    let live = RunContext::<OpenHumanRunContext>::new(
        tinyagents_harness::context::RunConfig::new("registration-failure-parent")
            .with_thread(thread_id)
            .with_max_depth(1),
        host_context,
    );
    let detached_data = live.data.detached_child();
    let detached = live
        .child(
            tinyagents_harness::context::RunConfig::new("registration-failure-detached-child")
                .with_max_depth(1),
            detached_data.clone(),
        )
        .expect("the already-detached invocation is allowed")
        .with_cancellation(detached_data.cancellation.clone());

    let result = crate::agent::orchestration::tools::SpawnAsyncSubagentTool::new()
        .execute_with_live_parent_context(
            json!({
                "agent_id": "task_manager_agent",
                "prompt": "registration fails before provider dispatch",
                "model": "test-model"
            }),
            None,
            live.data.child(),
            detached,
        )
        .await
        .expect("tool execution");

    assert!(result.is_error, "depth rejection is a failed registration");
    let events = std::iter::from_fn(|| progress_rx.try_recv().ok()).collect::<Vec<_>>();
    assert!(
        matches!(
            events.as_slice(),
            [
                crate::progress::AgentProgress::SubagentSpawned { .. },
                crate::progress::AgentProgress::SubagentFailed { .. }
            ]
        ),
        "the announced product card must end in failure: {events:?}"
    );
    let store = SubagentSessionStore::new(workspace.clone());
    let rows = store.load().expect("session row is durable");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, DurableSubagentStatus::Failed);
    assert!(
        provider.seen.lock().is_empty(),
        "registration error must not dispatch the model"
    );
    let jobs = crate::core::runtime::slot_for::<
        tinyagents_orchestration::subagent::SubAgentJobRegistry,
    >(&live.data.dispatch_ctx());
    assert!(
        jobs.list().is_empty(),
        "failed request construction must not register a job"
    );
}
