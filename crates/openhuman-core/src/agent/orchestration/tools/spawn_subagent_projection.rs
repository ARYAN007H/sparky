// Async delivery is thread-addressed: the finished result is inserted back into the
// parent chat thread as a follow-up turn (`background_delivery`). Outside a chat turn
// (flow `agent` nodes, CLI and cron runs intentionally have no parent thread to deliver
// into, so `background_delivery::deliver_batch` logs "dropping headless batch" and the
// (possibly real, completed) work is silently discarded — the caller sees "Accepted" and
// never learns the result never arrived. Fail loudly instead: the caller has a
// synchronous alternative (`spawn_subagent` with `blocking: true`, or a `delegate_*`
// tool). Both of those self-heal to blocking dispatch in this situation rather than
// reaching this guard — see the `has_delivery_thread` checks in `spawn_subagent_surface.rs` and
// `subagent_dispatch_host.rs::dispatch_subagent`. Only a *direct* `spawn_subagent` call lands
// here.
// The follow-up vocabulary offered back to the parent is limited to
// the fleet tools visible on *this turn* (see `fleet_tools`), not
// just the parent definition's static scope: a hide or named
// restriction can narrow the turn's real tool surface below what the
// definition alone would suggest, and offering a control the parent
// cannot currently call invites a denied tool call.
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
                "[spawn_subagent] serialized async reference payload bytes={}",
                serialized.len()
            );
            serialized
        }
        Err(error) => {
            log::debug!(
                "[spawn_subagent] failed to serialize async reference payload: {}",
                error
            );
            "{}".to_string()
        }
    };
    log::debug!("[spawn_subagent] formatting accepted response");
    ToolResult::success(format_async_subagent_accepted(
        payload["agent_id"].as_str().unwrap_or("subagent"),
        &payload_json,
        fleet,
    ))
}

fn refuse_empty_spawn_args(agent_id: &str, prompt: &str) -> Option<ToolResult> {
    if agent_id.is_empty() {
        return Some(ToolResult::error(
            "spawn_subagent: `agent_id` is required",
        ));
    }
    if prompt.is_empty() {
        return Some(ToolResult::error(
            "spawn_subagent: `prompt` is required",
        ));
    }
    None
}
