//! Sessions, transcripts, turn states and the run ledger.
//!
//! Before the storage ports a workspace kept its conversation state in three
//! shapes: a SQLite `session_db/sessions.db` (session history, the run ledger,
//! entry trees), JSONL transcripts under `session_raw/`, and per-turn
//! snapshots as JSON under `memory/conversations/turn_states/`. OpenHuman owns
//! the historical file readers and imports their decoded public records into
//! TinyAgents' current storage ports. The readers are read-only, id-preserving,
//! and only-if-absent. This module handles:
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
use tinyagents_session::run_ledger::RunLedgerStore;
use tinyagents_session::run_ledger::{ImportCounts, LegacyDatabaseReport, LegacySessionImport};
use tinyagents_session::transcript::{
    DisplayMessage, DisplayRecord, SessionRef, TranscriptHistory, TranscriptMessage,
    TranscriptMeta, TranscriptPartial, TranscriptTurn, decode_transcript_jsonl, session_stem,
};
use tinyagents_session::turn_state::TurnState;

use super::tinyflows::{finish, marked};
use super::{Outcome, Report, Snapshot};
use crate::config::Config;
use crate::storage::StorageBackend;
use crate::storage::sessions::DEFAULT_AGENT;

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
    marked(backend, &workspace, SESSIONS_DOMAIN, |condition| {
        if !db.is_file() && !raw.is_dir() && !turns.is_dir() {
            return Ok(None);
        }
        let claimed_backend = super::super::import_state::backend_with_condition(
            Arc::clone(backend),
            condition.clone(),
        );
        let factory: tinyagents_session::port::AgentStorageFactory = Arc::new(move |agent_id| {
            let scope = crate::storage::scope_for_agent(agent_id);
            let storage = claimed_backend.for_scope(&scope)?;
            Ok(tinyagents_storage_kit::ScopedStorage::new(
                storage,
                (*scope).clone(),
            ))
        });
        let provider = DriverSessionStores::new(factory)
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
            let mut rows = super::session_db::SessionDatabase::read(snapshot.db())
                .context("[sessions] read the session database snapshot")?;
            sanitize_legacy_system_messages(&mut rows.messages);
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
            let report = crate::storage::block_on_anyhow(async move {
                ledger
                    .import_legacy_batch(batch)
                    .await
                    .map_err(|error| anyhow::anyhow!("{error}"))
            })
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

fn sanitize_legacy_system_messages(messages: &mut [tinyagents_session::SessionMessage]) {
    for message in messages {
        if message.role == "system" {
            if let Some(cleaned) =
                crate::config::migrations::phase_out_profile_md::strip_profile_md_block(
                    &message.content,
                )
            {
                message.content = cleaned;
            }
        }
    }
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
                tracing::warn!(
                    "[sessions] skipping unreadable turn state {}: {error}",
                    path.display()
                );
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
    if std::fs::symlink_metadata(dir)?.file_type().is_symlink() {
        anyhow::bail!("turn-state root {} is a symbolic link", dir.display());
    }
    let mut files = Vec::new();
    let mut entries = std::fs::read_dir(dir)
        .with_context(|| format!("list turn-state directory {}", dir.display()))?
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            let mut children = std::fs::read_dir(path)?.collect::<std::io::Result<Vec<_>>>()?;
            children.sort_by_key(std::fs::DirEntry::file_name);
            for child in children {
                let path = child.path();
                if child.file_type()?.is_file()
                    && path.extension().and_then(|extension| extension.to_str()) == Some("json")
                {
                    files.push(path);
                }
            }
        } else if file_type.is_file()
            && path.extension().and_then(|extension| extension.to_str()) == Some("json")
        {
            files.push(path);
        }
    }
    Ok(files)
}

fn decode_transcript(
    path: &Path,
) -> Result<(
    TranscriptMeta,
    Option<serde_json::Value>,
    Vec<DisplayRecord>,
)> {
    let bytes =
        std::fs::read(path).with_context(|| format!("read transcript {}", path.display()))?;
    let (import_bytes, _) = tinyagents_session::transcript::rewrite_first_system_message_jsonl(
        &bytes,
        |content| {
            let Some(cleaned) = crate::config::migrations::phase_out_profile_md::strip_profile_md_block(content) else {
                return false;
            };
            *content = cleaned;
            true
        },
    )
    .with_context(|| format!("sanitize imported transcript {}", path.display()))?;
    let decoded = decode_transcript_jsonl(&import_bytes)
        .with_context(|| format!("decode transcript {}", path.display()))?;
    Ok((
        decoded.session.meta,
        decoded.session.tools,
        decoded.display.records,
    ))
}

