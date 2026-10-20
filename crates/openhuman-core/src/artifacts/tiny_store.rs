//! OpenHuman's side of the versioned artifacts (`tiny-artifacts`).
//!
//! The crate owns what an artifact is and how it changes; every creation and
//! revision goes through its [`Artifacts`] service. OpenHuman supplies the
//! service's ports:
//!
//! * [`versioned_store`] — the records, in the configured storage backend
//!   ([`super::version_documents::DocumentArtifactStore`]);
//! * the acting tenant's scoped blob store as TinyArtifacts' file store;
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
//! ([`super::store::create_artifact_regenerating`]), so a revision overwrites
//! the file the artifact already owns and its card updates in place. The bridge
//! carries the chat routing of the turn that made the change, so the card
//! reaches that chat live.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use tiny_artifacts::{
    ArtifactEvent, ArtifactEventKind, ArtifactKind, ArtifactObserver, ArtifactRecord,
    ArtifactStore, Artifacts, Author, Content, FileRef, FileStore,
};

use super::files::FileRoots;
use super::types::ArtifactKind as LegacyArtifactKind;
use crate::security::approval::ApprovalChatContext;
use crate::tools::per_call::PerCallTool;

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

/// The versioned records of a workspace, in the configured storage backend
/// under the acting scope (records an earlier build kept as files under
/// [`legacy_versioned_dir`] are imported once).
pub fn versioned_store(workspace_dir: &Path) -> super::version_documents::DocumentArtifactStore {
    super::version_documents::DocumentArtifactStore::new(workspace_dir)
}

/// Imports version records written by earlier OpenHuman builds, whose file
/// references held host paths, through the owner's guarded legacy importer.
struct LegacyFileImportStore {
    records: Arc<dyn ArtifactStore>,
    files: Arc<dyn FileStore>,
    roots: tiny_artifacts::FileRoots,
}

impl LegacyFileImportStore {
    async fn import_record(&self, mut record: ArtifactRecord) -> anyhow::Result<ArtifactRecord> {
        if !record.kind.holds_file() {
            return Ok(record);
        }
        let mut changed = false;
        for version in &mut record.versions {
            let legacy = version.file.clone().unwrap_or_else(|| FileRef {
                key: version.body.clone(),
                size_bytes: 0,
                media_type: None,
            });
            if legacy.key.starts_with("artifacts/files/") {
                continue;
            }
            let ext = Path::new(&legacy.key)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("bin");
            let key = format!("artifacts/files/{}/v{}.{}", record.id, version.version, ext);
            let imported =
                tiny_artifacts::import_legacy_file(self.files.as_ref(), &self.roots, &legacy, &key)
                    .await?;
            version.body = imported.key.clone();
            version.file = Some(imported);
            changed = true;
        }
        if changed {
            self.records.upsert(&record).await?;
        }
        Ok(record)
    }
}

#[async_trait]
impl ArtifactStore for LegacyFileImportStore {
    async fn list(&self) -> anyhow::Result<Vec<ArtifactRecord>> {
        let mut imported = Vec::new();
        for record in self.records.list().await? {
            imported.push(self.import_record(record).await?);
        }
        Ok(imported)
    }

    async fn get(&self, id: &str) -> anyhow::Result<Option<ArtifactRecord>> {
        match self.records.get(id).await? {
            Some(record) => self.import_record(record).await.map(Some),
            None => Ok(None),
        }
    }

    async fn upsert(&self, record: &ArtifactRecord) -> anyhow::Result<()> {
        self.records.upsert(record).await
    }

    async fn delete(&self, id: &str) -> anyhow::Result<bool> {
        self.records.delete(id).await
    }
}

/// Where an earlier build kept versioned records, one `<id>.json` each.
pub fn legacy_versioned_dir(workspace_dir: &Path) -> PathBuf {
    workspace_dir.join("artifacts").join(VERSIONED_SUBDIR)
}

/// Shows versioned artifacts in the desktop UI through the legacy pipeline.
pub struct LegacyUiBridge {
    workspace_dir: PathBuf,
    files: FileRoots,
    file_store: Option<Arc<dyn FileStore>>,
    chat: Option<ApprovalChatContext>,
}

