use super::*;

#[tokio::test]
async fn goal_store_is_reused_within_a_context_and_isolated_between_agents() {
    let first_workspace = tempfile::tempdir().expect("first workspace");
    let second_workspace = tempfile::tempdir().expect("second workspace");
    let first_context = crate::core::runtime::CoreContext::for_test(
        crate::core::runtime::DomainSet::full(),
        Some(first_workspace.path().to_path_buf()),
    )
    .for_agent("agent-a");
    let second_context = crate::core::runtime::CoreContext::for_test(
        crate::core::runtime::DomainSet::full(),
        Some(first_workspace.path().to_path_buf()),
    )
    .for_agent("agent-b");
    let first_dispatch = crate::core::runtime::DispatchCtx::new(first_context);
    let second_dispatch = crate::core::runtime::DispatchCtx::new(second_context);

    let first = goal_store(first_workspace.path(), &first_dispatch);
    let repeated = goal_store(first_workspace.path(), &first_dispatch);
    let isolated = goal_store(first_workspace.path(), &second_dispatch);
    let other_workspace = goal_store(second_workspace.path(), &first_dispatch);

    assert!(Arc::ptr_eq(&first, &repeated));
    assert!(!Arc::ptr_eq(&first, &isolated));
    assert!(!Arc::ptr_eq(&first, &other_workspace));

    first
        .set("same-thread", "agent a objective", None)
        .await
        .expect("write first agent goal");
    assert!(isolated
        .get("same-thread")
        .await
        .expect("read second agent goal")
        .is_none());
}
