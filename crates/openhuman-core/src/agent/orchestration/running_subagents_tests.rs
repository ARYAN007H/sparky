use super::*;

/// The context these tests dispatch under: the calling task's own.
fn cx() -> crate::core::runtime::DispatchCtx {
    crate::core::runtime::DispatchCtx::ambient()
}

use crate::agent::orchestration::running_subagents::resolve::resume_ref_for_task;
use crate::agent::orchestration::running_subagents::resolve::task_id_for_session;
use crate::agent::orchestration::running_subagents::roster::snapshot_for_parent;
use crate::agent::queued_turn::QueuedTurn;
use std::path::PathBuf;
use std::sync::Arc;
use tinyagents_harness::ids::TaskId;
use tinyagents_harness::run_queue::RunQueue;
use tinyagents_orchestration::subagent::FinishedOutcome;
use tinyagents_orchestration::subagent::{
    DetachedSubagentStatus, WaitError, DETACHED_LEDGER_TIMEOUT_MS,
};
use tinyagents_tasks::OrchestrationTaskStatus;
use tokio::sync::watch;
use tokio::task::AbortHandle;

#[path = "running_subagents_cancel_tests.rs"]
mod cancel_tests;

#[path = "running_subagents_wire_tests.rs"]
mod wire_tests;

#[path = "running_subagents_tenant_tests.rs"]
mod tenant_tests;

#[path = "running_subagents_projection_tests.rs"]
mod projection_tests;

/// Serializes every test that touches the global [`REGISTRY`]. We reuse the
/// crate-wide `TEST_ENV_LOCK` (rather than a module-local mutex) because the
/// destructive `cancel_all` path is also reachable from the `threads::ops`
/// tests — those hold the same lock, so this prevents a purge there from
/// wiping entries a test here is mid-way through.
fn test_guard() -> tokio::sync::MutexGuard<'static, ()> {
    // Recover from a poisoned guard so one panicking test doesn't cascade.
    crate::config::TEST_ENV_LOCK.blocking_lock()
}

async fn test_guard_async() -> tokio::sync::MutexGuard<'static, ()> {
    // Recover from a poisoned guard so one panicking test doesn't cascade.
    crate::config::TEST_ENV_LOCK.lock().await
}

fn dummy_abort() -> AbortHandle {
    tokio::spawn(async {}).abort_handle()
}

fn run_queue() -> Arc<RunQueue<QueuedTurn>> {
    Arc::new(RunQueue::new())
}

/// Per-process-run unique workspace for the detached task store.
///
/// The task store is now a durable JSONL file under the given workspace
/// (issue #4249). Pointing tests at the shared `std::env::temp_dir()` leaked
/// records **across** test-process runs: a `task-ledger-1` left `Completed`
/// by a prior run would make `record_spawned`'s insert/`mark_running` no-op
/// (id already terminal), so this run would observe the stale terminal status
/// instead of `Running`. A fresh temp dir per process run keeps the store
/// hermetic; task ids are unique across tests so a single shared dir is safe
/// within a run. The `TempDir` lives for the whole process (cleaned at exit).
fn test_workspace() -> PathBuf {
    static WORKSPACE: std::sync::LazyLock<tempfile::TempDir> = std::sync::LazyLock::new(|| {
        tempfile::tempdir().expect("create hermetic test task-store workspace")
    });
    WORKSPACE.path().to_path_buf()
}

/// Register a sub-agent for tests, returning the status sender so the test
/// can drive completion. Keeping the sender alive keeps the channel open.
fn register_test(
    task_id: &str,
    parent_session: &str,
    rq: Arc<RunQueue<QueuedTurn>>,
) -> watch::Sender<DetachedSubagentStatus> {
    register_test_with_thread(task_id, parent_session, None, rq)
}

/// Like [`register_test`] but lets a test set the parent thread id so it can
/// exercise [`cancel_for_thread`].
fn register_test_with_thread(
    task_id: &str,
    parent_session: &str,
    parent_thread_id: Option<&str>,
    rq: Arc<RunQueue<QueuedTurn>>,
) -> watch::Sender<DetachedSubagentStatus> {
    let (tx, rx) = status_channel();
    register(
        &cx(),
        task_id.into(),
        "researcher".into(),
        parent_session.into(),
        None,
        None,
        test_workspace(),
        parent_thread_id.map(Into::into),
        rq,
        dummy_abort(),
        rx,
    )
    .expect("fixture registration admitted");
    tx
}