fn replay_transcript(
    history: &dyn TranscriptHistory,
    meta: &TranscriptMeta,
    tools: Option<&serde_json::Value>,
    records: Vec<DisplayRecord>,
) -> Result<usize> {
    let mut logical = Vec::<TranscriptMessage>::new();
    let mut group = Vec::<DisplayMessage>::new();
    let mut replayed = 0usize;
    let flush = |logical: &mut Vec<TranscriptMessage>,
                 group: &mut Vec<DisplayMessage>,
                 replayed: &mut usize|
     -> Result<()> {
        if group.is_empty() {
            return Ok(());
        }
        let mut next = logical.clone();
        next.extend(group.iter().map(|message| message.message.clone()));
        let usage = group
            .iter()
            .rev()
            .find_map(|message| message.turn_usage.as_ref());
        let request_id = group
            .iter()
            .rev()
            .find_map(|message| message.request_id.as_deref());
        history.append_turn(TranscriptTurn {
            prev: logical,
            next: &next,
            meta,
            turn_usage: usage,
            request_id,
            tools: None,
        })?;
        *replayed += group.len();
        *logical = next;
        group.clear();
        Ok(())
    };
    for record in records {
        match record {
            DisplayRecord::Message(message) if message.interrupted => {
                flush(&mut logical, &mut group, &mut replayed)?;
                let partial = TranscriptPartial {
                    content: message.message.content.clone(),
                    reasoning_content: message.reasoning_content.clone(),
                    iteration: message.iteration,
                };
                history.append_turn_with_partial(
                    TranscriptTurn {
                        prev: &logical,
                        next: &logical,
                        meta,
                        turn_usage: None,
                        request_id: message.request_id.as_deref(),
                        tools: None,
                    },
                    Some(&partial),
                )?;
            }
            DisplayRecord::Message(message) => {
                if group
                    .last()
                    .is_some_and(|last| last.request_id != message.request_id)
                {
                    flush(&mut logical, &mut group, &mut replayed)?;
                }
                group.push(*message);
            }
            DisplayRecord::Compaction(marker) => {
                flush(&mut logical, &mut group, &mut replayed)?;
                let usage = marker
                    .replacement
                    .iter()
                    .rev()
                    .find_map(|message| message.turn_usage.clone());
                let request_id = marker.request_id.clone();
                let next = marker
                    .replacement
                    .into_iter()
                    .map(|message| message.message)
                    .collect::<Vec<_>>();
                history.append_turn(TranscriptTurn {
                    prev: &logical,
                    next: &next,
                    meta,
                    turn_usage: usage.as_ref(),
                    request_id: request_id.as_deref(),
                    tools: None,
                })?;
                replayed += next.len();
                logical = next;
            }
        }
    }
    flush(&mut logical, &mut group, &mut replayed)?;
    if let Some(tools) = tools {
        history.append_turn(TranscriptTurn {
            prev: &logical,
            next: &logical,
            meta,
            turn_usage: None,
            request_id: None,
            tools: Some(tools),
        })?;
    }
    Ok(replayed)
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
    if std::fs::symlink_metadata(raw)?.file_type().is_symlink() {
        anyhow::bail!("transcript root {} is a symbolic link", raw.display());
    }
    let mut flat = Vec::new();
    let mut dated = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(raw)
        .with_context(|| format!("read {}", raw.display()))?
        .collect::<std::io::Result<_>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            let mut inner: Vec<_> = std::fs::read_dir(&path)?.collect::<std::io::Result<_>>()?;
            inner.sort_by_key(std::fs::DirEntry::file_name);
            for entry in inner {
                if entry.file_type()?.is_file() {
                    push_jsonl(&mut dated, entry.path());
                }
            }
        } else if file_type.is_file() {
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
        let stores = provider
            .try_for_agent(&agent)
            .map_err(|error| anyhow::anyhow!("open the transcripts of {agent}: {error}"))?;
        for (stem, path) in files {
            let (meta, tools, records) = match decode_transcript(&path) {
                Ok(decoded) => decoded,
                Err(error) => {
                    tracing::warn!(
                        "[sessions] skipping unreadable transcript {}: {error}",
                        path.display()
                    );
                    skipped += 1;
                    continue;
                }
            };
            found += 1;
            let history = stores.transcripts.open_stem(&stem, meta.clone())?;
            if history.read_session()?.is_some() {
                continue;
            }
            replay_transcript(history.as_ref(), &meta, tools.as_ref(), records)
                .with_context(|| format!("[sessions] replay transcript {}", path.display()))?;
            written += 1;
        }
    }
    Ok((found, written, skipped))
}

#[cfg(test)]
#[path = "sessions_tests.rs"]
mod tests;
