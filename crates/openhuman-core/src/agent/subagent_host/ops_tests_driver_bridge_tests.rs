use super::*;

#[tokio::test]
async fn prepared_subagent_runs_through_driver_jobs_dispatcher() {
    let scratch = tempfile::tempdir().expect("isolated async spawn fixture");
    let workspace = scratch.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create isolated workspace");
    let provider = ScriptedProvider::new(vec![text_response("driver result")]);
    let mut parent_context = make_parent(provider.clone(), vec![]);
    parent_context.workspace_dir = workspace;
    let mut options = with_parent(parent_context, SubagentRunOptions::default());
    options.checkpoint_dir = Some(scratch.path().join("checkpoints"));
    options.run_context.thread_id = Some("driver-spawn-thread".into());
    let parent = options.run_context.clone().into_tinyagents(
        tinyagents_harness::context::RunConfig::new("driver-spawn-parent")
            .with_thread("driver-spawn-thread"),
    );

    let (outcome, job_id) = run_subagent_via_driver_tool(
        &parent,
        make_def_named_tools(&[]),
        "return the requested result".into(),
        options,
    )
    .await
    .expect("driver-backed spawn should complete");

    assert!(matches!(outcome.status, SubagentRunStatus::Completed));
    assert_eq!(outcome.output, "driver result");
    assert!(
        !outcome.task_id.is_empty(),
        "host must retain generated task ID"
    );
    assert_eq!(provider.captured.lock().len(), 1, "child provider must run");
    let jobs = crate::core::runtime::slot_for::<
        tinyagents_orchestration::subagent::SubAgentJobRegistry,
    >(&parent.data.dispatch_ctx());
    let job = jobs.get(&job_id).expect("driver tool registered the job");
    assert_eq!(job.agent, "test");
    assert_eq!(
        job.status,
        tinyagents_orchestration::subagent::SubAgentJobStatus::Completed
    );
}

