use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use crate::agent::subagent_host::SubagentRunStatus;
use tinyagents_session::transcript::TranscriptMessage;

use super::types::{
    DurableSubagentSession, DurableSubagentStatus, ReuseDecision, SubagentSessionSelector,
    SubagentSessionStore, SubagentSessionUpsert,
};

/// Requests cancellation of driver-backed children owned by a thread being
/// removed. The TinyAgents registry owns cancellation and settlement; this
/// adapter only selects durable rows and resolves their authenticated keys.
pub fn cancel_driver_jobs_for_thread(
    store: &SubagentSessionStore,
    workspace_dir: &Path,
    jobs: &tinyagents_orchestration::subagent::SubAgentJobRegistry,
    thread_id: &str,
) -> Vec<String> {
    let sessions = match store.load() {
        Ok(sessions) => sessions,
        Err(error) => {
            log::warn!(
                "[subagent_sessions] failed to load sessions for thread cancellation: {error}"
            );
            return Vec::new();
        }
    };
    let mut cancelled = Vec::new();
    for session in sessions.into_iter().filter(|session| {
        session.parent_thread_id.as_deref() == Some(thread_id)
            && matches!(
                session.status,
                DurableSubagentStatus::Running | DurableSubagentStatus::AwaitingUser
            )
    }) {
        let Some(task_id) = session.current_task_id.as_deref() else {
            continue;
        };
        let key = session
            .lifecycle_key
            .filter(|key| key.task_id == task_id)
            .or_else(|| {
                resolve_lifecycle_key(
                    store,
                    &session.subagent_session_id,
                    &session.parent_session,
                    Some(thread_id),
                    &workspace_dir.join(".openhuman/subagent_checkpoints"),
                )
                .ok()
                .filter(|key| key.task_id == task_id)
            });
        let Some(key) = key else {
            continue;
        };
        let owner = crate::agent::tinyagents::tool_registration::subagent_owner_key(
            &session.parent_session,
            Some(thread_id),
        );
        if jobs.cancel_key_owned(&key, owner).is_ok() {
            cancelled.push(task_id.to_owned());
        }
    }
    cancelled
}

/// Cancels the current driver-backed lifecycle for one authenticated durable
/// session. Legacy rows recover a key only through their validated checkpoint.
pub fn cancel_driver_job_for_session(
    store: &SubagentSessionStore,
    checkpoint_dir: &Path,
    jobs: &tinyagents_orchestration::subagent::SubAgentJobRegistry,
    session_id: &str,
    parent_session: &str,
    parent_thread_id: Option<&str>,
) -> bool {
    let session = match list_for_parent(store, parent_session, parent_thread_id) {
        Ok(sessions) => sessions
            .into_iter()
            .find(|session| session.subagent_session_id == session_id),
        Err(error) => {
            log::warn!("[subagent_sessions] session cancellation lookup failed: {error}");
            None
        }
    };
    let Some(session) = session else {
        return false;
    };
    let Some(task_id) = session.current_task_id.as_deref() else {
        return false;
    };
    let key = session
        .lifecycle_key
        .clone()
        .filter(|key| key.task_id == task_id && key.thread_id.as_deref() == parent_thread_id)
        .or_else(|| {
            resolve_lifecycle_key(
                store,
                session_id,
                parent_session,
                parent_thread_id,
                checkpoint_dir,
            )
            .ok()
            .filter(|key| key.task_id == task_id)
        });
    let Some(key) = key else {
        return false;
    };
    let owner = crate::agent::tinyagents::tool_registration::subagent_owner_key(
        parent_session,
        parent_thread_id,
    );
    jobs.cancel_key_owned(&key, owner).is_ok()
}

/// Requests cancellation for the exact durable row observed by the caller.
/// Keeping this separate from a fresh session-id lookup prevents concurrent
/// reuse from redirecting cancellation to a replacement child.
pub fn cancel_driver_job_for_observed_session(
    store: &SubagentSessionStore,
    checkpoint_dir: &Path,
    jobs: &tinyagents_orchestration::subagent::SubAgentJobRegistry,
    session: &DurableSubagentSession,
    parent_session: &str,
    parent_thread_id: Option<&str>,
) -> bool {
    if session.parent_session != parent_session
        || session.parent_thread_id.as_deref() != parent_thread_id
    {
        return false;
    }
    let Some(task_id) = session.current_task_id.as_deref() else {
        return false;
    };
    let key = session
        .lifecycle_key
        .clone()
        .filter(|key| key.task_id == task_id && key.thread_id.as_deref() == parent_thread_id)
        .or_else(|| {
            resolve_lifecycle_key(
                store,
                &session.subagent_session_id,
                parent_session,
                parent_thread_id,
                checkpoint_dir,
            )
            .ok()
            .filter(|key| key.task_id == task_id)
        });
    let Some(key) = key else {
        return false;
    };
    let owner = crate::agent::tinyagents::tool_registration::subagent_owner_key(
        parent_session,
        parent_thread_id,
    );
    jobs.cancel_key_owned(&key, owner).is_ok()
}

