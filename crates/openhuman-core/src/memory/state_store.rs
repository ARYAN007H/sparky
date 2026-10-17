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

/// State value `name`, or `None` when none was stored.
///
/// # Errors
///
/// A storage error, or a stored value that is not a `T`.
pub(crate) fn load<T: DeserializeOwned + Send + 'static>(
    workspace_dir: &Path,
    name: &str,
) -> Result<Option<T>> {
    open(workspace_dir, name)?.get(name)
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
    let kv = open(workspace_dir, name)?;
    if kv.insert_if_absent(name, value)? {
        return Ok(value.clone());
    }
    kv.get(name)?
        .ok_or_else(|| anyhow::anyhow!("the {name} record vanished"))
}
