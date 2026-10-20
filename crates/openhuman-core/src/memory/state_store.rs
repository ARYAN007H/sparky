//! The memory domain's small runtime state values (`jobs`, `channel_threads`,
//! `pending_deletions`, `sources_state`, `import_state`,
//! `conversations_backfill`, `layout_migration`, `local_root`) on the storage
//! port.
//!
//! Each used to be `<workspace>/memory/<name>.json`. Each is now one
//! `tsd.kv` entry (owner `memory`, namespace `state`, key `<name>`) in the
//! acting scope; the first read in a backend imports the legacy file once
//! ([`StateKv::import_file`]) and leaves it in place.

use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sha2::Digest;

use crate::storage::StorageBackend;
use crate::storage::state_kv::StateKv;

const OWNER: &str = "memory";
const NS: &str = "state";

/// The legacy file of state value `name` in `workspace_dir`.
#[must_use]
pub(crate) fn legacy_path(workspace_dir: &Path, name: &str) -> PathBuf {
    workspace_dir.join("memory").join(format!("{name}.json"))
}

fn open(workspace_dir: &Path, name: &str) -> Result<StateKv> {
    let kv = StateKv::open(workspace_dir, OWNER, NS)?;
    kv.import_file(name, &legacy_path(workspace_dir, name))?;
    Ok(kv)
}

fn open_quarantining<T: DeserializeOwned>(workspace_dir: &Path, name: &str) -> Result<StateKv> {
    let kv = StateKv::open(workspace_dir, OWNER, NS)?;
    kv.import_file_quarantining::<T>(name, &legacy_path(workspace_dir, name))?;
    Ok(kv)
}

fn workspace_scope(workspace_dir: &Path) -> Result<crate::storage::Scope> {
    let path = std::fs::canonicalize(workspace_dir).unwrap_or_else(|_| workspace_dir.to_path_buf());
    let digest = sha2::Sha256::digest(path.to_string_lossy().as_bytes());
    let suffix = digest
        .iter()
        .take(12)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    crate::storage::scope_for_module(None, &format!("memory-workspace-{suffix}"))
}

fn open_workspace_quarantining<T: DeserializeOwned>(
    workspace_dir: &Path,
    name: &str,
) -> Result<StateKv> {
    let scope = workspace_scope(workspace_dir)?;
    let kv = StateKv::open_in_scope(workspace_dir, &scope, OWNER, NS)?;
    kv.import_file_quarantining::<T>(name, &legacy_path(workspace_dir, name))?;
    Ok(kv)
}

/// A workspace-owned state value in a deterministic local module scope. Use
/// this for queues that must remain readable after the ambient profile scope
/// has ended, while remaining isolated from other workspaces on shared storage.
pub(crate) fn load_workspace<T: DeserializeOwned + Send + 'static>(
    workspace_dir: &Path,
    name: &str,
) -> Result<Option<T>> {
    open_workspace_quarantining::<T>(workspace_dir, name)?.get(name)
}

/// Stores a workspace-owned state value in its deterministic local module scope.
pub(crate) fn save_workspace<T>(workspace_dir: &Path, name: &str, value: &T) -> Result<()>
where
    T: Serialize + DeserializeOwned + Send + 'static,
{
    open_workspace_quarantining::<T>(workspace_dir, name)?.put(name, value)
}

/// Removes a workspace-owned state value from its deterministic local module scope.
pub(crate) fn remove_workspace<T: DeserializeOwned + Send + 'static>(
    workspace_dir: &Path,
    name: &str,
) -> Result<()> {
    open_workspace_quarantining::<T>(workspace_dir, name)?.delete(name)?;
    Ok(())
}

/// [`open`] for a value a damaged legacy file must not reset.
fn open_strict(workspace_dir: &Path, name: &str) -> Result<StateKv> {
    let kv = StateKv::open(workspace_dir, OWNER, NS)?;
    kv.import_file_strict(name, &legacy_path(workspace_dir, name))?;
    Ok(kv)
}