#[tokio::test]
async fn late_registration_rejects_a_parent_generation_reopened_after_spawn() {
    let _guard = test_guard_async().await;
    let ctx = cx();
    let workspace = tempfile::tempdir().expect("workspace");
    let thread_id = "late-register-old-generation";
    let old_epoch = crate::agent::orchestration::background_completions::CompletionTarget::new(
        ctx.clone(),
        workspace.path().to_path_buf(),
        "late-register-parent".into(),
        Some(thread_id.into()),
    )
    .expect("initial spawn admitted")
    .parent_epoch()
    .unwrap();
    let router = crate::agent::orchestration::background_completions::router_for_workspace_ctx(
        &ctx,
        workspace.path(),
    );
    router
        .cancel_parent(thread_id)
        .expect("stop old generation");
    router.resume_parent(thread_id);
    assert!(!router
        .parent_generation_admitted(thread_id, old_epoch)
        .unwrap());

    let child = tokio::spawn(std::future::pending::<()>());
    let (_status_tx, status_rx) = status_channel();
    let result = register_with_parent_epoch(
        &ctx,
        "late-register-old-task".into(),
        "researcher".into(),
        "late-register-parent".into(),
        None,
        None,
        workspace.path().to_path_buf(),
        Some(thread_id.into()),
        Some(old_epoch),
        run_queue(),
        child.abort_handle(),
        status_rx,
    );
    assert!(matches!(result, Err(RegistrationError::ParentLifecycle(_))));
    assert!(child.await.unwrap_err().is_cancelled());
    assert!(cancel_by_task(&ctx, "late-register-old-task").is_none());
}

#[tokio::test]
async fn task_store_records_spawn_complete_and_cancel() {
    let _guard = test_guard_async().await;
    // Spawn → the ledger sees a running SubAgent task scoped to the parent.
    let tx = register_test("task-ledger-1", "ledger-parent", run_queue());
    let running = task_records(Some("ledger-parent"));
    assert!(
        running
            .iter()
            .any(|r| r.spec.task_id.as_str() == "task-ledger-1"
                && r.spec.parent_run_id.as_ref().map(|id| id.as_str()) == Some("ledger-parent")
                && r.spec.root_run_id.as_ref().map(|id| id.as_str()) == Some("ledger-parent")
                && r.spec.timeout_ms == Some(DETACHED_LEDGER_TIMEOUT_MS)
                && r.status == OrchestrationTaskStatus::Running),
        "spawned sub-agent is recorded Running: {running:?}"
    );

    // Publish a terminal status → the watcher mirrors Completed into the store.
    tx.send(DetachedSubagentStatus::Completed {
        output: "done".into(),
        iterations: 2,
    })
    .unwrap();
    // Let the watcher task observe the change.
    for _ in 0..50 {
        tokio::task::yield_now().await;
        if task_records(None)
            .iter()
            .any(|r| r.spec.task_id.as_str() == "task-ledger-1" && r.is_terminal())
        {
            break;
        }
    }
    let after = task_records(None);
    let rec = after
        .iter()
        .find(|r| r.spec.task_id.as_str() == "task-ledger-1")
        .expect("ledger record present");
    assert_eq!(rec.status, OrchestrationTaskStatus::Completed);

    // A second sub-agent that gets cancelled is recorded Cancelled.
    let _tx2 = register_test("task-ledger-2", "ledger-parent", run_queue());
    assert!(cancel_by_task(&cx(), "task-ledger-2").is_some());
    let cancelled = task_records(None)
        .into_iter()
        .find(|r| r.spec.task_id.as_str() == "task-ledger-2")
        .expect("cancelled record present");
    assert_eq!(cancelled.status, OrchestrationTaskStatus::Cancelled);

    prune("task-ledger-1");
}

#[tokio::test]
async fn task_id_for_session_enforces_parent_ownership() {
    let _guard = test_guard_async().await;
    let rq = run_queue();
    let (tx, rx) = status_channel();
    register(
        &cx(),
        "task-session".into(),
        "researcher".into(),
        "session-owner".into(),
        None,
        Some("subsess-1".into()),
        test_workspace(),
        Some("thread-1".into()),
        rq,
        dummy_abort(),
        rx,
    )
    .expect("fixture registration admitted");

    assert_eq!(
        task_id_for_session(&cx(), "subsess-1", "session-owner").unwrap(),
        "task-session"
    );
    assert!(matches!(
        task_id_for_session(&cx(), "subsess-1", "session-other"),
        Err(WaitError::NotOwned)
    ));
    let _ = tx.send(DetachedSubagentStatus::Completed {
        output: "done".into(),
        iterations: 1,
    });
    prune("task-session");
}