impl LegacyUiBridge {
    /// A bridge writing into `files` (the visible files folder), announcing
    /// cards to `chat` (`None` outside a chat turn).
    pub fn new(
        workspace_dir: impl Into<PathBuf>,
        files: FileRoots,
        file_store: Option<Arc<dyn FileStore>>,
        chat: Option<&ApprovalChatContext>,
    ) -> Self {
        Self {
            workspace_dir: workspace_dir.into(),
            files,
            file_store,
            chat: chat.cloned(),
        }
    }

    /// Put the newest version in the files folder under the artifact's id and
    /// mark it ready: the content for text kinds, a copy of the file for file
    /// kinds (unless the file already is the one the artifact owns).
    pub(crate) async fn show_newest(&self, record: &ArtifactRecord) -> anyhow::Result<()> {
        let (Some(latest), Some((legacy_kind, ext))) = (record.latest(), legacy_shape(record))
        else {
            return Ok(());
        };
        // A file version lands in the file its card already owns: copy it there
        // (a producer has usually written that file itself) and mark the card
        // ready. Re-reserving would flash the card back to "generating".
        if let Some(file) = latest.file.as_ref() {
            if let Ok(meta) = super::store::get_artifact(&self.workspace_dir, &record.id).await {
                let owned = meta.file.as_deref().map(Path::new).filter(|owned| {
                    owned
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(str::to_ascii_lowercase)
                        == Some(ext.clone())
                });
                if let Some(owned) = owned {
                    let file_store = self
                        .file_store
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("artifact file storage is unavailable"))?;
                    let bytes = file_store.read(&file.key).await?;
                    tokio::fs::write(owned, &bytes).await?;
                    let size = bytes.len() as u64;
                    super::store::finalize_artifact(
                        &self.workspace_dir,
                        &record.id,
                        size,
                        self.chat.as_ref(),
                    )
                    .await
                    .map_err(anyhow::Error::msg)?;
                    return Ok(());
                }
            }
        }
        let (meta, dest) = super::store::create_artifact_regenerating(
            &self.workspace_dir,
            self.files.clone(),
            legacy_kind,
            &record.title,
            &ext,
            None,
            Some(&record.id),
            self.chat.as_ref(),
        )
        .await
        .map_err(anyhow::Error::msg)?;
        let size = if record.holds_file() {
            if let Some(file) = latest.file.as_ref() {
                let file_store = self
                    .file_store
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("artifact file storage is unavailable"))?;
                let bytes = file_store.read(&file.key).await?;
                tokio::fs::write(&dest, &bytes).await?;
                bytes.len() as u64
            } else {
                let source = PathBuf::from(&latest.body);
                if same_file(&source, &dest) {
                    tokio::fs::metadata(&dest).await?.len()
                } else {
                    tokio::fs::copy(&source, &dest).await?
                }
            }
        } else {
            tokio::fs::write(&dest, latest.body.as_bytes()).await?;
            latest.body.len() as u64
        };
        super::store::finalize_artifact(&self.workspace_dir, &meta.id, size, self.chat.as_ref())
            .await
            .map_err(anyhow::Error::msg)?;
        Ok(())
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
                        self.chat.as_ref(),
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

/// The folders a file version may point into: its own kept copy, the agent's
/// working folder, and every files folder the host trusts. A folder used
/// before the current one stays trusted, so an artifact made there can still
/// be revised in place.
pub fn file_roots(
    workspace_dir: &Path,
    files: &FileRoots,
    action_dir: Option<&Path>,
) -> Vec<PathBuf> {
    // A relative path resolves against the first root: the agent's working
    // folder when there is one, else the current files folder. A working
    // folder that contains OpenHuman's workspace (the home directory, say) is
    // left out: it would make OpenHuman's own data referenceable.
    let mut roots: Vec<PathBuf> = action_dir
        .filter(|action| !contains(action, workspace_dir))
        .map(Path::to_path_buf)
        .into_iter()
        .collect();
    for folder in files.trusted() {
        if !roots.contains(folder) {
            roots.push(folder.clone());
        }
    }
    roots.push(version_files_dir(workspace_dir));
    roots
}

