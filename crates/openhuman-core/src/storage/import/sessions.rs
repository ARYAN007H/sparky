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
use tinyagents_session::run_ledger::{ImportCounts, LegacyDatabaseReport, LegacySessionImport};
use tinyagents_session::transcript::{
    CompactionMarker, DisplayMessage, DisplayRecord, SessionRef, TranscriptHistory,
    TranscriptLocator, TranscriptMessage, TranscriptMeta, TranscriptPartial, TranscriptTurn,
    TurnUsage, session_stem,
};
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
            let report =
                crate::storage::block_on(async move { ledger.import_legacy_batch(batch).await })?
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
            files.extend(
                children
                    .into_iter()
                    .map(|child| child.path())
                    .filter(|path| {
                        path.extension().and_then(|extension| extension.to_str()) == Some("json")
                    }),
            );
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
            files.push(path);
        }
    }
    Ok(files)
}

#[derive(serde::Deserialize)]
struct LegacyMetaLine {
    #[serde(rename = "_meta")]
    meta: LegacyMeta,
}

#[derive(serde::Deserialize)]
struct LegacyMeta {
    agent: String,
    #[serde(default)]
    agent_id: Option<String>,
    #[serde(default)]
    agent_type: Option<String>,
    dispatcher: String,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    model: Option<String>,
    created: String,
    updated: String,
    #[serde(default)]
    turn_count: usize,
    #[serde(default)]
    prefix_message_count: Option<usize>,
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cached_input_tokens: u64,
    #[serde(default)]
    charged_amount_usd: f64,
    #[serde(default)]
    thread_id: Option<String>,
    #[serde(default)]
    task_id: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    parent_session_id: Option<String>,
}

impl From<LegacyMeta> for TranscriptMeta {
    fn from(meta: LegacyMeta) -> Self {
        Self {
            agent_name: meta.agent,
            agent_id: meta.agent_id,
            agent_type: meta.agent_type,
            dispatcher: meta.dispatcher,
            provider: meta.provider,
            model: meta.model,
            created: meta.created,
            updated: meta.updated,
            turn_count: meta.turn_count,
            prefix_message_count: meta.prefix_message_count,
            input_tokens: meta.input_tokens,
            output_tokens: meta.output_tokens,
            cached_input_tokens: meta.cached_input_tokens,
            charged_amount_usd: meta.charged_amount_usd,
            thread_id: meta.thread_id,
            task_id: meta.task_id,
            session_id: meta.session_id,
            parent_session_id: meta.parent_session_id,
        }
    }
}

#[derive(serde::Deserialize)]
struct LegacyMessageLine {
    role: String,
    content: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    extra_metadata: Option<serde_json::Value>,
    #[serde(default)]
    cache_breakpoints: Vec<usize>,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    usage: Option<tinyagents_session::transcript::MessageUsage>,
    #[serde(default)]
    reasoning_content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<tinyagents_session::transcript::TranscriptToolCall>,
    #[serde(default)]
    iteration: Option<u32>,
    #[serde(default)]
    ts: Option<String>,
    #[serde(default)]
    request_id: Option<String>,
    #[serde(default)]
    interrupted: bool,
    #[serde(default)]
    failure: bool,
    #[serde(default)]
    failure_detail: Option<String>,
    #[serde(default)]
    tool_call_id: Option<String>,
    #[serde(default)]
    parts: Option<Vec<tinyagents_session::transcript::TranscriptPart>>,
    #[serde(default)]
    background: Option<tinyagents_session::transcript::BackgroundOrigin>,
}