#[tokio::test]
async fn snapshot_and_block_scope_to_parent_and_reflect_live_status() {
    let _guard = test_guard_async().await;
    let (tx_a, rx_a) = status_channel();
    register(
        &cx(),
        "task-fleet-a".into(),
        "researcher".into(),
        "fleet-parent".into(),
        None,
        Some("subsess-a".into()),
        test_workspace(),
        None,
        run_queue(),
        dummy_abort(),
        rx_a,
    )
    .expect("fixture registration admitted");
    let (tx_b, rx_b) = status_channel();
    register(
        &cx(),
        "task-fleet-b".into(),
        "code_executor".into(),
        "fleet-parent".into(),
        None,
        Some("subsess-b".into()),
        test_workspace(),
        None,
        run_queue(),
        dummy_abort(),
        rx_b,
    )
    .expect("fixture registration admitted");
    // A worker owned by a different parent must not leak into the snapshot.
    let (tx_other, rx_other) = status_channel();
    register(
        &cx(),
        "task-fleet-other".into(),
        "researcher".into(),
        "other-parent".into(),
        None,
        Some("subsess-other".into()),
        test_workspace(),
        None,
        run_queue(),
        dummy_abort(),
        rx_other,
    )
    .expect("fixture registration admitted");

    // `b` pauses awaiting the user; `a` stays running.
    tx_b.send(DetachedSubagentStatus::AwaitingUser {
        question: "which repo?".into(),
    })
    .unwrap();

    let snap = snapshot_for_parent(&cx(), "fleet-parent");
    assert_eq!(snap.len(), 2, "only this parent's workers: {snap:?}");
    // Ordered by agent_id then task_id → code_executor before researcher.
    assert_eq!(snap[0].agent_id, "code_executor");
    assert_eq!(snap[0].status, "awaiting_user");
    assert_eq!(snap[1].agent_id, "researcher");
    assert_eq!(snap[1].status, "running");

    let block = active_subagents_context_block(&cx(), "fleet-parent", &test_workspace())
        .expect("block present");
    assert!(block.contains("[active_subagents]"));
    assert!(block.contains("reuse its task_key with spawn_subagent"));
    assert!(block.contains("You have 2 sub-agent worker(s)"));
    assert!(block.contains("session=subsess-a"));
    assert!(block.contains("session=subsess-b · task=task-fleet-b · status=awaiting_user"));
    assert!(block.ends_with("[/active_subagents]\n\n"));

    // A parent with no registered workers gets no block (no perturbation).
    assert!(active_subagents_context_block(&cx(), "nobody-here", &test_workspace(),).is_none());

    // The shipped orchestrator has no wait/steer/close tools (#5701): the
    // guidance must not name them and must say results arrive on their own.
    {
        let block = active_subagents_context_block(&cx(), "fleet-parent", &test_workspace())
            .expect("block present");
        for name in [
            "wait_subagent",
            "steer_subagent",
            "close_subagent",
            "wait_loop",
        ] {
            assert!(
                !block.contains(name),
                "{name} named for a parent without it:\n{block}"
            );
        }
        assert!(block.contains("delivered automatically"));
    }

    // Durable-store fallback: a session persisted by an EARLIER turn /
    // process lifetime (empty live registry for this parent) must still
    // surface in the roster, so a cold-booted orchestrator can resume by
    // subagent_session_id instead of re-delegating from scratch.
    {
        use crate::agent::orchestration::subagent_sessions::{
            self, SubagentSessionSelector, SubagentSessionStore, SubagentSessionUpsert,
        };
        use crate::agent::subagent_host::SubagentRunStatus;
        let durable_ws = tempfile::tempdir().expect("durable roster tempdir");
        let store = SubagentSessionStore {
            workspace_dir: durable_ws.path().to_path_buf(),
        };
        let session = subagent_sessions::upsert_running(
            &store,
            SubagentSessionUpsert {
                selector: SubagentSessionSelector {
                    parent_session: "cold-parent".into(),
                    parent_thread_id: Some("thread-cold".into()),
                    agent_id: "workflow_builder".into(),
                    model: None,
                    sandbox_mode: "None".into(),
                    action_root: None,
                    task_key: "daily-x-trending".into(),
                },
                display_name: Some("Workflow Builder".into()),
                task_title: "Daily X trending email workflow".into(),
                worker_thread_id: None,
                task_id: "task-cold-1".into(),
                lifecycle_key: None,
                parent_epoch: None,
            },
            None,
        )
        .expect("upsert durable session");
        subagent_sessions::mark_finished(
            &store,
            &session.subagent_session_id,
            "task-cold-1",
            &SubagentRunStatus::Completed,
            Vec::new(),
        )
        .expect("mark idle");

        let block = active_subagents_context_block(&cx(), "cold-parent", durable_ws.path())
            .expect("durable-only roster present");
        assert!(block.contains(&format!("session={}", session.subagent_session_id)));
        assert!(block.contains("status=idle"));
        assert!(block.contains("about: Daily X trending email workflow"));
        // Other parents' durable sessions must not leak in.
        assert!(
            active_subagents_context_block(&cx(), "unrelated-parent", durable_ws.path(),).is_none()
        );
    }

    let _ = tx_a.send(DetachedSubagentStatus::Completed {
        output: "x".into(),
        iterations: 1,
    });
    let _ = tx_other.send(DetachedSubagentStatus::Completed {
        output: "x".into(),
        iterations: 1,
    });
    prune("task-fleet-a");
    prune("task-fleet-b");
    prune("task-fleet-other");
}

