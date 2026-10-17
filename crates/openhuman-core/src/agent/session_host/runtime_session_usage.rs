//! Committed-turn goal accounting and billing projection.

fn rollup_usage(
    sidecar: &crate::agent::tinyagents::host::run_context::SessionTurnSidecar,
) -> tinyagents_harness::cost::UsageRollup {
    // Each host entry already represents a complete child subtree. Treat it
    // as one leaf under its actual execution id; the owner rollup handles
    // replay deduplication and cost certainty without counting descendants a
    // second time.
    tinyagents_harness::cost::roll_up_subagent_usage(
        sidecar.input_tokens,
        sidecar.output_tokens,
        sidecar.cached_input_tokens,
        sidecar.cost,
        sidecar.subagents.iter().map(|entry| {
            let usage = &entry.usage;
            tinyagents_harness::cost::SubagentUsage {
                run_id: entry.run_id.clone(),
                task_id: entry.task_id.clone(),
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
                cached_input_tokens: usage.cached_input_tokens,
                cost: usage.cost(),
                children: Vec::new(),
            }
        }),
    )
}

/// Account only a receipt-backed turn, using the same direct-plus-completed
/// child total the codec attached to the atomic transcript append.
pub(in crate::agent::session_host) async fn account_committed_turn_against_goal(
    workspace_dir: &std::path::Path,
    thread_id: Option<&str>,
    sidecar: &crate::agent::tinyagents::host::run_context::SessionTurnSidecar,
) {
    let usage = rollup_usage(sidecar);
    crate::agent::goals::runtime::account_turn_against_goal(
        workspace_dir,
        thread_id,
        usage.input_tokens,
        usage.output_tokens,
        sidecar
            .duration
            .map(|duration| duration.as_secs())
            .unwrap_or_default(),
    )
    .await;
}

/// UI billing projection for a committed turn. It keeps child records for a
/// detailed display while reporting the same all-in totals the codec persists.
pub(in crate::agent::session_host) fn holistic_last_turn_usage(
    sidecar: &crate::agent::tinyagents::host::run_context::SessionTurnSidecar,
) -> crate::agent::tinyagents::host::LastTurnUsage {
    let rollup = rollup_usage(sidecar);
    let mut details = std::collections::HashMap::new();
    for entry in &sidecar.subagents {
        details.entry(entry.run_id.as_str()).or_insert(entry);
    }
    let subagents = rollup
        .subagents
        .iter()
        .filter_map(|usage| details.get(usage.run_id.as_str()).map(|entry| (*entry).clone()))
        .collect();
    crate::agent::tinyagents::host::LastTurnUsage {
        input_tokens: rollup.input_tokens,
        output_tokens: rollup.output_tokens,
        cached_input_tokens: rollup.cached_input_tokens,
        cost_usd: rollup.cost.usd(),
        cost_source: rollup.cost.source,
        context_window: sidecar.context_window,
        context_tokens: sidecar
            .last_call_input_tokens
            .saturating_add(sidecar.last_call_output_tokens),
        subagents,
        // The session does not count these; a library turn that asked for a
        // final-response report fills them in (`response_shape`).
        reasoning_tokens: 0,
    }
}
