let dispatch = run_context.dispatch_ctx();
let AsyncSpawnArgs {
    agent_id,
    mut prompt,
    context,
    model_override,
    task_title,
    task_key,
    force_fresh,
} = decode_async_spawn_args(&args);

if let Some(refusal) = refuse_empty_spawn_args(&agent_id, &prompt) {
    return Ok(refusal);
}
let Some(detached_parent) = detached_parent else {
    return Ok(ToolResult::error(
        "spawn_subagent requires a live harness run context.",
    ));
};
