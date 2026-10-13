//! Ids of thread messages that were delivered remixed, kept at
//! `<workspace>/tinymemes/remixed.json`, so the next reading can tell the
//! module which assistant turns carry the bot's own style.
//!
//! One store per workspace. Writes are atomic (temp file, then rename).

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};

use crate::security::secret_storage::file_store::write_atomic;

pub(super) const DIR: &str = "tinymemes";
const FILE: &str = "remixed.json";
/// Remixed-message ids remembered per workspace (oldest dropped first).
const CAP: usize = 4000;

pub(super) struct RemixedIds {
    path: PathBuf,
    ids: Mutex<VecDeque<String>>,
}

static STORES: LazyLock<Mutex<HashMap<PathBuf, Arc<RemixedIds>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// The store for a workspace, loaded from disk on first use. `None` when the
/// state directory cannot be created.
pub(super) fn for_workspace(workspace_dir: &Path) -> Option<Arc<RemixedIds>> {
    let mut stores = STORES.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(store) = stores.get(workspace_dir) {
        return Some(store.clone());
    }
    let dir = workspace_dir.join(DIR);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        log::warn!("[tinymemes] cannot create state dir: {e}");
        return None;
    }
    let store = Arc::new(RemixedIds::open(dir.join(FILE)));
    stores.insert(workspace_dir.to_path_buf(), store.clone());
    Some(store)
}

impl RemixedIds {
    /// A store over `path`, with the newest [`CAP`] ids from disk.
    pub(super) fn open(path: PathBuf) -> Self {
        let mut ids: VecDeque<String> = std::fs::read_to_string(&path)
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();
        cap(&mut ids);
        Self {
            path,
            ids: Mutex::new(ids),
        }
    }

    pub(super) fn contains(&self, message_id: &str) -> bool {
        self.ids
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .any(|id| id == message_id)
    }

    /// Record a delivered remix. The list stays locked through the write, so
    /// concurrent calls persist in order.
    pub(super) fn mark(&self, message_id: String) {
        let mut ids = self.ids.lock().unwrap_or_else(|e| e.into_inner());
        if ids.contains(&message_id) {
            return;
        }
        ids.push_back(message_id);
        cap(&mut ids);
        let snapshot = serde_json::to_string(&*ids).unwrap_or_default();
        if let Err(e) = write_atomic(&self.path, snapshot.as_bytes()) {
            log::warn!("[tinymemes] cannot save remixed ids: {e}");
        }
    }
}

fn cap(ids: &mut VecDeque<String>) {
    while ids.len() > CAP {
        ids.pop_front();
    }
}

#[cfg(test)]
#[path = "remixed_tests.rs"]
mod tests;
