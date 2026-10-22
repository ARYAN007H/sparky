use super::*;
use crate::agent::orchestration::background_completions::{
    forget_workspace_for_test, pending_for, record_completion, router_for_workspace, TestWorkspace,
};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::MutexGuard;

async fn test_guard() -> MutexGuard<'static, ()> {
    crate::config::TEST_ENV_LOCK.lock().await
}

async fn record(ws: &Path, session: &str, task: &str, summary: &str, thread: &str) {
    record_completion(
        &DispatchCtx::ambient(),
        ws,
        session,
        task,
        "researcher",
        summary,
        Some(thread.to_owned()),
    )
    .await;
}

#[tokio::test]
async fn try_deliver_applies_the_requested_turn_end_delay() {
    let _guard = test_guard().await;
    let workspace = TestWorkspace::new();
    let workspace_dir = workspace.path().to_path_buf();
    let thread_id = "thread-turn-end-delay";
    record(
        &workspace_dir,
        "session-turn-end-delay",
        "sub-delay",
        "done",
        thread_id,
    )
    .await;
    let ctx = DispatchCtx::ambient();
    let requested_delay = Duration::from_millis(300);
    let started = std::time::Instant::now();
    let delivered_after = Arc::new(Mutex::new(None));
    let observed = Arc::clone(&delivered_after);

    try_deliver(
        ctx.clone(),
        thread_id.to_owned(),
        DrainFence::current(&ctx),
        false,
        requested_delay,
        move |_ctx, _workspace, _thread, _notice| {
            let observed = Arc::clone(&observed);
            async move {
                *observed.lock().expect("delivery timestamp") = Some(started.elapsed());
                Ok(())
            }
        },
    )
    .await;

    let delivered_after = delivered_after
        .lock()
        .expect("delivery timestamp")
        .expect("try_deliver should invoke the turn callback");
    assert!(delivered_after >= requested_delay);
    assert!(
        delivered_after < Duration::from_secs(2),
        "requested {requested_delay:?} delay was replaced by the 3s debounce: {delivered_after:?}"
    );
    assert!(pending_for(&workspace_dir, thread_id).is_empty());
}

#[tokio::test]
async fn an_unexpired_completion_claim_stays_leased_after_router_reopen() {
    let _guard = test_guard().await;
    let workspace = TestWorkspace::new();
    let workspace_dir = workspace.path();
    record(
        workspace_dir,
        "bd-claimed-restart",
        "sub-1",
        "claimed before restart",
        "thread-claimed-restart",
    )
    .await;

    let router = router_for_workspace(workspace_dir);
    let claimed = router
        .claim_pending("thread-claimed-restart", usize::MAX)
        .expect("completion claim succeeds");
    assert_eq!(claimed.len(), 1);
    drop(claimed);
    drop(router);
    forget_workspace_for_test(workspace_dir);

    let reopened = router_for_workspace(workspace_dir);
    assert!(
        reopened
            .claim_pending("thread-claimed-restart", usize::MAX)
            .expect("reopened claim succeeds")
            .is_empty(),
        "an unexpired durable claim prevents duplicate delivery after reopening"
    );
    assert_eq!(
        pending_for(workspace_dir, "thread-claimed-restart")[0].task_id,
        "sub-1"
    );
}