#[tokio::test]
async fn closing_a_durable_driver_session_cancels_only_its_registered_child() {
    use crate::agent::orchestration::subagent_sessions::{
        DurableSubagentSession, DurableSubagentStatus, OpenHumanTinySubagentSessionStore,
        SubagentSessionStore as HostSessionStore,
    };
    use tinyagents_orchestration::subagent::{
        SubAgentJobAction, SubAgentJobRegistry, SubAgentJobStatus, SubAgentJobsTool,
        SubagentSessionStore as TinySessionStore, SubagentTaskKey,
    };

    let scratch = tempfile::tempdir().expect("isolated driver close fixture");
    let workspace = scratch.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create isolated workspace");
    let (started_tx, mut started_rx) = tokio::sync::mpsc::channel(1);
    let (finished_tx, mut finished_rx) = tokio::sync::mpsc::channel(1);
    let (completion_tx, mut completion_rx) = tokio::sync::mpsc::channel(1);
    let release = Arc::new(tokio::sync::Notify::new());
    let provider = ScriptedProvider::new(vec![
        tool_response("blocking_close_fixture", "{}"),
        text_response("sibling completed"),
    ]);
    let mut parent_context = make_parent(
        provider.clone(),
        vec![Box::new(BlockingSubagentCloseTool {
            started: started_tx,
            release: release.clone(),
            finished: finished_tx,
        })],
    );
    parent_context.workspace_dir = workspace.clone();
    let mut definition = make_def_named_tools(&["blocking_close_fixture"]);
    definition.id = "cancelled-child".into();
    let mut options = with_parent(parent_context, SubagentRunOptions::default());
    options.task_id = Some("close-driver-task".into());
    options.thread_id = Some("close-driver-thread".into());
    options.checkpoint_dir = Some(scratch.path().join("checkpoints"));
    let mut test_config = crate::config::Config::default();
    // This lifecycle fixture exercises driver cancellation, not TinyJuice's
    // module-backed output compaction. Keep the optional component disabled so
    // the scripted child cannot wait on a module service that this unit test
    // does not install.
    test_config.components.juice = false;
    let core = crate::core::runtime::CoreContext::for_test_with_config(
        crate::core::runtime::DomainSet::full(),
        test_config,
    );
    options.run_context = options.run_context.with_core(Some(Arc::clone(&core)));
    let parent = options.run_context.clone().into_tinyagents(
        tinyagents_harness::context::RunConfig::new("close-driver-parent")
            .with_thread("close-driver-thread"),
    );
    let parent_cancellation = parent.data.child().cancellation;
    let jobs =
        (*crate::core::runtime::slot_for::<SubAgentJobRegistry>(&parent.data.dispatch_ctx()))
            .clone();
    let task_key = SubagentTaskKey::from_context(&parent, "close-driver-task", None);
    let now = chrono::Utc::now().to_rfc3339();
    let sessions = HostSessionStore::new(workspace.clone());
    sessions
        .save(&[DurableSubagentSession {
            subagent_session_id: "close-driver-session".into(),
            parent_session: "test-session".into(),
            parent_thread_id: Some("close-driver-thread".into()),
            worker_thread_id: None,
            agent_id: "cancelled-child".into(),
            display_name: None,
            model: None,
            sandbox_mode: "workspace".into(),
            action_root: None,
            task_key: "close driver lifecycle".into(),
            task_title: "Close driver lifecycle".into(),
            current_task_id: Some("close-driver-task".into()),
            lifecycle_key: Some(task_key),
            status: DurableSubagentStatus::Running,
            reusable: true,
            latest_history: None,
            latest_error: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            last_used_at: now,
        }])
        .expect("save the authenticated durable session");

    let _spawn_reference = spawn_async_via_driver_tool_with_jobs(
        &parent,
        definition,
        "must be cancelled before the child runs".into(),
        options,
        jobs.clone(),
        move |_target, result, _emit_effects| {
            let completion_tx = completion_tx.clone();
            Box::pin(async move {
                let _ = completion_tx.send(result).await;
            })
        },
    )
    .await
    .expect("background driver registers the child");
    let job_id = jobs
        .list()
        .into_iter()
        .find(|job| job.agent == "cancelled-child")
        .expect("driver child is visible in the shared registry")
        .id;
    tokio::time::timeout(std::time::Duration::from_secs(5), started_rx.recv())
        .await
        .expect("child reaches its blocking tool")
        .expect("blocking tool notification");

    let port = OpenHumanTinySubagentSessionStore::new(
        workspace.clone(),
        "test-session",
        Some("close-driver-thread".into()),
    )
    .with_jobs(jobs.clone());
    let owner = crate::agent::tinyagents::tool_registration::subagent_owner_key(
        "test-session",
        Some("close-driver-thread"),
    );
    let owner_scope: std::sync::Arc<dyn Fn(u64) -> String + Send + Sync> =
        std::sync::Arc::new(move |_| owner.clone());
    let message_store: std::sync::Arc<dyn TinySessionStore> = std::sync::Arc::new(port.clone());
    let message_dispatch: crate::agent::tinyagents::tool_registration::RegisteredToolDispatch =
        std::sync::Arc::new(
            tinyagents_orchestration::subagent::SubAgentMessageTool::new(jobs.clone())
                .with_tool_name("steer_subagent")
                .with_argument_fields("task_id", "subagent_session_id", "message", "__request_id")
                .with_lane_field("mode")
                .with_owner_scope(owner_scope.clone())
                .with_sessions(message_store, |_| "test-session".to_owned()),
        );
    let message_dispatch =
        crate::agent::tinyagents::tool_registration::TaskFirstSessionMessageDispatch::new(
            message_dispatch,
        );
    let steered = tinyagents_harness::tool::ToolDispatch::execute(
        &message_dispatch,
        &(),
        tinyagents_harness::CallId::new("steer-raw-task-id"),
        serde_json::json!({
            "task_id": "close-driver-task",
            "subagent_session_id": "foreign-session-must-not-win",
            "message": "please continue after this note",
            "mode": "collect"
        }),
        tinytools::ToolCallOptions::default(),
        &parent,
    )
    .await
    .expect("TaskFirst host adapter resolves raw task ID via the owned session port");
    let steered_payload: serde_json::Value =
        serde_json::from_str(&steered.output()).expect("message dispatch returns JSON");
    assert_eq!(steered_payload["job_id"].as_str(), Some(job_id.as_str()));
    assert_eq!(steered_payload["status"], "message_queued");

    assert!(port
        .close("close-driver-session")
        .await
        .expect("Tiny close adapter cancels then closes the durable row"));
    assert_eq!(
        jobs.get(job_id.as_str()).unwrap().status,
        SubAgentJobStatus::Running,
        "the job remains visible while cooperative cancellation unwinds"
    );
    assert!(port
        .close("close-driver-session")
        .await
        .expect("repeated close is idempotent without re-cancelling"));
    assert!(!parent_cancellation.is_cancelled(), "parent stays live");

    let tiny_store: std::sync::Arc<dyn TinySessionStore> = std::sync::Arc::new(port.clone());
    let wait = SubAgentJobsTool::new(jobs.clone())
        .with_tool_name("wait_subagent")
        .with_fixed_action(SubAgentJobAction::Await)
        .with_identifier_fields("task_id", "subagent_session_id")
        .with_await_timeout_field("timeout_secs", 1, 3600)
        .with_owner_scope(owner_scope.clone())
        .with_sessions(tiny_store, |_| "test-session".to_owned());
    let wait_result = tinyagents_harness::tool::ToolDispatch::execute(
        &wait,
        &(),
        tinyagents_harness::CallId::new("wait-cancelled-close-child"),
        serde_json::json!({"task_id":"close-driver-task","timeout_secs":1}),
        tinytools::ToolCallOptions::default(),
        &parent,
    )
    .await
    .expect("Tiny await observes close cancellation");
    assert!(
        !wait_result.is_error,
        "raw product task ID resolves through the authenticated session port: {}",
        wait_result.output()
    );
    let running_wait: serde_json::Value =
        serde_json::from_str(&wait_result.output()).expect("Tiny wait returns a JSON job snapshot");
    assert_eq!(running_wait["id"].as_str(), Some(job_id.as_str()));
    assert_eq!(running_wait["status"], "running");
    release.notify_one();
    tokio::time::timeout(std::time::Duration::from_secs(2), finished_rx.recv())
        .await
        .expect("released tool future must unwind")
        .expect("tool reports that it unwound");
    let completion = tokio::time::timeout(std::time::Duration::from_secs(2), completion_rx.recv())
        .await
        .expect("driver should finish after the tool unwinds")
        .expect("driver invokes the host completion projection");
    assert!(
        matches!(&completion, Ok(outcome) if matches!(&outcome.status, SubagentRunStatus::Cancelled)),
        "the driver should classify the released child as cancelled: {completion:?}"
    );
    let final_wait = SubAgentJobsTool::new(jobs.clone())
        .with_tool_name("wait_subagent")
        .with_fixed_action(SubAgentJobAction::Await)
        .with_identifier_fields("task_id", "subagent_session_id")
        .with_required_identifier()
        .with_owner_scope(owner_scope.clone())
        .with_sessions(std::sync::Arc::new(port.clone()), |_| {
            "test-session".to_owned()
        });
    let settled = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        tinyagents_harness::tool::ToolDispatch::execute(
            &final_wait,
            &(),
            tinyagents_harness::CallId::new("wait-cancelled-close-child-settled"),
            serde_json::json!({"task_id":"close-driver-task"}),
            tinytools::ToolCallOptions::default(),
            &parent,
        ),
    )
    .await;
    assert!(
        settled.is_ok(),
        "shared Jobs wait should settle after tool unwind: job={:?}, provider_calls={}",
        jobs.get(job_id.as_str()),
        provider.captured.lock().len()
    );
    let settled = settled
        .expect("shared Jobs wait observes cooperative cancellation settlement")
        .expect("Tiny await dispatches successfully");
    assert!(!settled.is_error, "settled Jobs wait must remain a result");
    assert_eq!(
        jobs.get(job_id.as_str()).unwrap().status,
        SubAgentJobStatus::Cancelled
    );

    let cancelled = jobs
        .list()
        .into_iter()
        .find(|job| job.agent == "cancelled-child")
        .expect("driver child is visible in the shared registry");
    assert_eq!(cancelled.status, SubAgentJobStatus::Cancelled);
    assert_eq!(
        sessions.load().unwrap()[0].status,
        DurableSubagentStatus::Closed,
        "the completion callback cannot revive a closed session"
    );
    assert_eq!(
        provider.captured.lock().len(),
        1,
        "child reaches one tool call"
    );

    let mut sibling_options = SubagentRunOptions::default();
    sibling_options.task_id = Some("close-driver-sibling-task".into());
    sibling_options.thread_id = Some("close-driver-thread".into());
    sibling_options.checkpoint_dir = Some(scratch.path().join("checkpoints"));
    sibling_options.run_context = parent.data.clone();
    let mut sibling = make_def_named_tools(&[]);
    sibling.id = "live-sibling".into();
    let sibling = run_subagent_via_driver_tool(
        &parent,
        sibling,
        "complete independently".into(),
        sibling_options,
    )
    .await
    .expect("sibling driver remains usable");
    assert!(matches!(sibling.0.status, SubagentRunStatus::Completed));
    assert_eq!(provider.captured.lock().len(), 2);
}

