//! OpenHuman's side of the versioned artifacts (`tiny-artifacts`).
//!
//! The crate owns what an artifact is and how it changes; every creation and
//! revision goes through its [`Artifacts`] service. OpenHuman supplies the
//! service's ports:
//!
//! * [`versioned_store`] — the crate's [`FsArtifactStore`] over
//!   `<workspace>/artifacts/versioned/` (one `<id>.json` per artifact);
//! * [`LegacyUiBridge`] — the observer that shows each artifact in the
//!   desktop UI;
//! * the folders file artifacts may point into: the agent's working folder
//!   and the visible files folder.
//!
//! # UI bridge
//!
//! The desktop UI knows only the legacy artifact pipeline (an `ArtifactMeta`
//! row plus the `ArtifactPending` / `ArtifactReady` domain events), and only
//! the newest version of each artifact. [`LegacyUiBridge`] turns every
//! service write into that pipeline: the newest version's content (or, for an
//! image or file artifact, a copy of its file) is written into the files folder
//! under the same artifact id, through the "regenerate in place" flow
//! ([`super::store::REGENERATE_TARGET_ID`]), so a revision overwrites the file
//! the artifact already owns and its card updates in place.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use tiny_artifacts::{
    ArtifactEvent, ArtifactEventKind, ArtifactKind, ArtifactObserver, ArtifactRecord, Artifacts,
    Author, Content,
};

use super::files::FileRoots;
use super::types::ArtifactKind as LegacyArtifactKind;

/// Subdirectory under the artifacts root holding versioned records.
const VERSIONED_SUBDIR: &str = "versioned";

/// The legacy kind and file extension the UI gets for an artifact's newest
/// version: text kinds by kind, file kinds by the file's extension.
fn legacy_shape(record: &ArtifactRecord) -> Option<(LegacyArtifactKind, String)> {
    let latest = record.latest()?;
    let shape = match record.kind {
        ArtifactKind::Text => (LegacyArtifactKind::Document, "txt".to_string()),
        ArtifactKind::Markdown => (LegacyArtifactKind::Document, "md".to_string()),
        ArtifactKind::Html => (LegacyArtifactKind::Document, "html".to_string()),
        ArtifactKind::Image | ArtifactKind::File => {
            let ext = latest
                .file
                .as_ref()
                .and_then(tiny_artifacts::FileRef::extension)
                .or_else(|| {
                    Path::new(&latest.body)
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(str::to_ascii_lowercase)
                })
                .unwrap_or_else(|| "bin".to_string());
            let kind = if record.kind == ArtifactKind::Image {
                LegacyArtifactKind::Image
            } else {
                legacy_kind_for_extension(&ext)
            };
            (kind, ext)
        }
    };
    Some(shape)
}

pub(crate) fn legacy_kind_for_extension(ext: &str) -> LegacyArtifactKind {
    match ext {
        "pptx" | "ppt" | "key" | "odp" => LegacyArtifactKind::Presentation,
        "docx" | "doc" | "pdf" | "md" | "txt" | "rtf" | "odt" | "html" | "htm" => {
            LegacyArtifactKind::Document
        }
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" => LegacyArtifactKind::Image,
        "mp4" | "webm" | "mov" => LegacyArtifactKind::Video,
        _ => LegacyArtifactKind::Other,
    }
}

pub use tiny_artifacts::FsArtifactStore;

/// The versioned records of a workspace: `tiny-artifacts`' file-system store
/// over `<workspace>/artifacts/versioned/` (one `<id>.json` per artifact).
pub fn versioned_store(workspace_dir: &Path) -> FsArtifactStore {
    FsArtifactStore::new(workspace_dir.join("artifacts").join(VERSIONED_SUBDIR))
}

/// Shows versioned artifacts in the desktop UI through the legacy pipeline.
pub struct LegacyUiBridge {
    workspace_dir: PathBuf,
    files: FileRoots,
}

impl LegacyUiBridge {
    /// A bridge writing into `files` (the visible files folder).
    pub fn new(workspace_dir: impl Into<PathBuf>, files: FileRoots) -> Self {
        Self {
            workspace_dir: workspace_dir.into(),
            files,
        }
    }

