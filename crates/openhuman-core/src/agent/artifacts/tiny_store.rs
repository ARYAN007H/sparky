//! `ArtifactStore` implementation for OpenHuman: versioned artifacts live
//! under `<workspace>/artifacts/versioned/`.
//!
//! The crate `tiny-artifacts` defines the versioned model (`ArtifactRecord`,
//! `ArtifactVersion`, `ArtifactAuthor`) and the [`ArtifactStore`] trait; this
//! module provides OpenHuman's storage backend for it. Records are persisted
//! one file per artifact as `<root>/versioned/<id>.json`, outside the
//! legacy `meta.json` layout the one-shot producers use, so the two systems
//! coexist.
//!
//! # UI bridge
//!
//! The desktop UI knows only the legacy artifact pipeline (an `ArtifactMeta`
//! row plus the `ArtifactPending` / `ArtifactReady` domain events), and it
//! only needs the *latest* revision of a deliverable — there is no version
//! stack in the UI. So every [`FsArtifactStore::upsert`] of a text-bearing
//! versioned artifact also mirrors the latest version into that legacy
//! pipeline, using the existing "regenerate in place" flow
//! ([`super::store::REGENERATE_TARGET_ID`]) so the same artifact id is
//! reused, the file it already owns is overwritten, and the UI card swaps in
//! place without flicker. Image/File kinds carry a *reference* (path), not
//! content, so they are not mirrored — their producers already file them
//! through the legacy path directly.

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use tiny_artifacts::{ArtifactKind, ArtifactRecord, ArtifactStore};

use super::files::FileRoots;
use super::types::ArtifactKind as LegacyArtifactKind;

/// Subdirectory under the artifacts root holding versioned records.
const VERSIONED_SUBDIR: &str = "versioned";

/// The artifacts root, mirroring [`super::store::artifacts_root`]:
/// `<workspace_dir>/artifacts/`.
async fn root(workspace_dir: &Path) -> Result<std::path::PathBuf, String> {
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

/// Mapping from the shared model's kinds to the legacy kinds the UI renders,
/// plus the file extension for the mirrored deliverable. `None` for kinds
/// whose body is a reference rather than content (image, file) — mirroring
/// those would write the reference string as the file's bytes.
fn legacy_mirror(kind: ArtifactKind) -> Option<(LegacyArtifactKind, &'static str)> {
    match kind {
        ArtifactKind::Text => Some((LegacyArtifactKind::Document, "txt")),
        ArtifactKind::Markdown => Some((LegacyArtifactKind::Document, "md")),
        ArtifactKind::Html => Some((LegacyArtifactKind::Document, "html")),
        ArtifactKind::Image | ArtifactKind::File => None,
    }
}

/// OpenHuman's storage backend for the shared versioned artifact model.
///
/// Each record is stored whole (JSON) at `<workspace>/artifacts/versioned/<id>.json`.
pub struct FsArtifactStore {
    workspace_dir: std::path::PathBuf,
    files: FileRoots,
}

impl FsArtifactStore {
    /// Creates a store rooted at `workspace_dir`; `files` is the visible
    /// files folder mirrored deliverables land in (same as the legacy
    /// producers use, see [`FileRoots`]).
    pub fn new(workspace_dir: impl Into<std::path::PathBuf>, files: FileRoots) -> Self {
        Self { workspace_dir: workspace_dir.into(), files }
    }

    async fn record_path(&self, id: &str) -> Result<std::path::PathBuf, String> {
        validate_id(id)?;
        let root = root(&self.workspace_dir).await?;
        let dir = root.join(VERSIONED_SUBDIR);
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| format!("[tiny-artifacts] failed to create versioned dir {:?}: {e}", dir))?;
        Ok(dir.join(format!("{id}.json")))
    }

    /// Mirror the latest revision into the legacy UI pipeline: mint (or
    /// regenerate) the legacy meta row under the same artifact id, write the
    /// latest body to the visible files folder, and publish the ready event.
    /// Best-effort — a mirror failure must not fail the versioned write.
    async fn mirror_to_legacy(&self, record: &ArtifactRecord) -> anyhow::Result<()> {
        let Some((legacy_kind, ext)) = legacy_mirror(record.kind) else {
            return Ok(());
        };
        let Some(latest) = record.latest() else {
            return Ok(());
        };
        // Reuse the existing in-place regenerate flow so the artifact id is
        // stable and the file an older revision already owns is overwritten
        // rather than claiming a second name (#5505).
        super::store::REGENERATE_TARGET_ID
            .scope(record.id.clone(), async {
                let (meta, path) = super::store::create_artifact(
                    &self.workspace_dir,
                    self.files.clone(),
                    legacy_kind,
                    &record.title,
                    ext,
                )
                .await
                .map_err(anyhow::Error::msg)?;
                tokio::fs::write(&path, latest.body.as_bytes()).await?;
                super::store::finalize_artifact(&self.workspace_dir, &meta.id, latest.body.len() as u64)
                    .await
                    .map_err(anyhow::Error::msg)?;
                Ok::<(), anyhow::Error>(())
            })
            .await?;
        Ok(())
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
        // UI bridge: surface the latest revision through the legacy pipeline.
        if let Err(err) = self.mirror_to_legacy(artifact).await {
            tracing::warn!(
                target: "tiny-artifacts",
                err = %err,
                artifact_id = %artifact.id,
                "[tiny-artifacts] failed to mirror artifact into legacy UI pipeline"
            );
        }
        Ok(())
    }

    async fn delete(&self, id: &str) -> anyhow::Result<bool> {
        let path = self.record_path(id).await.map_err(anyhow::Error::msg)?;
        let removed = match tokio::fs::remove_file(&path).await {
            Ok(()) => true,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
            Err(e) => return Err(e.into()),
        };
        if removed {
            // Drop the mirrored legacy row too so the Files panel does not
            // keep listing a deleted artifact.
            let _ = super::store::delete_artifact(&self.workspace_dir, self.files.clone(), id)
                .await;
        }
        Ok(removed)
    }
}

/// Build the versioned artifact tools bound to this store — the four tools in
/// the crate's catalogue order: `create_artifact`, `read_artifact`,
/// `update_artifact`, `list_artifacts`.
pub fn registered_tools(workspace_dir: &Path, files: FileRoots) -> Vec<Box<dyn tinytools::Tool>> {
    let store: Arc<dyn ArtifactStore> = Arc::new(FsArtifactStore::new(workspace_dir, files));
    tiny_artifacts::registered_tools(store)
}
