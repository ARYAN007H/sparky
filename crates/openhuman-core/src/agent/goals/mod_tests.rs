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
    assert!(Arc::ptr_eq(&first, &repeated));
    drop(first);
    drop(repeated);
    let reacquired = goal_store(first_workspace.path(), &first_dispatch);
    let isolated = goal_store(first_workspace.path(), &second_dispatch);
    let other_workspace = goal_store(second_workspace.path(), &first_dispatch);

    assert!(!Arc::ptr_eq(&reacquired, &isolated));
    assert!(!Arc::ptr_eq(&reacquired, &other_workspace));

    reacquired
        .set("same-thread", "agent a objective", None)
        .await
        .expect("write first agent goal");
    assert!(isolated
        .get("same-thread")
        .await
        .expect("read second agent goal")
        .is_none());
}

#[tokio::test]
async fn goal_tool_with_context_uses_the_business_goal_store() {
    struct ThreadContext(&'static str);
    impl tinytools::ToolRunContext for ThreadContext {
        fn thread_id(&self) -> Option<&str> {
            Some(self.0)
        }
    }

    let workspace = tempfile::tempdir().expect("workspace");
    let context = crate::core::runtime::CoreContext::for_test(
        crate::core::runtime::DomainSet::full(),
        Some(workspace.path().to_path_buf()),
    );
    crate::core::runtime::CoreContext::scope(context, async {
        let dispatch = crate::core::runtime::DispatchCtx::ambient();
        let mut tools = goal_tools(workspace.path());
        let set = tools
            .iter_mut()
            .find(|tool| tool.name() == "goal_set")
            .expect("goal_set tool");
        let thread = ThreadContext("goal-tool-business-store");
        let result = set
            .execute_with_context(
                serde_json::json!({ "objective": "persist through the host store" }),
                ToolCallOptions::default(),
                Some(&thread),
            )
            .await
            .expect("execute goal_set");
        assert!(!result.is_error, "{}", result.output());

        let stored = goal_store(workspace.path(), &dispatch)
            .get("goal-tool-business-store")
            .await
            .expect("read business goal store")
            .expect("goal was persisted");
        assert_eq!(stored.objective, "persist through the host store");

        let no_thread = set.execute(serde_json::json!({ "objective": "no thread" })).await
            .expect("execute without a thread");
        assert!(no_thread.is_error);
        assert!(goal_store(workspace.path(), &dispatch)
            .get("goal-tool-business-store")
            .await
            .expect("confirm existing goal")
            .is_some());
    })
    .await;
}