    /// Put the newest version in the files folder under the artifact's id and
    /// mark it ready: the content for text kinds, a copy of the file for file
    /// kinds (unless the file already is the one the artifact owns).
    async fn show_newest(&self, record: &ArtifactRecord) -> anyhow::Result<()> {
        let (Some(latest), Some((legacy_kind, ext))) = (record.latest(), legacy_shape(record))
        else {
            return Ok(());
        };
        // A producer writes its file straight into the row it reserved: the
        // row only needs marking ready (re-reserving would flash its card
        // back to "generating").
        if let Some(file) = latest.file.as_ref() {
            if let Ok(meta) = super::store::get_artifact(&self.workspace_dir, &record.id).await {
                let owned = meta.file.as_deref().map(Path::new);
                if owned.is_some_and(|owned| same_file(owned, Path::new(&file.path))) {
                    let size = tokio::fs::metadata(&file.path).await?.len();
                    super::store::finalize_artifact(&self.workspace_dir, &record.id, size)
                        .await
                        .map_err(anyhow::Error::msg)?;
                    return Ok(());
                }
            }
        }
        super::store::REGENERATE_TARGET_ID
            .scope(record.id.clone(), async {
                let (meta, dest) = super::store::create_artifact(
                    &self.workspace_dir,
                    self.files.clone(),
                    legacy_kind,
                    &record.title,
                    &ext,
                )
                .await
                .map_err(anyhow::Error::msg)?;
                let size = if record.holds_file() {
                    let source = latest
                        .file
                        .as_ref()
                        .map_or_else(|| PathBuf::from(&latest.body), |f| PathBuf::from(&f.path));
                    if same_file(&source, &dest) {
                        tokio::fs::metadata(&dest).await?.len()
                    } else {
                        tokio::fs::copy(&source, &dest).await?
                    }
                } else {
                    tokio::fs::write(&dest, latest.body.as_bytes()).await?;
                    latest.body.len() as u64
                };
                super::store::finalize_artifact(&self.workspace_dir, &meta.id, size)
                    .await
                    .map_err(anyhow::Error::msg)?;
                Ok::<(), anyhow::Error>(())
            })
            .await
    }
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

#[async_trait]
impl ArtifactObserver for LegacyUiBridge {
    async fn on_event(&self, event: &ArtifactEvent) -> anyhow::Result<()> {
        let id = &event.record.id;
        match event.kind {
            ArtifactEventKind::Ready => self.show_newest(&event.record).await,
            ArtifactEventKind::Failed => {
                let reason = event.record.error.as_deref().unwrap_or("failed");
                // Only an artifact the UI already shows has a row to fail.
                if super::store::get_artifact(&self.workspace_dir, id)
                    .await
                    .is_ok()
                {
                    super::store::fail_artifact(
                        &self.workspace_dir,
                        self.files.clone(),
                        id,
                        reason,
                    )
                    .await
                    .map_err(anyhow::Error::msg)?;
                }
                Ok(())
            }
            ArtifactEventKind::Deleted => {
                let _ = super::store::delete_artifact(&self.workspace_dir, self.files.clone(), id)
                    .await;
                Ok(())
            }
            // The legacy pipeline's pending row needs the file's extension,
            // which a pending artifact does not have yet; its card appears
            // when the first version lands.
            ArtifactEventKind::Pending => Ok(()),
        }
    }
}

/// The artifact service for one workspace: OpenHuman's store and UI bridge,
/// with file artifacts allowed from the agent's working folder and the files
/// folder.
pub fn artifacts_service(
    workspace_dir: &Path,
    files: FileRoots,
    action_dir: Option<&Path>,
) -> Artifacts {
    let mut roots: Vec<PathBuf> = action_dir.map(Path::to_path_buf).into_iter().collect();
    roots.push(files.current().to_path_buf());
    Artifacts::new(Arc::new(versioned_store(workspace_dir)))
        .with_observer(Arc::new(LegacyUiBridge::new(workspace_dir, files)))
        .with_file_roots(tiny_artifacts::FileRoots::new(roots))
}

/// Build the artifact tools bound to this workspace's service: the crate's
/// default set (`create_artifact`, `read_artifact`, `update_artifact`,
/// `list_artifacts`), `delete_artifact` (default-off through its
/// `tools::user_filter` family), and — only when a `publisher` exists, i.e. the
/// user can use cloud file storage — `share_artifact`.
pub fn registered_tools(
    workspace_dir: &Path,
    files: FileRoots,
    action_dir: &Path,
    publisher: Option<Arc<dyn tiny_artifacts::ArtifactPublisher>>,
) -> Vec<Box<dyn tinytools::Tool>> {
    let mut service = artifacts_service(workspace_dir, files, Some(action_dir));
    if let Some(publisher) = publisher {
        service = service.with_publisher(publisher);
    }
    let artifacts = Arc::new(service);
    let mut tools =
        tiny_artifacts::registered_tools(artifacts.clone(), Author::agent(AGENT_AUTHOR_ID));
    tools.push(Box::new(tiny_artifacts::DeleteArtifactTool::new(
        artifacts.clone(),
    )));
    if artifacts.can_publish() {
        tools.push(Box::new(tiny_artifacts::ShareArtifactTool::new(artifacts)));
    }
    tools
}

/// Author id stamped on versions the user writes in the desktop editor.
pub const OPERATOR_AUTHOR_ID: &str = "operator";

/// Author id stamped on versions OpenHuman's agent writes — through the
/// artifact tools and through producer tools alike. A tool run carries no
/// agent identity, so every agent-written version is attributed to this one.
pub const AGENT_AUTHOR_ID: &str = "openhuman";

/// Append the user's edit of artifact `id` as a new operator-authored version
/// (the preview panel's autosave). The UI bridge writes it to the files folder
/// and updates the card. A body identical to the newest version appends
/// nothing.
pub async fn save_operator_version(
    workspace_dir: &Path,
    files: FileRoots,
    id: &str,
    body: &str,
) -> Result<SavedVersion, String> {
    let saved = artifacts_service(workspace_dir, files, None)
        .revise(
            id,
            Content::Text(body.to_string()),
            &Author::operator(OPERATOR_AUTHOR_ID),
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
    if let Some(notice) = &saved.notice {
        tracing::warn!(target: "tiny-artifacts", artifact_id = %id, "[tiny-artifacts] {notice}");
    }
    Ok(SavedVersion {
        version: saved.version,
        saved: saved.appended,
    })
}

/// Result of [`save_operator_version`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct SavedVersion {
    /// The newest version number after the call.
    pub version: u32,
    /// Whether a version was appended.
    pub saved: bool,
}

#[cfg(test)]
#[path = "tiny_store_tests.rs"]
mod tests;