#[tokio::test]
async fn resume_ref_for_task_includes_resume_fields_and_enforces_ownership() {
    let _guard = test_guard_async().await;
    let (tx, rx) = status_channel();
    register(
        &cx(),
        "task-resume".into(),
        "researcher".into(),
        "session-owner".into(),
        None,
        Some("subsess-resume".into()),
        test_workspace(),
        Some("thread-1".into()),
        run_queue(),
        dummy_abort(),
        rx,
    )
    .expect("fixture registration admitted");

    let reference =
        resume_ref_for_task(&cx(), "task-resume", "session-owner").expect("resume reference");
    assert_eq!(reference.task_id, "task-resume");
    assert_eq!(reference.agent_id, "researcher");
    assert_eq!(
        reference.subagent_session_id.as_deref(),
        Some("subsess-resume")
    );
    assert!(matches!(
        resume_ref_for_task(&cx(), "task-resume", "session-other"),
        Err(WaitError::NotOwned)
    ));

    let _ = tx.send(DetachedSubagentStatus::Completed {
        output: "done".into(),
        iterations: 1,
    });
    prune("task-resume");
}

#[tokio::test]
async fn task_id_for_session_prefers_live_task_over_terminal_task() {
    let _guard = test_guard_async().await;
    let (old_tx, old_rx) = status_channel();
    register(
        &cx(),
        "task-old".into(),
        "researcher".into(),
        "session-owner".into(),
        None,
        Some("subsess-live".into()),
        test_workspace(),
        Some("thread-1".into()),
        run_queue(),
        dummy_abort(),
        old_rx,
    )
    .expect("fixture registration admitted");
    let _ = old_tx.send(DetachedSubagentStatus::Completed {
        output: "old".into(),
        iterations: 1,
    });
    let (_new_tx, new_rx) = status_channel();
    register(
        &cx(),
        "task-new".into(),
        "researcher".into(),
        "session-owner".into(),
        None,
        Some("subsess-live".into()),
        test_workspace(),
        Some("thread-1".into()),
        run_queue(),
        dummy_abort(),
        new_rx,
    )
    .expect("fixture registration admitted");

    assert_eq!(
        task_id_for_session(&cx(), "subsess-live", "session-owner").unwrap(),
        "task-new"
    );
    prune("task-old");
    prune("task-new");
}

#[tokio::test]
async fn cancel_for_thread_aborts_only_matching_entries() {
    let _guard = test_guard_async().await;
    let rq = run_queue();
    let _a = register_test_with_thread("task-tA-1", "session-A", Some("thread-X"), rq.clone());
    let _b = register_test_with_thread("task-tA-2", "session-A", Some("thread-X"), rq.clone());
    // Different thread — must survive.
    let _c = register_test_with_thread("task-tB", "session-A", Some("thread-Y"), rq.clone());
    // Headless (no parent thread) — must survive.
    let _d = register_test_with_thread("task-headless", "session-A", None, rq);

    let cancelled = cancel_for_thread(&cx(), "thread-X");
    assert_eq!(cancelled, 2, "both thread-X entries should be cancelled");

    // Idempotent: a second pass cancels nothing.
    assert_eq!(cancel_for_thread(&cx(), "thread-X"), 0);

    prune("task-tB");
    prune("task-headless");
}

#[tokio::test]
async fn cancel_by_task_returns_metadata_and_removes_entry() {
    let _guard = test_guard_async().await;
    let rq = run_queue();
    let _tx = register_test_with_thread("task-cbt", "session-Z", Some("thread-cbt"), rq.clone());
    let task_id = TaskId::new("task-cbt");

    let meta = cancel_by_task(&cx(), "task-cbt").expect("known task should cancel");
    assert_eq!(
        meta.already_finished, None,
        "a running task is a real cancel"
    );
    assert_eq!(meta.agent_id, "researcher");
    assert_eq!(meta.parent_session, "session-Z");
    assert_eq!(meta.parent_thread_id.as_deref(), Some("thread-cbt"));
    // Entry is gone and a second cancel is a no-op.
    assert!(cancel_by_task(&cx(), "task-cbt").is_none());
    // Unknown ids are simply None.
    assert!(cancel_by_task(&cx(), "never-existed").is_none());
}
