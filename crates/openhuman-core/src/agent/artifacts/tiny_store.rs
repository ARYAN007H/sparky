//! `ArtifactStore` implementation for OpenHuman: versioned artifacts live
//! under `<workspace>/artifacts/versioned/`.
//!
//! The crate `tiny-artifacts` defines the versioned model (`ArtifactRecord`,
//! `ArtifactVersion`, `ArtifactAuthor`) and the [`ArtifactStore`] trait; this
//! module provides OpenHuman's storage backend for it. Records are persisted
//! one file per artifact as `<root>/versioned/<id>.json`, outside the
//! legacy `meta.json` layout the one-shot producers use, so the two systems
//! coexist.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use tiny_artifacts::{ArtifactRecord, ArtifactStore};

/// Subdirectory under the artifacts root holding versioned records.
const VERSIONED_SUBDIR: &str = "versioned";

/// The artifacts root, mirroring [`super::store::artifacts_root`]:
/// `<workspace_dir>/artifacts/`.
async fn root(workspace_dir: &Path) -> Result<PathBuf, String> {
    let root = workspace_dir.join("artifacts");
    tokio::fs::create_dir_all(&root)
        .await
        .map_err(|e| format!("[tiny-artifacts] failed to create artifacts root {:?}: {e}", root))?;
    Ok(root)
}

/// Reject ids that could escape the root as a path component.
fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("[tiny-artifacts] artifact id must not be empty".into());
    }
    if id == "." || id == ".." {
        return Err(format!("[tiny-artifacts] invalid artifact id: {id:?}"));
    }
    if id.contains('/') || id.contains('\\') {
        return Err(format!("[tiny-artifacts] artifact id must not contain path separators: {id:?}"));
    }
    if id.starts_with('/') || id.starts_with('\\') {
        return Err(format!("[tiny-artifacts] artifact id must not be absolute: {id:?}"));
    }
    Ok(())
}

/// OpenHuman's storage backend for the shared versioned artifact model.
///
/// Each record is stored whole (JSON) at `<workspace>/artifacts/versioned/<id>.json`.
pub struct FsArtifactStore {
    workspace_dir: PathBuf,
}

impl FsArtifactStore {
    /// Creates a store rooted at `workspace_dir`.
    pub fn new(workspace_dir: impl Into<PathBuf>) -> Self {
        Self { workspace_dir: workspace_dir.into() }
    }

    async fn record_path(&self, id: &str) -> Result<PathBuf, String> {
        validate_id(id)?;
        let root = root(&self.workspace_dir).await?;
        let dir = root.join(VERSIONED_SUBDIR);
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| format!("[tiny-artifacts] failed to create versioned dir {:?}: {e}", dir))?;
        Ok(dir.join(format!("{id}.json")))
    }
}

#[async_trait]
impl ArtifactStore for FsArtifactStore {
    async fn list(&self) -> anyhow::Result<Vec<ArtifactRecord>> {
        let root = root(&self.workspace_dir).await.map_err(anyhow::Error::msg)?;
        let dir = root.join(VERSIONED_SUBDIR);
        let mut records = Vec::new();
        let mut entries = match tokio::fs::read_dir(&dir).await {
            Ok(e) => e,
            Err(_) => return Ok(records), // no versioned dir yet → nothing stored
        };
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let bytes = tokio::fs::read(&path).await?;
            if let Ok(record) = serde_json::from_slice::<ArtifactRecord>(&bytes) {
                records.push(record);
            }
        }
        // Most-recently-updated first, mirroring the crate's documented order.
        records.sort_by(|a, b| b.updated_at_millis.cmp(&a.updated_at_millis));
        Ok(records)
    }

    async fn get(&self, id: &str) -> anyhow::Result<Option<ArtifactRecord>> {
        let path = self.record_path(id).await.map_err(anyhow::Error::msg)?;
        let bytes = match tokio::fs::read(&path).await {
            Ok(b) => b,
            Err(_) => return Ok(None),
        };
        let record = serde_json::from_slice(&bytes)?;
        Ok(Some(record))
    }

    async fn upsert(&self, artifact: &ArtifactRecord) -> anyhow::Result<()> {
        let path = self.record_path(&artifact.id).await.map_err(anyhow::Error::msg)?;
        let bytes = serde_json::to_vec(artifact)?;
        tokio::fs::write(&path, bytes).await?;
        Ok(())
    }

    async fn delete(&self, id: &str) -> anyhow::Result<bool> {
        let path = self.record_path(id).await.map_err(anyhow::Error::msg)?;
        match tokio::fs::remove_file(&path).await {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e.into()),
        }
    }
}

/// Build the versioned artifact tools bound to this store — the four tools in
/// the crate's catalogue order: `create_artifact`, `read_artifact`,
/// `update_artifact`, `list_artifacts`.
pub fn registered_tools(workspace_dir: &Path) -> Vec<Box<dyn tinytools::Tool>> {
    let store: Arc<dyn ArtifactStore> = Arc::new(FsArtifactStore::new(workspace_dir));
    tiny_artifacts::registered_tools(store)
}
