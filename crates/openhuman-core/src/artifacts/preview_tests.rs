use std::path::Path;

use tiny_artifacts::{
    ArtifactAuthor, ArtifactKind, ArtifactRecord, ArtifactVersion, FileRef, FileStore,
};

use super::*;

fn file_record(key: &str, size_bytes: u64) -> ArtifactRecord {
    let mut version = ArtifactVersion::new(1, key, ArtifactAuthor::Agent, "agent", 1);
    version.file = Some(FileRef {
        key: key.to_string(),
        size_bytes,
        media_type: None,
    });
    let mut record = ArtifactRecord::pending("f-1", "Report", ArtifactKind::File, 1);
    record.versions.push(version);
    record
}

fn file_store(dir: &Path) -> std::sync::Arc<dyn FileStore> {
    std::sync::Arc::new(tiny_artifacts::FsFileStore::new(dir))
}

#[tokio::test]
async fn a_text_kind_shows_its_newest_version() {
    let mut record =
        ArtifactRecord::new("m-1", "Notes", ArtifactKind::Markdown, "# v1", "agent", 1);
    record.push_version(ArtifactVersion::new(
        0,
        "# v2",
        ArtifactAuthor::Operator,
        "operator",
        2,
    ));
    let dir = tempfile::tempdir().unwrap();
    let body = body_of(&record, None, None, &DocumentConverter::none()).await;
    assert_eq!(body.kind, "markdown");
    assert_eq!(body.version, 2);
    assert_eq!(body.body, "# v2");
    assert!(body.is_text && !body.converted);
}

#[tokio::test]
async fn a_text_file_is_read_from_the_scoped_file_store() {
    let dir = tempfile::tempdir().unwrap();
    let files = file_store(dir.path());
    files
        .put("artifacts/files/f-1/v1.txt", b"plain notes", None)
        .await
        .unwrap();
    let record = file_record("artifacts/files/f-1/v1.txt", 11);
    let body = body_of(&record, Some(&files), None, &DocumentConverter::none()).await;
    assert_eq!(body.kind, "file");
    assert_eq!(body.body, "plain notes");
    assert!(body.is_text);
}

#[tokio::test]
async fn a_blob_file_is_not_read_without_its_store() {
    let elsewhere = tempfile::tempdir().unwrap();
    let files = file_store(elsewhere.path());
    files
        .put("artifacts/files/f-1/v1.txt", b"do not read", None)
        .await
        .unwrap();
    let record = file_record("artifacts/files/f-1/v1.txt", 11);
    let body = body_of(&record, None, None, &DocumentConverter::none()).await;
    // Shown as a file: its path, never its content.
    assert!(!body.is_text);
    assert_eq!(body.body, "v1.txt");
}

#[tokio::test]
async fn a_file_over_the_preview_limit_is_not_read() {
    let dir = tempfile::tempdir().unwrap();
    let files = file_store(dir.path());
    let bytes = vec![b'a'; (PREVIEW_MAX_BYTES + 1) as usize];
    files
        .put("artifacts/files/f-1/v1.txt", &bytes, None)
        .await
        .unwrap();
    let record = file_record("artifacts/files/f-1/v1.txt", bytes.len() as u64);
    let body = body_of(&record, Some(&files), None, &DocumentConverter::none()).await;
    assert!(!body.is_text);
    assert_eq!(body.body, "v1.txt");
}

#[tokio::test]
async fn a_binary_file_with_no_converter_shows_as_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let files = file_store(dir.path());
    files
        .put(
            "artifacts/files/f-1/v1.docx",
            b"PK\x03\x04\x00\x00binary",
            None,
        )
        .await
        .unwrap();
    let record = file_record("artifacts/files/f-1/v1.docx", 10);
    let body = body_of(&record, Some(&files), None, &DocumentConverter::none()).await;
    assert!(!body.is_text && !body.converted);
    assert_eq!(body.body, "v1.docx");
}

fn config_in(workspace: &Path) -> crate::config::Config {
    crate::config::Config {
        workspace_dir: workspace.to_path_buf(),
        action_dir: workspace.join("action"),
        files_dir_override: Some(workspace.join("Files")),
        ..crate::config::Config::default()
    }
}

#[tokio::test]
async fn ids_that_name_another_file_are_refused() {
    let workspace = tempfile::tempdir().unwrap();
    let config = config_in(workspace.path());
    let ctx = crate::core::runtime::DispatchCtx::process_default();
    for id in ["../x", "a/b", "..", ".hidden"] {
        assert!(read_body(&config, id, &ctx).await.is_err(), "{id:?}");
    }
}

#[tokio::test]
async fn an_unknown_artifact_is_not_found() {
    let workspace = tempfile::tempdir().unwrap();
    let ctx = crate::core::runtime::DispatchCtx::process_default();
    let err = read_body(&config_in(workspace.path()), "missing", &ctx)
        .await
        .unwrap_err();
    assert!(err.contains("not found"), "{err}");
}

#[tokio::test]
async fn an_artifact_made_before_versioning_opens_from_its_legacy_record() {
    use crate::artifacts::store::{create_artifact, finalize_artifact};
    use crate::artifacts::types::ArtifactKind as LegacyKind;

    let workspace = tempfile::tempdir().unwrap();
    let config = config_in(workspace.path());
    let files = config.files_dir();
    std::fs::create_dir_all(&files).unwrap();
    // Only the legacy record: no versioned one.
    let (meta, path) = create_artifact(
        workspace.path(),
        files.clone(),
        LegacyKind::Document,
        "Old notes",
        "md",
        None,
    )
    .await
    .unwrap();
    std::fs::write(&path, "# from before").unwrap();
    finalize_artifact(workspace.path(), &meta.id, 13, None)
        .await
        .unwrap();

    let ctx = crate::core::runtime::DispatchCtx::process_default();
    let body = read_body(&config, &meta.id, &ctx).await.unwrap();
    assert_eq!(body.title, "Old notes");
    assert_eq!(body.kind, "file");
    assert_eq!(body.body, "# from before");
    assert!(body.is_text);
}