impl LegacyMessageLine {
    fn into_display(self) -> DisplayMessage {
        let turn_usage = match (self.provider, self.model, self.usage, self.ts.clone()) {
            (Some(provider), Some(model), Some(usage), Some(ts)) if self.role == "assistant" => {
                Some(TurnUsage {
                    provider,
                    model,
                    usage,
                    ts,
                    reasoning_content: self.reasoning_content.clone(),
                    tool_calls: self.tool_calls.clone(),
                    iteration: self.iteration.unwrap_or_default(),
                })
            }
            _ => None,
        };
        let message = TranscriptMessage {
            id: self.id,
            role: self.role,
            content: self.content,
            tool_calls: self.tool_calls,
            tool_call_id: self.tool_call_id,
            parts: self.parts,
            legacy: Default::default(),
            extra_metadata: self.extra_metadata,
            cache_breakpoints: self.cache_breakpoints,
            turn_usage: turn_usage.clone(),
            request_id: self.request_id.clone(),
            preserve_request_id: true,
            interrupted: self.interrupted,
            tool_failure: self
                .failure
                .then(|| tinyagents_session::transcript::ToolFailure {
                    failed: true,
                    detail: self.failure_detail.clone(),
                }),
        }
        .normalized();
        let message = if let Some(origin) = &self.background {
            let mut message = message;
            let mut metadata = message
                .extra_metadata
                .take()
                .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));
            if !metadata.is_object() {
                let mut object = serde_json::Map::new();
                object.insert("value".into(), metadata);
                metadata = serde_json::Value::Object(object);
            }
            if let Some(object) = metadata.as_object_mut() {
                object.insert(
                    tinyagents_session::transcript::BACKGROUND_METADATA_KEY.into(),
                    serde_json::to_value(origin).unwrap_or(serde_json::Value::Null),
                );
            }
            message.extra_metadata = Some(metadata);
            message
        } else {
            message
        };
        DisplayMessage {
            message,
            interrupted: self.interrupted,
            request_id: self.request_id,
            iteration: self.iteration,
            ts: self.ts,
            reasoning_content: self.reasoning_content.or_else(|| {
                turn_usage
                    .as_ref()
                    .and_then(|usage| usage.reasoning_content.clone())
            }),
            turn_usage,
            failure: self.failure,
            failure_detail: self.failure_detail,
            background: self.background,
        }
    }
}

fn decode_transcript(
    path: &Path,
) -> Result<(
    TranscriptMeta,
    Option<serde_json::Value>,
    Vec<DisplayRecord>,
)> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("read transcript {}", path.display()))?;
    let mut meta = None;
    let mut tools = None;
    let mut records = Vec::new();
    let mut seen_first = false;
    for (line_number, bytes) in bytes.split(|byte| *byte == b'\n').enumerate() {
        let line = match std::str::from_utf8(bytes) {
            Ok(line) => line,
            Err(error) if !seen_first => {
                anyhow::bail!(
                    "first transcript record in {} is not valid UTF-8: {error}",
                    path.display()
                );
            }
            Err(error) => {
                tracing::warn!(
                    "[sessions] skipping invalid UTF-8 transcript line {} in {}: {error}",
                    line_number + 1,
                    path.display()
                );
                continue;
            }
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let first_record = !seen_first;
        seen_first = true;
        let value: serde_json::Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(error) if first_record => return Err(error.into()),
            Err(error) => {
                tracing::warn!(
                    "[sessions] skipping malformed transcript line {} in {}: {error}",
                    line_number + 1,
                    path.display()
                );
                continue;
            }
        };
        if value.get("_meta").is_some() {
            match serde_json::from_value::<LegacyMetaLine>(value) {
                Ok(line) => meta = Some(line.meta.into()),
                Err(error) if meta.is_none() => return Err(error.into()),
                Err(error) => tracing::warn!(
                    "[sessions] skipping malformed transcript metadata line {}: {error}",
                    line_number + 1
                ),
            }
        } else if value.get("kind").and_then(serde_json::Value::as_str) == Some("tools") {
            tools = value.get("tools").cloned();
        } else if value.get("kind").and_then(serde_json::Value::as_str) == Some("compaction") {
            #[derive(serde::Deserialize)]
            struct Compaction {
                #[serde(default)]
                replacement: Vec<LegacyMessageLine>,
                #[serde(default)]
                ts: Option<String>,
                #[serde(default)]
                request_id: Option<String>,
            }
            match serde_json::from_value::<Compaction>(value) {
                Ok(compaction) => records.push(DisplayRecord::Compaction(CompactionMarker {
                    replacement: compaction
                        .replacement
                        .into_iter()
                        .collect::<Vec<_>>()
                        .into_iter()
                        .map(|line| line.into_display())
                        .collect(),
                    ts: compaction.ts,
                    request_id: compaction.request_id,
                })),
                Err(error) => tracing::warn!(
                    "[sessions] skipping malformed compaction line {}: {error}",
                    line_number + 1
                ),
            }
        } else {
            match serde_json::from_value::<LegacyMessageLine>(value) {
                Ok(line) => records.push(DisplayRecord::Message(Box::new(line.into_display()))),
                Err(error) => tracing::warn!(
                    "[sessions] skipping unknown transcript line {}: {error}",
                    line_number + 1
                ),
            }
        }
    }
    Ok((
        meta.context("transcript has no _meta record")?,
        tools,
        records,
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
