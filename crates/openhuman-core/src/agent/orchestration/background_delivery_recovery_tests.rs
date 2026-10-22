use super::*;
use crate::core::runtime::DispatchCtx;
use crate::agent::orchestration::background_completions::{
    forget_workspace_for_test, router_for_workspace,
};

#[tokio::test]
async fn an_unexpired_completion_claim_stays_leased_after_router_reopen() {
    let _g = test_guard().await;
    let ws = workspace();
    let w = ws.path();
    record(
        w,
        "bd-claimed-restart",
        "sub-1",
        "claimed before restart",
        "thread-claimed-restart",
    )
    .await;

    let router = router_for_workspace(w);
    let claimed = router
        .claim_pending("thread-claimed-restart", usize::MAX)
        .unwrap();
    assert_eq!(claimed.len(), 1, "the first owner holds the completion");
    drop(claimed);
    drop(router);
    forget_workspace_for_test(w);

    let reopened = router_for_workspace(w);
    assert!(
        reopened
            .claim_pending("thread-claimed-restart", usize::MAX)
            .unwrap()
            .is_empty(),
        "an unexpired durable claim prevents duplicate delivery after reopening"
    );
    assert_eq!(
        pending_ids(w, "thread-claimed-restart"),
        ["sub-1"],
        "the leased completion remains pending for delivery after its visibility lease expires"
    );
}