/// A driver-backed child selected and cancelled by its public task id.
///
/// The selector is scoped to the caller's durable session store and refuses
/// ambiguous task IDs. The actual cancellation remains keyed by the full
/// authenticated TinyAgents lifecycle key.
#[derive(Debug, Clone)]
pub struct DriverTaskCancellation {
    pub agent_id: String,
    pub parent_session: String,
    pub subagent_session_id: String,
    pub workspace_dir: std::path::PathBuf,
    pub parent_thread_id: Option<String>,
}

pub fn cancel_driver_job_by_task(
    store: &SubagentSessionStore,
    checkpoint_dir: &Path,
    jobs: &tinyagents_orchestration::subagent::SubAgentJobRegistry,
    task_id: &str,
) -> Option<DriverTaskCancellation> {
    let mut matches = store.load().ok()?.into_iter().filter(|session| {
        session.current_task_id.as_deref() == Some(task_id)
            && matches!(
                session.status,
                DurableSubagentStatus::Running | DurableSubagentStatus::AwaitingUser
            )
    });
    let session = matches.next()?;
    if matches.next().is_some()
        || !cancel_driver_job_for_observed_session(
            store,
            checkpoint_dir,
            jobs,
            &session,
            &session.parent_session,
            session.parent_thread_id.as_deref(),
        )
    {
        return None;
    }
    Some(DriverTaskCancellation {
        agent_id: session.agent_id,
        parent_session: session.parent_session,
        subagent_session_id: session.subagent_session_id,
        workspace_dir: store.workspace_dir.clone(),
        parent_thread_id: session.parent_thread_id,
    })
}

/// Resolve a saved product session to its exact TinyAgents lifecycle key.
///
/// The product session lookup is scoped by the authenticated parent session,
/// optional parent thread, and the workspace-specific store. Rows written
/// before `lifecycle_key` was added recover it only from the matching host
/// checkpoint; a public task id alone is never treated as a lifecycle key.
pub fn resolve_lifecycle_key(
    store: &SubagentSessionStore,
    session_or_task_id: &str,
    parent_session: &str,
    parent_thread_id: Option<&str>,
    checkpoint_dir: &Path,
) -> Result<tinyagents_orchestration::subagent::SubagentTaskKey, String> {
    let session = list_for_parent(store, parent_session, parent_thread_id)?
        .into_iter()
        .find(|session| {
            session.subagent_session_id == session_or_task_id
                || session.current_task_id.as_deref() == Some(session_or_task_id)
        })
        .ok_or_else(|| "durable subagent session not found".to_string())?;
    let task_id = session
        .current_task_id
        .as_deref()
        .ok_or_else(|| "durable subagent session has no current task".to_string())?;
    let key = if let Some(key) = session.lifecycle_key {
        key
    } else {
        crate::agent::subagent_host::load_subagent_checkpoint(checkpoint_dir, task_id)
            .map_err(|error| format!("legacy subagent checkpoint unavailable: {error}"))?
            .task_key
            .ok_or_else(|| "legacy subagent checkpoint has no lifecycle key".to_string())?
    };
    if key.task_id != task_id || key.thread_id.as_deref() != session.parent_thread_id.as_deref() {
        return Err("durable subagent lifecycle key does not match its owned session".into());
    }
    Ok(key)
}

pub fn normalize_task_key(input: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in input.trim().to_lowercase().chars() {
        if ch.is_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        if input.trim().is_empty() {
            "untitled-task".to_string()
        } else {
            format!("task-{:016x}", task_key_hash(input))
        }
    } else if out.len() > 96 {
        let hash = format!("{:016x}", task_key_hash(input));
        let mut prefix = String::new();
        for ch in out.chars() {
            if prefix.len() + ch.len_utf8() + hash.len() + 1 > 96 {
                break;
            }
            prefix.push(ch);
        }
        while prefix.ends_with('-') {
            prefix.pop();
        }
        format!("{prefix}-{hash}")
    } else {
        out
    }
}

