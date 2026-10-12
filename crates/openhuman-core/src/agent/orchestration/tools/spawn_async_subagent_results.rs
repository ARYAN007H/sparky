fn refuse_missing_delivery_thread(
    parent_thread_id: Option<&str>,
    parent_agent_id: &str,
    requested_agent_id: &str,
) -> Option<ToolResult> {
    if parent_thread_id.is_none() {
        log::warn!(
            "[spawn_async_subagent] refusing fire-and-forget spawn with no delivery thread \
                 parent={} requested={} — directing caller to synchronous delegation (flow node / \
                 CLI / cron context, background result would be discarded)",
            parent_agent_id,
            requested_agent_id
        );
        return Some(ToolResult::error(
            "spawn_async_subagent: no parent chat thread available to deliver the result \
                 into (this looks like a flow node, CLI, or cron run rather than an interactive \
                 chat turn). Fire-and-forget delegation has nowhere to land its result here and \
                 the sub-agent's work would be silently discarded. Use synchronous delegation \
                 instead: a `delegate_*` tool with `blocking: true` runs the sub-agent inline \
                 and hands you its output in this turn. For parallel work, model it as \
                 parallel flow nodes rather than background sub-agents.",
        ));
    }
    None
}

fn accepted_spawn_result(
    task_id: &str,
    session_id: &str,
    agent_id: &str,
    worker_thread_id: Option<&str>,
    reused: bool,
    reuse_decision: &str,
    fleet: &FleetToolSet,
) -> ToolResult {
    let payload = async_subagent_ref_payload(
        task_id,
        session_id,
        agent_id,
        worker_thread_id,
        reused,
        reuse_decision,
        "running",
        fleet,
    );
    let payload_json = match serde_json::to_string(&payload) {
        Ok(serialized) => {
            log::debug!(
                "[spawn_async_subagent] serialized async reference payload bytes={}",
                serialized.len()
            );
            serialized
        }
        Err(error) => {
            log::debug!(
                "[spawn_async_subagent] failed to serialize async reference payload: {}",
                error
            );
            "{}".to_string()
        }
    };
    log::debug!("[spawn_async_subagent] formatting accepted response");
    ToolResult::success(format_async_subagent_accepted(
        payload["agent_id"].as_str().unwrap_or("subagent"),
        &payload_json,
        fleet,
    ))
}