#[tokio::test]
async fn stopped_parent_thread_rejects_a_new_driver_spawn_before_execution() {
    let scratch = tempfile::tempdir().expect("isolated stopped-thread fixture");
    let workspace = scratch.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create isolated workspace");
    let provider = ScriptedProvider::new(vec![text_response("must not run")]);
    let mut parent_context = make_parent(provider.clone(), vec![]);
    parent_context.workspace_dir = workspace;
    let mut options = with_parent(parent_context, SubagentRunOptions::default());
    options.checkpoint_dir = Some(scratch.path().join("checkpoints"));
    options.run_context.thread_id = Some("stopped-driver-thread".into());
    let parent = options.run_context.clone().into_tinyagents(
        tinyagents_harness::context::RunConfig::new("stopped-driver-parent")
            .with_thread("stopped-driver-thread"),
    );
    let dispatch_ctx = parent.data.dispatch_ctx();
    crate::agent::orchestration::background_completions::discard_pending_for_thread(
        &dispatch_ctx,
        "stopped-driver-thread",
    );

    let result = run_subagent_via_driver_tool(
        &parent,
        make_def_named_tools(&[]),
        "return the requested result".into(),
        options,
    )
    .await;

    assert!(result.is_err(), "stopped thread must reject new children");
    assert!(
        provider.captured.lock().is_empty(),
        "the child model must not run after Stop"
    );
}