fn task_key_hash(input: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

pub fn task_title_from_prompt(prompt: &str) -> String {
    let collapsed = prompt.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= 80 {
        collapsed
    } else {
        let mut title: String = collapsed.chars().take(77).collect();
        title.push_str("...");
        title
    }
}

pub fn action_root_key(path: Option<&Path>) -> Option<String> {
    path.map(|p| p.to_string_lossy().to_string())
}

pub fn find_reusable(
    store: &SubagentSessionStore,
    selector: &SubagentSessionSelector,
) -> Result<Option<DurableSubagentSession>, String> {
    let mut matches: Vec<DurableSubagentSession> = store
        .load()?
        .into_iter()
        .filter(|session| session.matches_selector(selector))
        .collect();
    matches.sort_by(|a, b| b.last_used_at.cmp(&a.last_used_at));
    Ok(matches.into_iter().next())
}

pub fn upsert_running(
    store: &SubagentSessionStore,
    upsert: SubagentSessionUpsert,
    reuse: Option<&DurableSubagentSession>,
) -> Result<DurableSubagentSession, String> {
    let _guard = store_write_lock()
        .lock()
        .map_err(|_| "subagent session store lock poisoned".to_string())?;
    let mut sessions = store.load()?;
    let now = chrono::Utc::now().to_rfc3339();
    let session_id = reuse
        .map(|session| session.subagent_session_id.clone())
        .unwrap_or_else(|| format!("subsess-{}", uuid::Uuid::new_v4()));

    let mut session = if let Some(existing) = sessions
        .iter()
        .find(|session| session.subagent_session_id == session_id)
        .cloned()
    {
        if let Some(reuse) = reuse {
            if existing.status == DurableSubagentStatus::Closed
                || existing.current_task_id != reuse.current_task_id
                || existing.lifecycle_key != reuse.lifecycle_key
            {
                return Err("stale reusable sub-agent session changed during upsert".into());
            }
        }
        existing
    } else {
        DurableSubagentSession {
            subagent_session_id: session_id.clone(),
            parent_session: upsert.selector.parent_session.clone(),
            parent_thread_id: upsert.selector.parent_thread_id.clone(),
            worker_thread_id: upsert.worker_thread_id.clone(),
            agent_id: upsert.selector.agent_id.clone(),
            display_name: upsert.display_name.clone(),
            model: upsert.selector.model.clone(),
            sandbox_mode: upsert.selector.sandbox_mode.clone(),
            action_root: upsert.selector.action_root.clone(),
            task_key: upsert.selector.task_key.clone(),
            task_title: upsert.task_title.clone(),
            current_task_id: None,
            lifecycle_key: None,
            status: DurableSubagentStatus::Running,
            reusable: true,
            latest_history: None,
            latest_error: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            last_used_at: now.clone(),
        }
    };

    session.parent_session = upsert.selector.parent_session;
    session.parent_thread_id = upsert.selector.parent_thread_id;
    session.worker_thread_id = upsert.worker_thread_id.or(session.worker_thread_id);
    session.display_name = upsert.display_name.or(session.display_name);
    session.current_task_id = Some(upsert.task_id);
    session.lifecycle_key = upsert.lifecycle_key.or(session.lifecycle_key);
    session.status = DurableSubagentStatus::Running;
    session.reusable = true;
    session.latest_error = None;
    session.updated_at = now.clone();
    session.last_used_at = now;

    sessions.retain(|existing| existing.subagent_session_id != session_id);
    sessions.push(session.clone());
    store.save(&sessions)?;
    Ok(session)
}

pub fn mark_finished(
    store: &SubagentSessionStore,
    subagent_session_id: &str,
    task_id: &str,
    run_status: &SubagentRunStatus,
    history: Vec<TranscriptMessage>,
) -> Result<(), String> {
    let updated = update_session_for_task(store, subagent_session_id, task_id, |session, now| {
        session.status = DurableSubagentStatus::from_run_status(run_status);
        session.latest_history = Some(history);
        session.latest_error = None;
        session.updated_at = now.clone();
        session.last_used_at = now;
    })?;
    if updated {
        Ok(())
    } else {
        Err(format!(
            "sub-agent session not found: {subagent_session_id}"
        ))
    }
}

pub fn mark_failed(
    store: &SubagentSessionStore,
    subagent_session_id: &str,
    task_id: &str,
    error: String,
) -> Result<(), String> {
    let updated = update_session_for_task(store, subagent_session_id, task_id, |session, now| {
        session.status = DurableSubagentStatus::Failed;
        session.latest_error = Some(error);
        session.updated_at = now.clone();
        session.last_used_at = now;
    })?;
    if updated {
        Ok(())
    } else {
        Err(format!(
            "sub-agent session not found: {subagent_session_id}"
        ))
    }
}

pub fn close(store: &SubagentSessionStore, subagent_session_id: &str) -> Result<bool, String> {
    update_session(store, subagent_session_id, |session, now| {
        session.status = DurableSubagentStatus::Closed;
        session.reusable = false;
        session.updated_at = now.clone();
        session.last_used_at = now;
    })
}

/// Closes a session only if it still contains the lifecycle snapshot that the
/// caller observed and cancelled. `false` means a concurrent reuse or close
/// won the race and this stale request did not alter the newer row.
pub fn close_if_current(
    store: &SubagentSessionStore,
    observed: &DurableSubagentSession,
) -> Result<bool, String> {
    let _guard = store_write_lock()
        .lock()
        .map_err(|_| "subagent session store lock poisoned".to_string())?;
    let mut sessions = store.load()?;
    let now = chrono::Utc::now().to_rfc3339();
    let Some(session) = sessions
        .iter_mut()
        .find(|session| session.subagent_session_id == observed.subagent_session_id)
    else {
        return Ok(false);
    };
    if session.parent_session != observed.parent_session
        || session.parent_thread_id != observed.parent_thread_id
        || session.current_task_id != observed.current_task_id
        || session.lifecycle_key != observed.lifecycle_key
    {
        return Ok(false);
    }
    if session.status == DurableSubagentStatus::Closed {
        return Ok(true);
    }
    session.status = DurableSubagentStatus::Closed;
    session.reusable = false;
    session.updated_at = now.clone();
    session.last_used_at = now;
    store.save(&sessions)?;
    Ok(true)
}

pub fn list_for_parent(
    store: &SubagentSessionStore,
    parent_session: &str,
    parent_thread_id: Option<&str>,
) -> Result<Vec<DurableSubagentSession>, String> {
    let mut sessions: Vec<DurableSubagentSession> = store
        .load()?
        .into_iter()
        .filter(|session| {
            session.parent_session == parent_session
                && parent_thread_id
                    .map(|thread_id| session.parent_thread_id.as_deref() == Some(thread_id))
                    .unwrap_or(true)
        })
        .collect();
    sessions.sort_by(|a, b| b.last_used_at.cmp(&a.last_used_at));
    Ok(sessions)
}

pub fn reuse_decision(reuse: Option<&DurableSubagentSession>, force_fresh: bool) -> ReuseDecision {
    if force_fresh {
        return ReuseDecision::ForcedFresh;
    }
    match reuse.map(|session| session.status) {
        Some(DurableSubagentStatus::Running) => ReuseDecision::ReusedRunning,
        Some(_) => ReuseDecision::ReusedIdle,
        None => ReuseDecision::SpawnedNew,
    }
}

fn update_session<F>(
    store: &SubagentSessionStore,
    subagent_session_id: &str,
    update: F,
) -> Result<bool, String>
where
    F: FnOnce(&mut DurableSubagentSession, String),
{
    let _guard = store_write_lock()
        .lock()
        .map_err(|_| "subagent session store lock poisoned".to_string())?;
    let mut sessions = store.load()?;
    let now = chrono::Utc::now().to_rfc3339();
    if let Some(session) = sessions
        .iter_mut()
        .find(|session| session.subagent_session_id == subagent_session_id)
    {
        update(session, now);
        store.save(&sessions)?;
        return Ok(true);
    }
    Ok(false)
}

/// Apply a terminal write only while it still belongs to the current task.
/// The check and write share the store lock so a late completion cannot
/// overwrite a reused session or revive one that has already been closed.
fn update_session_for_task<F>(
    store: &SubagentSessionStore,
    subagent_session_id: &str,
    task_id: &str,
    update: F,
) -> Result<bool, String>
where
    F: FnOnce(&mut DurableSubagentSession, String),
{
    let _guard = store_write_lock()
        .lock()
        .map_err(|_| "subagent session store lock poisoned".to_string())?;
    let mut sessions = store.load()?;
    let now = chrono::Utc::now().to_rfc3339();
    if let Some(session) = sessions
        .iter_mut()
        .find(|session| session.subagent_session_id == subagent_session_id)
    {
        if session.status == DurableSubagentStatus::Closed
            || session.current_task_id.as_deref() != Some(task_id)
        {
            return Ok(false);
        }
        update(session, now);
        store.save(&sessions)?;
        return Ok(true);
    }
    Ok(false)
}

fn store_write_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[cfg(test)]
#[path = "ops_tests.rs"]
mod tests;