/// Imports every `<workspace>/memory/*.json` state file into the `local` scope
/// of an explicit `scoped` handle, once each. For the layout migration
/// (`storage::layout`), which deletes those files afterwards; a file that is
/// not JSON is an error here (nothing may be deleted unimported), so it is
/// imported strictly. Returns the names imported.
///
/// # Errors
///
/// A storage error or an unparsable file.
pub(crate) fn import_all(
    backend: &std::sync::Arc<dyn crate::storage::StorageBackend>,
    workspace_dir: &Path,
) -> Result<Vec<String>> {
    let local = backend.for_scope(&crate::storage::Scope::local())?;
    let local_kv = StateKv::over(&local, OWNER, NS);
    let queue_scope = workspace_scope(workspace_dir)?;
    let queue_kv = StateKv::on_backend(backend, &queue_scope, OWNER, NS)?;
    let mut names = Vec::new();
    let Ok(entries) = std::fs::read_dir(workspace_dir.join("memory")) else {
        return Ok(names);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path
            .file_name()
            .and_then(|f| f.to_str())
            .and_then(|f| f.strip_suffix(".json"))
            .map(str::to_owned)
        else {
            continue;
        };
        if !path.is_file() {
            continue;
        }
        if name == "pending_deletions" {
            queue_kv.import_file_strict(&name, &path)?;
        } else {
            local_kv.import_file_strict(&name, &path)?;
        }
        names.push(name);
    }
    // Earlier versions imported this queue with the rest of memory state in
    // `local`. Move any such durable entries to the workspace-owned scope
    // before the layout migrator archives the old files. Remove the source
    // only after the target write succeeds so an interrupted migration can
    // retry without losing owed deletions.
    if let Some(previous) = local_kv.get::<Vec<super::deletion::PendingDeletion>>("pending_deletions")? {
        let mut pending = queue_kv
            .get::<Vec<super::deletion::PendingDeletion>>("pending_deletions")?
            .unwrap_or_default();
        for deletion in previous {
            if !pending.contains(&deletion) {
                pending.push(deletion);
            }
        }
        if pending.is_empty() {
            queue_kv.delete("pending_deletions")?;
        } else {
            queue_kv.put("pending_deletions", &pending)?;
        }
        local_kv.delete("pending_deletions")?;
    }
    names.sort();
    Ok(names)
}

/// [`load`], failing on a damaged legacy file instead of starting over.
///
/// # Errors
///
/// A storage error, an unparsable legacy file, or a stored value that is not
/// a `T`.
pub(crate) fn load_strict<T: DeserializeOwned + Send + 'static>(
    workspace_dir: &Path,
    name: &str,
) -> Result<Option<T>> {
    open_strict(workspace_dir, name)?.get(name)
}

/// State value `name`, or `None` when none was stored.
///
/// # Errors
///
/// A storage error, or a stored value that is not a `T`.
pub(crate) fn load<T: DeserializeOwned + Send + 'static>(
    workspace_dir: &Path,
    name: &str,
) -> Result<Option<T>> {
    open_quarantining::<T>(workspace_dir, name)?.get(name)
}

/// The names of the state values starting with `prefix`: those stored, plus
/// those whose legacy file is still in `<workspace>/memory/` (each is
/// imported first, so a name appears once).
///
/// # Errors
///
/// A storage error, or an unparsable legacy file.
pub(crate) fn names(workspace_dir: &Path, prefix: &str) -> Result<Vec<String>> {
    if let Ok(entries) = std::fs::read_dir(workspace_dir.join("memory")) {
        for entry in entries.flatten() {
            let file = entry.file_name();
            let Some(name) = file
                .to_str()
                .and_then(|file| file.strip_suffix(".json"))
                .filter(|name| name.starts_with(prefix))
            else {
                continue;
            };
            open_strict(workspace_dir, name)?;
        }
    }
    let kv = StateKv::open(workspace_dir, OWNER, NS)?;
    Ok(kv
        .list::<serde_json::Value>(prefix)?
        .into_iter()
        .map(|(name, _)| name)
        .collect())
}

/// Stores `value` as state value `name`.
///
/// # Errors
///
/// A storage error.
pub(crate) fn save<T: Serialize>(workspace_dir: &Path, name: &str, value: &T) -> Result<()> {
    open(workspace_dir, name)?.put(name, value)
}

/// Removes state value `name`.
///
/// # Errors
///
/// A storage error.
pub(crate) fn remove(workspace_dir: &Path, name: &str) -> Result<()> {
    open(workspace_dir, name)?.delete(name).map(|_| ())
}

/// Stores `value` as `name` only when none exists, and returns what is stored.
///
/// # Errors
///
/// A storage error.
pub(crate) fn record_once<T>(workspace_dir: &Path, name: &str, value: &T) -> Result<T>
where
    T: Serialize + DeserializeOwned + Clone + Send + 'static,
{
    let kv = open_strict(workspace_dir, name)?;
    if kv.insert_if_absent(name, value)? {
        return Ok(value.clone());
    }
    kv.get(name)?
        .ok_or_else(|| anyhow::anyhow!("the {name} record vanished"))
}

#[cfg(test)]
#[path = "state_store_tests.rs"]
mod tests;
