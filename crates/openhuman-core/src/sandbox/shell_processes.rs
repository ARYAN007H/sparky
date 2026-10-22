//! The shell commands a turn left running past their yield point.
//!
//! The `shell` tool adopts a [`LiveShell`] here and gives the model its id
//! (`p1`, `p2`, …); the `shell_process` tool waits on it, stops it or reports
//! its status. One registry belongs to one turn's run context, so dropping it
//! at the end of the turn stops every process still running (each
//! [`LiveHandle`] stops its own on drop). Every process also stops at its own
//! runtime budget, which the turn budget already clamps.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::shell_live::{LiveHandle, LiveShell, LiveState};

/// Processes a turn may leave running at once. A further command runs to its
/// end the usual way.
pub(crate) const MAX_RUNNING: usize = 4;

/// Consecutive waits with no new output after which the report suggests
/// stopping the process.
pub(crate) const QUIET_WAITS_NOTE: u32 = 3;

/// How a process ended, kept for later questions about it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ended {
    Exited(i32),
    TimedOut,
    Stopped,
}

/// What a wait, stop or status call found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ReportState {
    /// Still running.
    Running,
    /// Ended during this call.
    Ended(Ended),
    /// Had already ended, and its output was already returned.
    AlreadyEnded(Ended),
}

/// One answer about a process, for the tool to render.
#[derive(Clone, Debug)]
pub(crate) struct Report {
    pub(crate) id: String,
    pub(crate) command: String,
    /// Time since the command started.
    pub(crate) elapsed: Duration,
    /// The process's runtime budget.
    pub(crate) timeout: Duration,
    /// Output written since the previous answer.
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) state: ReportState,
    /// Consecutive waits, this one included, that brought no new output.
    pub(crate) quiet_waits: u32,
}

struct Entry {
    handle: LiveHandle,
    command: String,
    started: Instant,
    timeout: Duration,
    ended: Option<Ended>,
    quiet_waits: u32,
}

/// The running processes of one turn.
#[derive(Default)]
pub(crate) struct ShellProcesses {
    next: AtomicU32,
    entries: Mutex<BTreeMap<String, Arc<tokio::sync::Mutex<Entry>>>>,
    running: Mutex<usize>,
}

/// A registry is equal only to itself: two turns never share one.
impl PartialEq for ShellProcesses {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for ShellProcesses {}

impl ShellProcesses {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Whether another process may be left running.
    pub(crate) fn has_room(&self) -> bool {
        *self.running() < MAX_RUNNING
    }

    /// Take over a process still running at its yield point and name it.
    pub(crate) fn adopt(
        &self,
        live: LiveShell,
        command: &str,
        started: Instant,
        timeout: Duration,
    ) -> String {
        let id = format!("p{}", self.next.fetch_add(1, Ordering::SeqCst) + 1);
        *self.running() += 1;
        self.entries().insert(
            id.clone(),
            Arc::new(tokio::sync::Mutex::new(Entry {
                handle: live.handle,
                command: command.to_owned(),
                started,
                timeout,
                ended: None,
                quiet_waits: 0,
            })),
        );
        tracing::debug!(process = %id, "[shell] process left running past its yield point");
        id
    }

    /// Wait up to `wait` for new output or the end of the process.
    pub(crate) async fn wait(&self, id: &str, wait: Duration) -> Result<Report, String> {
        let entry = self.entry(id)?;
        let mut entry = entry.lock().await;
        if let Some(ended) = entry.ended {
            return Ok(report(
                id,
                &entry,
                String::new(),
                String::new(),
                ReportState::AlreadyEnded(ended),
            ));
        }
        let read = entry
            .handle
            .read(wait)
            .await
            .map_err(|error| format!("Could not read process {id}: {error}"))?;
        let quiet = read.stdout.is_empty() && read.stderr.is_empty();
        entry.quiet_waits = if quiet { entry.quiet_waits + 1 } else { 0 };
        let state = self.settle(&mut entry, read.state);
        Ok(report(id, &entry, read.stdout, read.stderr, state))
    }

    /// Stop the process and return what it wrote since the previous answer.
    pub(crate) async fn kill(&self, id: &str) -> Result<Report, String> {
        let entry = self.entry(id)?;
        let mut entry = entry.lock().await;
        if let Some(ended) = entry.ended {
            return Ok(report(
                id,
                &entry,
                String::new(),
                String::new(),
                ReportState::AlreadyEnded(ended),
            ));
        }
        tracing::debug!(process = %id, "[shell] stopping a running process on request");
        let read = entry
            .handle
            .kill()
            .await
            .map_err(|error| format!("Could not stop process {id}: {error}"))?;
        let state = self.settle(&mut entry, read.state);
        Ok(report(id, &entry, read.stdout, read.stderr, state))
    }

    /// Where the process is, without its output.
    pub(crate) async fn status(&self, id: &str) -> Result<Report, String> {
        let entry = self.entry(id)?;
        let entry = entry.lock().await;
        let state = match entry.ended {
            Some(ended) => ReportState::AlreadyEnded(ended),
            None => ReportState::Running,
        };
        Ok(report(id, &entry, String::new(), String::new(), state))
    }

    /// Record an ending, once.
    fn settle(&self, entry: &mut Entry, state: LiveState) -> ReportState {
        let ended = match state {
            LiveState::Running => return ReportState::Running,
            LiveState::Exited(code) => Ended::Exited(code),
            LiveState::TimedOut => Ended::TimedOut,
            LiveState::Stopped => Ended::Stopped,
        };
        if entry.ended.is_none() {
            entry.ended = Some(ended);
            let mut running = self.running();
            *running = running.saturating_sub(1);
        }
        ReportState::Ended(ended)
    }

    fn entry(&self, id: &str) -> Result<Arc<tokio::sync::Mutex<Entry>>, String> {
        let entries = self.entries();
        entries.get(id.trim()).cloned().ok_or_else(|| {
            let known: Vec<&str> = entries.keys().map(String::as_str).collect();
            if known.is_empty() {
                format!("No process {id} in this turn: no command has been left running.")
            } else {
                format!(
                    "No process {id} in this turn. Processes: {}.",
                    known.join(", ")
                )
            }
        })
    }

    fn entries(
        &self,
    ) -> std::sync::MutexGuard<'_, BTreeMap<String, Arc<tokio::sync::Mutex<Entry>>>> {
        self.entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn running(&self) -> std::sync::MutexGuard<'_, usize> {
        self.running
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

fn report(id: &str, entry: &Entry, stdout: String, stderr: String, state: ReportState) -> Report {
    Report {
        id: id.to_owned(),
        command: entry.command.clone(),
        elapsed: entry.started.elapsed(),
        timeout: entry.timeout,
        stdout,
        stderr,
        state,
        quiet_waits: entry.quiet_waits,
    }
}

#[cfg(test)]
#[path = "shell_processes_tests.rs"]
mod tests;
