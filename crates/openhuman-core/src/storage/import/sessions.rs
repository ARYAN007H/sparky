//! Sessions, transcripts, turn states and the run ledger.
//!
//! Before the storage ports a workspace kept its conversation state in three
//! shapes: a SQLite `session_db/sessions.db` (session history, the run ledger,
//! entry trees), JSONL transcripts under `session_raw/`, and per-turn
//! snapshots as JSON under `memory/conversations/turn_states/`. TinyAgents owns
//! those formats and the importer that reads them
//! (`tinyagents_session::legacy`): read-only, id-preserving, only-if-absent.
//! This module is the host's side of it:
//!
//! - **Quiescence.** The importer refuses a database with a WAL or journal
//!   sidecar, so `sessions.db` is first copied to a consistent scratch
//!   snapshot ([`Snapshot`], `VACUUM INTO`); the real file is only ever
//!   opened read-only.
//! - **Whose sessions.** The workspace belongs to the operator. The run
//!   ledger and the turn states land in the operator's default agent
//!   ([`DEFAULT_AGENT`](crate::storage::sessions::DEFAULT_AGENT)), where
//!   the desktop's own work resolves them. A transcript stem names the agent
//!   definition that wrote it, but only as a digest, so each stem is matched
//!   against the digests of the known definitions
//!   ([`crate::agent_registry::host_definition::AgentDefinitionRegistry`]) and
//!   imported into that definition's scope, which is where the session host
//!   resumes it. A stem that matches none lands in the default agent.
//! - **Once.** A marker (like the other domains') is written when the whole
//!   import finished without skipping a record; a partial run is retried at
//!   the next boot and, being only-if-absent, never duplicates.
//!
//! Legacy files are never modified or deleted.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use tinyagents_session::DriverSessionStores;
use tinyagents_session::port::TurnStates;
use tinyagents_session::run_ledger::{ImportCounts, LegacyDatabaseReport, LegacySessionImport};
use tinyagents_session::transcript::{SessionRef, session_stem};
use tinyagents_session::turn_state::TurnState;

use super::tinyflows::{finish, marked};
use super::{Outcome, Report, Snapshot};
use crate::config::Config;
use crate::storage::sessions::DEFAULT_AGENT;
use crate::storage::{Scope, StorageBackend};

/// The marker name of the session import.
pub const SESSIONS_DOMAIN: &str = "sessions";

/// `session_db/sessions.db`, relative to the workspace.
const SESSIONS_DB: &str = "session_db/sessions.db";

/// Runs the session import for `config`'s workspace.
pub fn run_with(backend: &Arc<dyn StorageBackend>, config: &Config) -> Vec<Report> {
    vec![finish(SESSIONS_DOMAIN, import(backend, config))]
}

fn import(backend: &Arc<dyn StorageBackend>, config: &Config) -> Result<Outcome> {
    let workspace = config.workspace_dir.clone();
    let db = workspace.join(SESSIONS_DB);
    let raw = workspace.join("session_raw");
    let turns = workspace
        .join("memory")
        .join("conversations")
        .join("turn_states");
    marked(backend, &workspace, SESSIONS_DOMAIN, || {
        if !db.is_file() && !raw.is_dir() && !turns.is_dir() {
            return Ok(None);
        }
        let provider = DriverSessionStores::new(Arc::clone(backend), Scope::local())
            .map_err(|error| anyhow::anyhow!("start the session store bridge: {error}"))?
            .recover_on_open(false);
        let mut found = 0usize;
        let mut written = 0usize;
        let mut skipped = 0usize;

        if db.is_file() {
            let snapshot = Snapshot::of(&db, SESSIONS_DB)
                .context("[sessions] copy sessions.db for reading")?;
            let ledger = provider
                .driver_run_ledger(DEFAULT_AGENT)
                .map_err(|error| anyhow::anyhow!("open the run ledger: {error}"))?;
            let rows = super::session_db::SessionDatabase::read(snapshot.db())
                .context("[sessions] read the session database snapshot")?;
            let batch = LegacySessionImport {
                sessions: rows.sessions,
                messages: rows.messages,
                tool_calls: rows.tool_calls,
                agent_runs: rows.agent_runs,
                telemetry: rows.telemetry,
                events: rows.events,
                workflow_runs: rows.workflows,
                teams: rows.teams,
                members: rows.members,
                tasks: rows.tasks,
                tool_effects: rows.tool_effects,
            };
            let report = crate::storage::block_on(async move {
                ledger.import_legacy_batch(batch).await
            })?
            .context("[sessions] import sessions.db")?;
            let (database_found, database_written) = database_totals(&report);
            found += database_found;
            written += database_written;
        }

        if raw.is_dir() {
            let (f, w, s) = import_transcript_groups(&provider, &workspace)?;
            found += f;
            written += w;
            skipped += s;
        }

        if turns.is_dir() {
            let stores = provider
                .try_for_agent(DEFAULT_AGENT)
                .map_err(|error| anyhow::anyhow!("open the turn states: {error}"))?;
            let report = import_turn_states(&turns, stores.turn_states.as_ref())
                .context("[sessions] import turn states")?;
            found += report.read;
            written += report.imported;
            skipped += report.unreadable;
        }
        Ok(Some((found, written, skipped)))
    })
}

