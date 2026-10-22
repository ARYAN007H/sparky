async fn find_reusable_driver_session(
    driver: &std::sync::Arc<dyn tinyagents_orchestration::subagent::SubagentSessionStore>,
    ctx: &DispatchCtx,
    workspace_dir: &std::path::Path,
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
    let parent_epoch = subagent_sessions::parent_epoch_for_context(
        ctx,
        workspace_dir,
        selected.subagent_session_id.clone(),
    )
    .map_err(|error| error.to_string())?;
    Ok(Some(subagent_sessions::host_projection(
        selected,
        parent_epoch,
    )))
}

async fn upsert_driver_session(
    driver: &std::sync::Arc<dyn tinyagents_orchestration::subagent::SubagentSessionStore>,
    ctx: &DispatchCtx,
    workspace_dir: &std::path::Path,
    upsert: SubagentSessionUpsert,
    force_fresh: bool,
) -> Result<DurableSubagentSession, String> {
    let lifecycle_key = upsert
        .lifecycle_key
        .ok_or_else(|| "driver session upsert requires a lifecycle key".to_owned())?;
    let parent_epoch = upsert.parent_epoch;
    let tiny_selector = tinyagents_orchestration::subagent::SubagentSessionSelector {
        parent_session: upsert.selector.parent_session,
        parent_thread_id: upsert.selector.parent_thread_id,
        agent_id: upsert.selector.agent_id,
        model: upsert.selector.model,
        sandbox_mode: upsert.selector.sandbox_mode,
        action_root: upsert.selector.action_root,
        task_key: upsert.selector.task_key,
    };
    let (session, _) = driver
        .upsert_running(tinyagents_orchestration::subagent::SubagentSessionUpsert {
            selector: tiny_selector,
            lifecycle_key,
            display_name: upsert.display_name,
            task_title: upsert.task_title,
            worker_thread_id: upsert.worker_thread_id,
            task_id: upsert.task_id,
            force_fresh,
        })
        .await
        .map_err(|error| error.to_string())?;
    crate::agent::orchestration::subagent_sessions::set_parent_epoch_for_context(
        ctx,
        workspace_dir,
        session.subagent_session_id.clone(),
        parent_epoch,
    )
    .map_err(|error| error.to_string())?;
    Ok(crate::agent::orchestration::subagent_sessions::host_projection(session, parent_epoch))
}
