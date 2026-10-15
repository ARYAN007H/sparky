use std::path::PathBuf;

use tiny_artifacts::{ArtifactKind, ArtifactStatus as RecordStatus, ArtifactStore};

use super::*;
use crate::agent::artifacts::store::{get_artifact, REGENERATE_TARGET_ID};
use crate::agent::artifacts::tiny_store::versioned_store;
use crate::agent::artifacts::types::{ArtifactKind as CardKind, ArtifactStatus as CardStatus};

fn temp_dirs() -> (PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("production-{}", uuid::Uuid::new_v4()));
    let (workspace, files) = (root.join("workspace"), root.join("Files"));
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::create_dir_all(&files).unwrap();
    (workspace, files)
}

fn files_in(dir: &PathBuf) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

async fn produce(
    workspace: &PathBuf,
    files: &PathBuf,
    bytes: &[u8],
    tool_call_id: Option<&str>,
    revise: Option<&str>,
) -> String {
    let production = Production::start(
        workspace,
        FileRoots::new(files),
        ArtifactKind::File,
        "Quarterly Deck",
        "pptx",
        tool_call_id,
        revise,
    )
    .await
    .unwrap();
    tokio::fs::write(production.path(), bytes).await.unwrap();
    assert_eq!(production.complete().await.unwrap(), bytes.len() as u64);
    production.id().to_string()
}

#[tokio::test]
async fn a_new_artifact_shares_one_id_and_one_file_between_card_and_record() {
    let (workspace, files) = temp_dirs();
    let production = Production::start(
        &workspace,
        FileRoots::new(&files),
        ArtifactKind::File,
        "Quarterly Deck",
        "pptx",
        None,
        None,
    )
    .await
    .unwrap();
    let id = production.id().to_string();

    // While generating: a pending card, and a pending record with no version.
    let card = get_artifact(&workspace, &id).await.unwrap();
    assert_eq!(card.status, CardStatus::Pending);
    assert_eq!(card.kind, CardKind::Presentation);
    let record = versioned_store(&workspace).get(&id).await.unwrap().unwrap();
    assert_eq!(record.status, RecordStatus::Pending);
    assert!(record.versions.is_empty());

    tokio::fs::write(production.path(), b"deck-v1")
        .await
        .unwrap();
    production.complete().await.unwrap();

    let card = get_artifact(&workspace, &id).await.unwrap();
    assert_eq!(card.status, CardStatus::Ready);
    assert_eq!(card.size_bytes, 7);
    let record = versioned_store(&workspace).get(&id).await.unwrap().unwrap();
    assert_eq!(record.status, RecordStatus::Ready);
    assert_eq!(
        record.latest().unwrap().author_id,
        crate::agent::artifacts::tiny_store::AGENT_AUTHOR_ID
    );
    let file = record.latest().unwrap().file.clone().unwrap();
    assert!(file.path.ends_with("quarterly-deck.pptx"), "{}", file.path);
    assert_eq!(
        files_in(&files),
        vec!["quarterly-deck.pptx"],
        "no second copy"
    );
}

#[tokio::test]
async fn a_revision_reuses_the_id_and_file_and_keeps_the_history() {
    let (workspace, files) = temp_dirs();
    let id = produce(&workspace, &files, b"deck-v1", Some("call-1"), None).await;
    let again = produce(&workspace, &files, b"deck-v2!", None, Some(&id)).await;
    assert_eq!(again, id);

    let record = versioned_store(&workspace).get(&id).await.unwrap().unwrap();
    assert_eq!(record.versions.len(), 2);
    let card = get_artifact(&workspace, &id).await.unwrap();
    assert_eq!(card.status, CardStatus::Ready);
    assert_eq!(card.size_bytes, 8);
    // The revision keeps the card tied to the call that made the artifact.
    assert_eq!(card.tool_call_id.as_deref(), Some("call-1"));
    assert_eq!(files_in(&files), vec!["quarterly-deck.pptx"]);
    assert_eq!(
        std::fs::read(files.join("quarterly-deck.pptx")).unwrap(),
        b"deck-v2!"
    );
}

#[tokio::test]
async fn a_regenerate_in_scope_revises_the_target() {
    let (workspace, files) = temp_dirs();
    let id = produce(&workspace, &files, b"deck-v1", None, None).await;
    let again = REGENERATE_TARGET_ID
        .scope(
            id.clone(),
            produce(&workspace, &files, b"deck-v2", None, None),
        )
        .await;
    assert_eq!(again, id);
    let record = versioned_store(&workspace).get(&id).await.unwrap().unwrap();
    assert_eq!(record.versions.len(), 2);
}

#[tokio::test]
async fn a_failed_production_fails_card_and_record() {
    let (workspace, files) = temp_dirs();
    let production = Production::start(
        &workspace,
        FileRoots::new(&files),
        ArtifactKind::Image,
        "Logo",
        "png",
        None,
        None,
    )
    .await
    .unwrap();
    production.fail("provider timed out").await;

    let card = get_artifact(&workspace, production.id()).await.unwrap();
    assert_eq!(card.status, CardStatus::Failed);
    assert_eq!(card.error.as_deref(), Some("provider timed out"));
    let record = versioned_store(&workspace)
        .get(production.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(record.status, RecordStatus::Failed);
    assert_eq!(record.error.as_deref(), Some("provider timed out"));
}