/// Whether `folder` is `inside` or one of its parents, compared canonically
/// where the paths exist.
fn contains(folder: &Path, inside: &Path) -> bool {
    let canon = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    canon(inside).starts_with(canon(folder))
}

/// Why a resolved path may not be an artifact's file, if it may not: it is in
/// a location OpenHuman never lets the agent reach (credential stores, system
/// roots), however broad the folders around it.
///
/// The guard sees canonical paths, and on macOS `/etc` canonicalizes to
/// `/private/etc`: a `/private/...` path is also checked without that prefix,
/// as the protected prefixes name the familiar form.
pub fn refused_path(path: &Path) -> Option<String> {
    let forbidden = |p: &Path| crate::security::SecurityPolicy::is_always_forbidden(p);
    let aliased = path
        .strip_prefix("/private")
        .ok()
        .map(|rest| Path::new("/").join(rest));
    (forbidden(path) || aliased.as_deref().is_some_and(forbidden))
        .then(|| "the file is in a protected location".to_string())
}

/// [`file_roots`] as the crate's folders, with [`refused_path`] as their guard.
pub fn guarded_roots(roots: Vec<PathBuf>) -> tiny_artifacts::FileRoots {
    tiny_artifacts::FileRoots::new(roots).with_guard(refused_path)
}

/// The artifact service for one workspace: OpenHuman's store and UI bridge,
/// with file artifacts allowed from [`file_roots`]. Cards are announced to
/// `chat`, the turn making the change (`None` outside a chat turn).
pub fn artifacts_service(
    workspace_dir: &Path,
    files: FileRoots,
    action_dir: Option<&Path>,
    chat: Option<&ApprovalChatContext>,
    ctx: &crate::core::runtime::DispatchCtx,
) -> Artifacts {
    build_artifacts_service(workspace_dir, files, action_dir, chat, ctx, None).0
}

/// Build an artifact service and bind an optional publisher to the exact
/// tenant-scoped file store used by that service.
pub fn artifacts_service_with_publisher(
    workspace_dir: &Path,
    files: FileRoots,
    action_dir: Option<&Path>,
    chat: Option<&ApprovalChatContext>,
    ctx: &crate::core::runtime::DispatchCtx,
    publisher: Option<&super::publisher_slot::ArtifactPublisherBinder>,
) -> Artifacts {
    build_artifacts_service(workspace_dir, files, action_dir, chat, ctx, publisher).0
}

pub(crate) fn artifacts_service_with_file_store(
    workspace_dir: &Path,
    files: FileRoots,
    action_dir: Option<&Path>,
    chat: Option<&ApprovalChatContext>,
    ctx: &crate::core::runtime::DispatchCtx,
) -> (Artifacts, Option<Arc<dyn FileStore>>) {
    build_artifacts_service(workspace_dir, files, action_dir, chat, ctx, None)
}

fn build_artifacts_service(
    workspace_dir: &Path,
    files: FileRoots,
    action_dir: Option<&Path>,
    chat: Option<&ApprovalChatContext>,
    ctx: &crate::core::runtime::DispatchCtx,
    publisher: Option<&super::publisher_slot::ArtifactPublisherBinder>,
) -> (Artifacts, Option<Arc<dyn FileStore>>) {
    let roots = file_roots(workspace_dir, &files, action_dir);
    let scoped = crate::storage::sessions::artifact_scoped_for_in(workspace_dir, ctx);
    let file_store = scoped
        .as_ref()
        .map(|scoped| Arc::new(tiny_artifacts::BlobFileStore::new(scoped)) as Arc<dyn FileStore>);
    let mut records: Arc<dyn ArtifactStore> = Arc::new(match scoped.as_ref() {
        Some(scoped) => {
            super::version_documents::DocumentArtifactStore::over(workspace_dir, scoped)
        }
        None => versioned_store(workspace_dir),
    });
    if let Some(store) = file_store.as_ref() {
        records = Arc::new(LegacyFileImportStore {
            records,
            files: store.clone(),
            roots: guarded_roots(roots.clone()),
        });
    }
    let mut artifacts = Artifacts::new(records)
        .with_observer(Arc::new(LegacyUiBridge::new(
            workspace_dir,
            files,
            file_store.clone(),
            chat,
        )))
        .with_file_roots(guarded_roots(roots));
    if let Some(file_store) = file_store.as_ref() {
        if let Some(publisher) = publisher {
            artifacts = artifacts.with_publisher(publisher(file_store.clone()));
        }
        artifacts = artifacts.with_file_store(file_store.clone());
    }
    (artifacts, file_store)
}