/// The native importer reports source rows per stored family. FTS rows are a
/// derived index and do not represent additional legacy records.
fn database_totals(report: &LegacyDatabaseReport) -> (usize, usize) {
    [
        report.sessions,
        report.messages,
        report.tool_calls,
        report.agent_runs,
        report.telemetry,
        report.events,
        report.workflow_runs,
        report.teams,
        report.members,
        report.tasks,
        report.tool_effects,
    ]
    .into_iter()
    .fold(
        (0, 0),
        |(found, written), ImportCounts { read, imported, .. }| (found + read, written + imported),
    )
}

#[derive(Default)]
struct TurnStateReport {
    read: usize,
    imported: usize,
    unreadable: usize,
}

/// Imports old per-turn snapshots without changing the workspace files. The
/// flat legacy form is retained alongside the newer per-thread directories.
fn import_turn_states(dir: &Path, destination: &dyn TurnStates) -> Result<TurnStateReport> {
    let mut report = TurnStateReport::default();
    for path in turn_state_files(dir)? {
        let state = match std::fs::read(&path)
            .with_context(|| format!("read turn-state snapshot {}", path.display()))
            .and_then(|bytes| serde_json::from_slice::<TurnState>(&bytes).map_err(Into::into))
        {
            Ok(state) => state,
            Err(error) => {
                tracing::warn!("[sessions] skipping unreadable turn state {}: {error}", path.display());
                report.unreadable += 1;
                continue;
            }
        };
        report.read += 1;
        if destination
            .get_turn(&state.thread_id, &state.request_id)
            .map_err(anyhow::Error::msg)?
            .is_none()
        {
            destination.put(&state).map_err(anyhow::Error::msg)?;
            report.imported += 1;
        }
    }
    Ok(report)
}

fn turn_state_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut entries = std::fs::read_dir(dir)
        .with_context(|| format!("list turn-state directory {}", dir.display()))?
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            let mut children = std::fs::read_dir(path)?.collect::<std::io::Result<Vec<_>>>()?;
            children.sort_by_key(std::fs::DirEntry::file_name);
            files.extend(children.into_iter().map(|child| child.path()).filter(|path| {
                path.extension().and_then(|extension| extension.to_str()) == Some("json")
            }));
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
            files.push(path);
        }
    }
    Ok(files)
}

/// The agent definitions a stem may belong to, by the component their ids
/// encode to in a stem.
fn known_agents() -> HashMap<String, String> {
    let mut ids: Vec<String> =
        crate::agent_registry::host_definition::AgentDefinitionRegistry::builtins_only()
            .list()
            .iter()
            .map(|def| def.id.clone())
            .collect();
    if let Some(current) =
        crate::agent_registry::host_definition::AgentDefinitionRegistry::current()
    {
        ids.extend(current.list().iter().map(|def| def.id.clone()));
    }
    ids.sort();
    ids.dedup();
    ids.into_iter()
        .filter_map(|id| agent_component(&id).map(|component| (component, id)))
        .collect()
}

