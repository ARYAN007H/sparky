async fn find_reusable_driver_session(
    driver: &std::sync::Arc<dyn tinyagents_orchestration::subagent::SubagentSessionStore>,
    legacy: &SubagentSessionStore,
    selector: &SubagentSessionSelector,
) -> Result<Option<DurableSubagentSession>, String> {
    let driver_selector = tinyagents_orchestration::subagent::SubagentSessionSelector {
        parent_session: selector.parent_session.clone(),
        parent_thread_id: selector.parent_thread_id.clone(),
        agent_id: selector.agent_id.clone(),
        model: selector.model.clone(),
        sandbox_mode: selector.sandbox_mode.clone(),
        action_root: selector.action_root.clone(),
        task_key: selector.task_key.clone(),
    };
    let sessions = driver
        .list_for_parent(
            &selector.parent_session,
            selector.parent_thread_id.as_deref(),
        )
        .await
        .map_err(|error| error.to_string())?;
    let Some(selected) = tinyagents_orchestration::subagent::find_reusable_session(
        sessions,
        &driver_selector,
    ) else {
        return Ok(None);
    };

    legacy
        .load()
        .map(|sessions| {
            sessions.into_iter().find(|session| {
                session.subagent_session_id == selected.subagent_session_id
                    && session.matches_selector(selector)
            })
        })
}
