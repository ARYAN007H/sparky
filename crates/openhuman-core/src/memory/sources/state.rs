//! Per-source sync state: last run, status, error and item count.
//!
//! This is runtime state, not configuration, so it lives in
//! `<workspace>/memory/sources_state.json` rather than `config.toml`. A source
//! with no entry has never synced. A `syncing` status found on load belongs to
//! a run the previous process never finished, and reads as `idle`.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{LazyLock, Mutex};

use tinymemory_integrations::sources::{SourceState, SourceStatus};

/// Serialises state-file read-modify-writes in this process.
static LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

const STATE: &str = "sources_state";

fn read_all(workspace_dir: &Path) -> BTreeMap<String, SourceState> {
    crate::memory::state_store::load(workspace_dir, STATE)
        .unwrap_or_else(|error| {
            tracing::warn!(error = %error, "[memory:sources] reading sync state failed");
            None
        })
        .unwrap_or_default()
}

fn write_all(workspace_dir: &Path, all: &BTreeMap<String, SourceState>) {
    if let Err(error) = crate::memory::state_store::save(workspace_dir, STATE, all) {
        tracing::warn!(error = %error, "[memory:sources] writing sync state failed");
    }
}

/// Every source's state.
#[must_use]
pub fn load(workspace_dir: &Path) -> BTreeMap<String, SourceState> {
    read_all(workspace_dir)
}

/// Applies `change` to `id`'s state and persists it.
pub fn update(workspace_dir: &Path, id: &str, change: impl FnOnce(&mut SourceState)) {
    let _guard = LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut all = read_all(workspace_dir);
    change(all.entry(id.to_string()).or_default());
    write_all(workspace_dir, &all);
}

/// Drops `id`'s state.
pub fn remove(workspace_dir: &Path, id: &str) {
    let _guard = LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut all = read_all(workspace_dir);
    if all.remove(id).is_some() {
        write_all(workspace_dir, &all);
    }
}

/// Marks every `syncing` source idle: called once at startup, when no sync
/// can be running yet.
pub fn reset_interrupted(workspace_dir: &Path) {
    let _guard = LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut all = read_all(workspace_dir);
    let changed = all
        .values()
        .any(|state| state.status == SourceStatus::Syncing);
    if changed {
        tinymemory_integrations::sources::reset_interrupted(&mut all);
        write_all(workspace_dir, &all);
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
