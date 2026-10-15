//! [`Production`]: how a producer tool (a deck, a document, a generated image
//! or video) files what it makes as a versioned artifact.
//!
//! Every artifact goes through the `tiny-artifacts` service; a producer only
//! adds the part that is OpenHuman's — reserving a file name in the visible
//! files folder, so it can write its bytes there while the user already sees
//! a "generating…" card. The flow:
//!
//! 1. [`Production::start`] reserves the file and the card, and opens the
//!    versioned record under the same id. A revision (`revise`, or a
//!    regenerate running inside [`super::store::REGENERATE_TARGET_ID`]) reuses
//!    the existing id and file, and keeps the record's history.
//! 2. The producer writes its bytes to [`Production::path`].
//! 3. [`Production::complete`] appends the file as a new agent version; the UI
//!    bridge marks the card ready. [`Production::fail`] fails both instead.
//!
//! The record is written before the card is marked ready, so a ready card
//! always has its versioned record.

use std::path::{Path, PathBuf};

use tiny_artifacts::{ArtifactKind, Artifacts, Author, Content};

use super::files::FileRoots;
use super::tiny_store::{artifacts_service, legacy_kind_for_extension};
use super::types::ArtifactKind as LegacyArtifactKind;

/// A file being produced as an artifact.
pub struct Production {
    artifacts: Artifacts,
    workspace_dir: PathBuf,
    files: FileRoots,
    id: String,
    path: PathBuf,
}

impl std::fmt::Debug for Production {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Production")
            .field("id", &self.id)
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl Production {
    /// Reserve the file (`<title>.<extension>` in the files folder) and open
    /// the artifact. `revise` names an existing artifact to add a version to;
    /// otherwise a regenerate in scope is revised, or a new artifact is made.
    /// `tool_call_id` ties the card to the producing call's bubble.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be reserved or the record cannot
    /// be opened; the card is failed in the second case.
    pub async fn start(
        workspace_dir: &Path,
        files: FileRoots,
        kind: ArtifactKind,
        title: &str,
        extension: &str,
        tool_call_id: Option<&str>,
        revise: Option<&str>,
    ) -> Result<Self, String> {
        let legacy_kind = if kind == ArtifactKind::Image {
            LegacyArtifactKind::Image
        } else {
            legacy_kind_for_extension(&extension.to_ascii_lowercase())
        };
        let target = revise
            .map(str::to_string)
            .or_else(|| {
                super::store::REGENERATE_TARGET_ID
                    .try_with(Clone::clone)
                    .ok()
            })
            .filter(|id| !id.trim().is_empty());
        let reserve = super::store::create_artifact_for_call(
            workspace_dir,
            files.clone(),
            legacy_kind,
            title,
            extension,
            tool_call_id,
        );
        let (meta, path) = match &target {
            Some(id) => {
                super::store::REGENERATE_TARGET_ID
                    .scope(id.clone(), reserve)
                    .await?
            }
            None => reserve.await?,
        };

        let artifacts = artifacts_service(workspace_dir, files.clone(), None);
        let production = Self {
            artifacts,
            workspace_dir: workspace_dir.to_path_buf(),
            files,
            id: meta.id.clone(),
            path,
        };
        // A revision keeps its record and history; a new artifact opens one.
        let existing = production
            .artifacts
            .get(&meta.id)
            .await
            .map_err(|e| e.to_string());
        let opened = match existing {
            Ok(Some(_)) => Ok(()),
            Ok(None) => production
                .artifacts
                .begin(kind, title, Some(meta.id.clone()))
                .await
                .map(|_| ())
                .map_err(|e| e.to_string()),
            Err(err) => Err(err),
        };
        if let Err(err) = opened {
            let reason = format!("could not open the artifact record: {err}");
            production.fail_card(&reason).await;
            return Err(reason);
        }
        Ok(production)
    }

    /// The artifact's id.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Where the producer writes its bytes.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// File the written bytes as a new version and mark the card ready.
    /// Returns the file's size.
    ///
    /// # Errors
    ///
    /// Returns an error when the version cannot be recorded or the card cannot
    /// be marked ready; the card is failed then.
    pub async fn complete(&self) -> Result<u64, String> {
        let path = self.path.display().to_string();
        let saved = self
            .artifacts
            .revise(
                &self.id,
                Content::File(path),
                &Author::agent(super::tiny_store::AGENT_AUTHOR_ID),
                None,
            )
            .await;
        let reason = match saved {
            Ok(saved) => match (
                saved.notice,
                saved.record.latest().and_then(|v| v.file.clone()),
            ) {
                (None, Some(file)) => return Ok(file.size_bytes),
                (Some(notice), _) => notice,
                (None, None) => "the recorded version has no file".to_string(),
            },
            Err(err) => err.to_string(),
        };
        let reason = format!("failed to finalize artifact: {reason}");
        self.fail_card(&reason).await;
        Err(reason)
    }

    /// Fail the artifact with `reason`.
    pub async fn fail(&self, reason: &str) {
        if let Err(err) = self.artifacts.fail(&self.id, reason).await {
            tracing::warn!(
                target: "tiny-artifacts",
                artifact_id = %self.id,
                err = %err,
                "[artifacts] could not mark the versioned record failed"
            );
            self.fail_card(reason).await;
        }
    }

    /// Fail the card directly (when the record could not take the change).
    async fn fail_card(&self, reason: &str) {
        let _ =
            super::store::fail_artifact(&self.workspace_dir, self.files.clone(), &self.id, reason)
                .await;
    }
}

#[cfg(test)]
#[path = "production_tests.rs"]
mod tests;