#[tokio::test]
async fn detached_driver_spawn_repairs_replayed_host_outcome_without_reexecution() {
    let scratch = tempfile::tempdir().expect("isolated async spawn fixture");
    let workspace = scratch.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create isolated workspace");
    let provider = ScriptedProvider::new(vec![text_response("durable driver result")]);
    let mut parent_context = make_parent(provider.clone(), vec![]);
    parent_context.workspace_dir = workspace;
    let parent_options = with_parent(parent_context, SubagentRunOptions::default());
    let parent = parent_options.clone().run_context.into_tinyagents(
        tinyagents_harness::context::RunConfig::new("driver-replay-parent")
            .with_thread("driver-replay-thread"),
    );

    let (first_tx, first_rx) = tokio::sync::oneshot::channel();
    let first_tx = Arc::new(std::sync::Mutex::new(Some(first_tx)));
    let mut first_options = parent_options.clone();
    first_options.task_id = Some("driver-replay-task".into());
    first_options.checkpoint_dir = Some(scratch.path().join("checkpoints"));
    first_options.run_context.thread_id = Some("driver-replay-thread".into());
    spawn_async_via_driver_tool_with_jobs(
        &parent,
        make_def_named_tools(&[]),
        "return the persisted result".into(),
        first_options,
        tinyagents_orchestration::subagent::SubAgentJobRegistry::new(),
        move |_target, result, effects| {
            let first_tx = first_tx.clone();
            Box::pin(async move {
                if let Some(first_tx) = first_tx.lock().unwrap().take() {
                    let _ = first_tx.send((result, effects));
                }
            })
        },
    )
    .await
    .expect("first detached driver spawn should register");
    let (first_result, first_effects) =
        tokio::time::timeout(std::time::Duration::from_secs(5), first_rx)
            .await
            .expect("first completion callback should not be skipped")
            .expect("first completion callback");
    let first_result = first_result.expect("first host outcome");
    assert!(first_effects);
    assert_eq!(first_result.output, "durable driver result");
    assert_eq!(provider.captured.lock().len(), 1, "child runs exactly once");

    let (replay_tx, replay_rx) = tokio::sync::oneshot::channel();
    let replay_tx = Arc::new(std::sync::Mutex::new(Some(replay_tx)));
    let mut replay_options = parent_options;
    replay_options.task_id = Some("driver-replay-task".into());
    replay_options.checkpoint_dir = Some(scratch.path().join("checkpoints"));
    replay_options.run_context.thread_id = Some("driver-replay-thread".into());
    spawn_async_via_driver_tool_with_jobs(
        &parent,
        make_def_named_tools(&[]),
        "return the persisted result".into(),
        replay_options,
        tinyagents_orchestration::subagent::SubAgentJobRegistry::new(),
        move |_target, result, effects| {
            let replay_tx = replay_tx.clone();
            Box::pin(async move {
                if let Some(replay_tx) = replay_tx.lock().unwrap().take() {
                    let _ = replay_tx.send((result, effects));
                }
            })
        },
    )
    .await
    .expect("replayed detached driver spawn should register");
    let (replay_result, replay_effects) =
        tokio::time::timeout(std::time::Duration::from_secs(5), replay_rx)
            .await
            .expect("replay repair callback should run")
            .expect("replay repair callback");
    let replay_result = replay_result.expect("durable replay must retain host result");
    assert!(!replay_effects, "terminal replay must not repeat effects");
    assert_eq!(replay_result.output, "durable driver result");
    assert_eq!(replay_result.agent_id, "test");
    assert_eq!(
        provider.captured.lock().len(),
        1,
        "replay must not execute child"
    );
}
