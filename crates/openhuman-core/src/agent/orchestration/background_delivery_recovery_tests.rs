use super::*;
use crate::core::runtime::DispatchCtx;

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

#[tokio::test]
async fn a_mixed_batch_that_partly_gives_up_still_retries_the_rest() {
    let _g = test_guard().await;
    let ws = workspace();
    let w = ws.path();
    record(w, "bd-mixed", "sub-old", "old", "thread-mixed").await;
    let router = router_for_workspace(w);
    for _ in 0..(DEFAULT_MAX_ATTEMPTS - 1) {
        try_deliver_with(
            &DispatchCtx::ambient(),
            "thread-mixed".into(),
            router.clone(),
            |_t, _n| async move { Err::<String, String>("checkout failed".to_string()) },
            |_t, _n| async move { unreachable!("below the ceiling") },
        )
        .await;
    }
    // A newer result joins the old one, which is on its last attempt.
    record(w, "bd-mixed", "sub-new", "new", "thread-mixed").await;

    let gave_up = Arc::new(Mutex::new(String::new()));
    let sink = Arc::clone(&gave_up);
    let retry = try_deliver_with(
        &DispatchCtx::ambient(),
        "thread-mixed".into(),
        router,
        |_t, _n| async move { Err::<String, String>("checkout failed".to_string()) },
        move |_t, notice| async move {
            *sink.lock().expect("sink") = notice;
        },
    )
    .await;

    assert!(gave_up.lock().expect("sink").contains("sub-old"));
    assert!(
        retry.is_some(),
        "the newer record must still be rescheduled"
    );
    assert_eq!(pending_ids(w, "thread-mixed"), ["sub-new"]);
}
