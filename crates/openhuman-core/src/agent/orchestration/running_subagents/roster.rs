//! Read-only rosters over the sub-agent registry: the durable+live merge used to render the
//! `[active_subagents]` context block.

use tinyagents_orchestration::subagent::{snapshot_for_owner, SubagentSnapshot};

use super::registry::{admit_workspace, owner_key, registry};

/// Snapshot the sub-agents registered under `parent_session`, with each status
/// read live from its watch channel (ordered by `agent_id` then `task_id`).
pub(crate) fn snapshot_for_parent(
    ctx: &crate::core::runtime::DispatchCtx,
    parent_session: &str,
) -> Vec<SubagentSnapshot> {
    let Ok(owner) = owner_key(ctx, parent_session) else {
        return Vec::new();
    };
    snapshot_for_owner(registry(), &owner).unwrap_or_else(|err| {
        log::warn!(
            "[running_subagents] roster unavailable parent_session={parent_session} err={err:?}"
        );
        Vec::new()
    })
}

/// The single supported follow-up path for a durable sub-agent task.
fn roster_guidance() -> &'static str {
    "reuse its task_key with spawn_subagent to continue or answer a clarification; completed results are delivered automatically."
}

/// Most-recent durable sessions surfaced in the roster when they are not in
/// the live registry (cold boot / later turn). Bounds prompt growth on
/// threads with a long delegation history.
const DURABLE_ROSTER_CAP: usize = 12;

/// Build the ambient `[active_subagents]` block prepended to a parent's turn
/// context. Returns `None` when the parent owns no sub-agents at all, so the
/// block only appears when it is actionable — turns for agents that never
/// spawn are untouched. Mirrors the thread-goal `[active_goal]` block: it
/// rides the per-turn context (not the cached system-prompt prefix), so it
/// reflects live status every turn.
///
/// The roster merges two sources:
/// 1. the in-memory registry (live async workers spawned this process), and
/// 2. the durable per-workspace `subagent_sessions` store — workers from
///    EARLIER turns / process lifetimes. Without this second source a
///    cold-booted parent had no idea its previous sub-agents existed and
///    would re-delegate from scratch instead of resuming by
///    `subagent_session_id` (the "fresh context from day 0" bug).
///
pub(crate) fn active_subagents_context_block(
    ctx: &crate::core::runtime::DispatchCtx,
    parent_session: &str,
    workspace_dir: &std::path::Path,
) -> Option<String> {
    admit_workspace(ctx, workspace_dir).ok()?;
    let workers = snapshot_for_parent(ctx, parent_session);

    // Durable sessions not already represented by a live registry entry.
    let live_session_ids: std::collections::HashSet<String> = workers
        .iter()
        .filter_map(|w| w.subagent_session_id.clone())
        .collect();
    let store = crate::agent::orchestration::subagent_sessions::SubagentSessionStore {
        workspace_dir: workspace_dir.to_path_buf(),
    };
    let durable: Vec<_> = match crate::agent::orchestration::subagent_sessions::list_for_parent(
        &store,
        parent_session,
        None,
    ) {
        Ok(sessions) => sessions
            .into_iter()
            .filter(|s| {
                use crate::agent::orchestration::subagent_sessions::DurableSubagentStatus;
                s.status != DurableSubagentStatus::Closed
                    && !live_session_ids.contains(&s.subagent_session_id)
            })
            .take(DURABLE_ROSTER_CAP)
            .collect(),
        Err(err) => {
            log::warn!(
                "[running_subagents] durable roster load failed parent_session={parent_session} error={err}"
            );
            Vec::new()
        }
    };

    if workers.is_empty() && durable.is_empty() {
        return None;
    }
    let mut block = format!(
        "[active_subagents]\n\
         You have {} sub-agent worker(s) for this conversation (live and/or from earlier \
         turns). This is your authoritative roster — trust it over memory. Track each by \
         subagent_session_id. {} Never fabricate a result for a worker still running \
         or one that has failed.\n",
        workers.len() + durable.len(),
        roster_guidance()
    );
    for w in &workers {
        let session = w.subagent_session_id.as_deref().unwrap_or("(none)");
        block.push_str(&format!(
            "- {} · session={} · task={} · status={}\n",
            w.agent_id, session, w.task_id, w.status
        ));
    }
    for s in &durable {
        use crate::agent::orchestration::subagent_sessions::DurableSubagentStatus;
        let status = match s.status {
            DurableSubagentStatus::Running => "running",
            DurableSubagentStatus::Idle => "idle",
            DurableSubagentStatus::AwaitingUser => "awaiting_user",
            DurableSubagentStatus::Failed => "failed",
            DurableSubagentStatus::Closed => "closed",
        };
        let task = s.current_task_id.as_deref().unwrap_or("(none)");
        block.push_str(&format!(
            "- {} · session={} · task={} · status={} · about: {}\n",
            s.agent_id, s.subagent_session_id, task, status, s.task_title
        ));
    }
    block.push_str("[/active_subagents]\n\n");
    Some(block)
}