/// The text a stem carries for agent `id`: `session_stem` of a probe session,
/// without its session key. A stem is `<key>.<agent>[.g<n>]`, each component
/// sanitized (a `.` inside one becomes `-`) and followed by a digest, so the
/// agent's component is what follows the first `.`.
fn agent_component(id: &str) -> Option<String> {
    let stem = session_stem(&SessionRef::scoped("probe", id));
    stem.split_once('.').map(|(_, agent)| agent.to_owned())
}

/// The agent whose component `stem` carries, if it is one of `known`.
fn agent_of(stem: &str, known: &HashMap<String, String>) -> Option<String> {
    // Sub-agent transcripts (`parent__child`) belong to their root's agent.
    let root = stem.split("__").next().unwrap_or(stem);
    let mut parts: Vec<&str> = root.split('.').collect();
    if parts.last().is_some_and(|last| {
        last.strip_prefix('g')
            .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
    }) {
        parts.pop();
    }
    if parts.len() < 2 {
        return None;
    }
    known.get(*parts.last()?).cloned()
}

/// Every `*.jsonl` under `raw` (flat and date-grouped), flat winning a
/// duplicate stem, as `(stem, path)`.
fn transcript_files(raw: &Path) -> Result<Vec<(String, PathBuf)>> {
    let mut flat = Vec::new();
    let mut dated = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(raw)
        .with_context(|| format!("read {}", raw.display()))?
        .collect::<std::io::Result<_>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            let mut inner: Vec<_> = std::fs::read_dir(&path)?.collect::<std::io::Result<_>>()?;
            inner.sort_by_key(std::fs::DirEntry::file_name);
            for entry in inner {
                push_jsonl(&mut dated, entry.path());
            }
        } else {
            push_jsonl(&mut flat, path);
        }
    }
    let seen: std::collections::HashSet<String> =
        flat.iter().map(|(stem, _)| stem.clone()).collect();
    flat.extend(dated.into_iter().filter(|(stem, _)| !seen.contains(stem)));
    Ok(flat)
}

fn push_jsonl(out: &mut Vec<(String, PathBuf)>, path: PathBuf) {
    if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
        return;
    }
    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
        out.push((stem.to_owned(), path));
    }
}

/// Imports the transcripts, each into the scope of the agent its stem names.
/// Returns `(found, written, skipped)`.
fn import_transcript_groups(
    provider: &DriverSessionStores,
    workspace: &Path,
) -> Result<(usize, usize, usize)> {
    let raw = workspace.join("session_raw");
    let known = known_agents();
    let mut groups: HashMap<String, Vec<(String, PathBuf)>> = HashMap::new();
    for (stem, path) in transcript_files(&raw)? {
        let agent = agent_of(&stem, &known).unwrap_or_else(|| DEFAULT_AGENT.to_owned());
        groups.entry(agent).or_default().push((stem, path));
    }
    let (mut found, mut written, mut skipped) = (0, 0, 0);
    for (agent, files) in groups {
        // The importer reads a directory; stage this agent's files in one
        // (hard links where possible, never touching the originals).
        let stage = tempfile::Builder::new()
            .prefix("openhuman-sessions-import-")
            .tempdir()
            .context("[sessions] create the staging directory")?;
        let staged_raw = stage.path().join("session_raw");
        std::fs::create_dir_all(&staged_raw)?;
        for (stem, path) in &files {
            let target = staged_raw.join(format!("{stem}.jsonl"));
            if std::fs::hard_link(path, &target).is_err() {
                std::fs::copy(path, &target)
                    .with_context(|| format!("[sessions] stage a transcript for {agent}"))?;
            }
        }
        let stores = provider
            .try_for_agent(&agent)
            .map_err(|error| anyhow::anyhow!("open the transcripts of {agent}: {error}"))?;
        let report = import_transcripts(stage.path(), stores.transcripts.as_ref())
            .with_context(|| format!("[sessions] import the transcripts of {agent}"))?;
        found += report.read;
        written += report.imported;
        skipped += report.unreadable;
    }
    Ok((found, written, skipped))
}

#[cfg(test)]
#[path = "sessions_tests.rs"]
mod tests;