/// Legacy workspace folder that earlier builds used for per-version file
/// copies. New file bytes live in the acting tenant's scoped blob store.
pub fn version_files_dir(workspace_dir: &Path) -> PathBuf {
    workspace_dir
        .join("artifacts")
        .join(VERSIONED_SUBDIR)
        .join("files")
}

/// Build the artifact tools bound to this workspace's service: the crate's
/// default set (`create_artifact`, `read_artifact`, `update_artifact`,
/// `list_artifacts`), `delete_artifact` (default-off through its
/// `tools::user_filter` family), and — only when a `publisher` exists, i.e. the
/// user can use cloud file storage — `share_artifact`.
///
/// The crate's tools run without a run context, so each call builds its tool
/// around a service whose UI bridge carries that call's chat routing
/// ([`PerCallTool`]); the card then reaches the chat the agent is answering.
pub fn registered_tools(
    workspace_dir: &Path,
    files: FileRoots,
    action_dir: &Path,
    publisher: Option<super::publisher_slot::ArtifactPublisherBinder>,
) -> Vec<Box<dyn tinytools::Tool>> {
    let can_publish = publisher.is_some();
    let service = {
        let workspace_dir = workspace_dir.to_path_buf();
        let action_dir = action_dir.to_path_buf();
        Arc::new(
            move |chat: Option<&ApprovalChatContext>, ctx: &crate::core::runtime::DispatchCtx| {
                Arc::new(artifacts_service_with_publisher(
                    &workspace_dir,
                    files.clone(),
                    Some(&action_dir),
                    chat,
                    ctx,
                    publisher.as_ref(),
                ))
            },
        )
    };
    let author = Author::agent(AGENT_AUTHOR_ID);
    let mut tools: Vec<Box<dyn tinytools::Tool>> = Vec::new();
    {
        let (service, author) = (service.clone(), author.clone());
        tools.push(Box::new(PerCallTool::new(move |chat, ctx| {
            tiny_artifacts::CreateArtifactTool::new(service(chat, ctx), author.clone())
        })));
    }
    {
        let service = service.clone();
        tools.push(Box::new(PerCallTool::new(move |chat, ctx| {
            tiny_artifacts::ReadArtifactTool::new(service(chat, ctx))
        })));
    }
    {
        let (service, author) = (service.clone(), author.clone());
        tools.push(Box::new(PerCallTool::new(move |chat, ctx| {
            tiny_artifacts::UpdateArtifactTool::new(service(chat, ctx), author.clone())
        })));
    }
    {
        let service = service.clone();
        tools.push(Box::new(PerCallTool::new(move |chat, ctx| {
            tiny_artifacts::ListArtifactsTool::new(service(chat, ctx))
        })));
    }
    {
        let service = service.clone();
        tools.push(Box::new(PerCallTool::new(move |chat, ctx| {
            tiny_artifacts::DeleteArtifactTool::new(service(chat, ctx))
        })));
    }
    if can_publish {
        tools.push(Box::new(PerCallTool::new(move |chat, ctx| {
            tiny_artifacts::ShareArtifactTool::new(service(chat, ctx))
        })));
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
    ctx: &crate::core::runtime::DispatchCtx,
) -> Result<SavedVersion, String> {
    let saved = artifacts_service(workspace_dir, files, None, None, ctx)
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
