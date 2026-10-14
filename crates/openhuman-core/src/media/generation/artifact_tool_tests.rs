use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tinyagents_harness::media::MediaOutput;
use tinystoragedrivers::BlobStore;
use tinytools::{PermissionLevel, Tool, ToolCategory, ToolContent, ToolResult};

use super::MediaArtifactTool;
use crate::agent::artifacts::{ArtifactKind, ArtifactStatus};

/// A minimal stand-in for `GenerateImageTool` / `GenerateVideoTool`: writes
/// a fixed number of files under `<action_dir>/generated-media/` and reports
/// them in the same `artifacts` array shape the real vendor tools use.
struct StubMediaTool {
    file_count: usize,
    fail: bool,
}

#[async_trait]
impl Tool for StubMediaTool {
    fn name(&self) -> &str {
        "stub_generate"
    }

    fn description(&self) -> &str {
        "stub"
    }

    fn parameters_schema(&self) -> Value {
        json!({ "type": "object" })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        if self.fail {
            return Ok(ToolResult::error("billed and failed"));
        }
        let dir = args
            .get("__dir")
            .and_then(Value::as_str)
            .map(std::path::PathBuf::from)
            .expect("stub requires __dir");
        std::fs::create_dir_all(&dir).unwrap();
        let mut artifacts = Vec::new();
        for i in 0..self.file_count {
            let path = dir.join(format!("stub-{i}.png"));
            std::fs::write(&path, format!("bytes-{i}")).unwrap();
            artifacts.push(json!({
                "type": "image",
                "path": path.display().to_string(),
                "media_type": "image/png",
                "bytes": 8,
            }));
        }
        Ok(ToolResult::success_with_markdown(
            json!({ "model": "stub/model", "artifacts": artifacts }),
            "generated stub media",
        ))
    }
}

/// TinyAgents' production media output is a blob key, not a filesystem path.
struct BlobBackedMediaTool {
    blobs: Arc<dyn BlobStore>,
}

#[async_trait]
impl Tool for BlobBackedMediaTool {
    fn name(&self) -> &str {
        "blob_generate"
    }

    fn description(&self) -> &str {
        "blob-backed media stub"
    }

    fn parameters_schema(&self) -> Value {
        json!({ "type": "object" })
    }

    async fn execute(&self, _args: Value) -> anyhow::Result<ToolResult> {
        let key = "generated-media/stub-0.png";
        self.blobs
            .put(key, b"blob-media-bytes".to_vec(), Some("image/png"))
            .await?;
        Ok(ToolResult::success_with_markdown(
            json!({
                "artifacts": [{
                    "type": "image",
                    "path": key,
                    "media_type": "image/png",
                    "bytes": 16
                }]
            }),
            "generated blob-backed media",
        ))
    }
}

fn json_payload(result: &ToolResult) -> Value {
    result
        .content
        .iter()
        .find_map(|block| match block {
            ToolContent::Json { data } => Some(data.clone()),
            _ => None,
        })
        .expect("json content block")
}

#[tokio::test]
async fn files_a_blob_backed_generated_artifact() {
    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("workspace");
    let files_dir = root.path().join("Files");
    let blobs = MediaOutput::new(root.path()).blobs.clone();
    let wrapped = MediaArtifactTool::new(
        BlobBackedMediaTool {
            blobs: Arc::clone(&blobs),
        },
        ArtifactKind::Image,
        workspace.clone(),
        files_dir.clone(),
    )
    .with_blobs(Arc::clone(&blobs));

    let result = wrapped.execute(json!({})).await.unwrap();
    assert!(!result.is_error, "{result:?}");
    let payload = json_payload(&result);
    let artifact = &payload["artifacts"][0];
    let artifact_id = artifact["artifact_id"]
        .as_str()
        .unwrap_or_else(|| panic!("blob-backed media was not filed: {artifact:?}"));
    let record = crate::agent::artifacts::store::get_artifact(&workspace, artifact_id)
        .await
        .expect("artifact record");
    let file = record.file.expect("visible file");
    assert_eq!(std::fs::read(file).unwrap(), b"blob-media-bytes");
    assert!(blobs
        .get("generated-media/stub-0.png")
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn files_a_single_generated_artifact() {
    let root = tempfile::tempdir().unwrap();
    let staging = root.path().join("staging");
    let workspace = root.path().join("workspace");

    let wrapped = MediaArtifactTool::new(
        StubMediaTool {
            file_count: 1,
            fail: false,
        },
        ArtifactKind::Image,
        workspace.clone(),
        root.path().join("Files"),
    );

    let result = wrapped
        .execute(json!({ "prompt": "a cat", "__dir": staging.display().to_string() }))
        .await
        .unwrap();
    assert!(!result.is_error, "{result:?}");

    let payload = json_payload(&result);
    let artifacts = payload["artifacts"].as_array().unwrap();
    assert_eq!(artifacts.len(), 1);
    let artifact_id = artifacts[0]["artifact_id"].as_str().unwrap();
    assert!(artifacts[0].get("artifact_error").is_none());

    // Original staged file is gone (moved).
    assert!(!staging.join("stub-0.png").exists());
    // The artifact record lives in the document store (no `meta.json`); resolve
    // it through the store and verify its user-facing Files-folder path.
    let meta = crate::agent::artifacts::store::get_artifact(&workspace, artifact_id)
        .await
        .expect("artifact metadata is persisted");
    assert_eq!(meta.kind, ArtifactKind::Image);
    assert_eq!(meta.status, ArtifactStatus::Ready);
    let file = meta.file.expect("artifact points at its visible file");
    assert!(std::path::Path::new(&file).exists(), "{file}");
    assert_eq!(std::fs::read(file).unwrap(), b"bytes-0");
}

#[tokio::test]
async fn files_every_artifact_when_n_greater_than_one() {
    let root = tempfile::tempdir().unwrap();
    let staging = root.path().join("staging");
    let workspace = root.path().join("workspace");

    let wrapped = MediaArtifactTool::new(
        StubMediaTool {
            file_count: 3,
            fail: false,
        },
        ArtifactKind::Image,
        workspace.clone(),
        root.path().join("Files"),
    );

    let result = wrapped
        .execute(json!({ "prompt": "three cats", "__dir": staging.display().to_string() }))
        .await
        .unwrap();
    let payload = json_payload(&result);
    let artifacts = payload["artifacts"].as_array().unwrap();
    assert_eq!(artifacts.len(), 3);
    let mut ids = std::collections::HashSet::new();
    for entry in artifacts {
        let id = entry["artifact_id"].as_str().expect("artifact_id set");
        assert!(ids.insert(id.to_string()), "artifact ids must be unique");
        let meta = crate::agent::artifacts::store::get_artifact(&workspace, id)
            .await
            .expect("artifact metadata is persisted");
        assert_eq!(meta.status, ArtifactStatus::Ready);
        assert!(meta
            .file
            .as_deref()
            .is_some_and(|file| std::path::Path::new(file).exists()));
    }
}

#[tokio::test]
async fn leaves_an_errored_tool_result_untouched() {
    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("workspace");

    let wrapped = MediaArtifactTool::new(
        StubMediaTool {
            file_count: 1,
            fail: true,
        },
        ArtifactKind::Image,
        workspace.clone(),
        root.path().join("Files"),
    );

    let result = wrapped.execute(json!({ "prompt": "x" })).await.unwrap();
    assert!(result.is_error);
    // No artifacts root should even be created.
    assert!(!workspace.join("artifacts").exists());
}

#[tokio::test]
async fn host_metadata_forwards_to_the_inner_tool() {
    let workspace_dir = tempfile::tempdir().unwrap();
    let workspace = workspace_dir.path().to_path_buf();
    let wrapped = MediaArtifactTool::new(
        StubMediaTool {
            file_count: 0,
            fail: false,
        },
        ArtifactKind::Video,
        workspace.clone(),
        workspace.join("Files"),
    );
    assert_eq!(wrapped.name(), "stub_generate");
    assert_eq!(wrapped.permission_level(), PermissionLevel::ReadOnly);
    assert_eq!(wrapped.category(), ToolCategory::System);
}

/// A video-like tool whose poll observer is the one its call was built with:
/// it polls a job state, then waits like a long job.
struct PollingTool {
    observer: tinyagents_harness::tinyinference_video::ProgressFn,
    state: tinyagents_harness::tinyinference_video::JobState,
}

#[async_trait]
impl Tool for PollingTool {
    fn name(&self) -> &str {
        "polling_generate"
    }
    fn description(&self) -> &str {
        "polling"
    }
    fn parameters_schema(&self) -> Value {
        json!({ "type": "object" })
    }
    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        self.execute_with_context(args, tinytools::ToolCallOptions::default(), None)
            .await
    }
    async fn execute_with_context(
        &self,
        _args: Value,
        _options: tinytools::ToolCallOptions,
        _context: Option<&dyn tinytools::ToolRunContext>,
    ) -> anyhow::Result<ToolResult> {
        (self.observer)(&tinyagents_harness::tinyinference_video::VideoJobStatus {
            id: "job".into(),
            state: self.state.clone(),
            outputs: 0,
            output_indices: Vec::new(),
            cost_usd: None,
            error: None,
        });
        tokio::time::sleep(std::time::Duration::from_secs(20)).await;
        Ok(ToolResult::error("no files"))
    }
}

#[derive(Default)]
struct Recorder(std::sync::Mutex<Vec<String>>);

impl tinytools::ToolRunContext for Recorder {
    fn report_progress(&self, update: tinytools::ToolProgress) {
        self.0
            .lock()
            .unwrap()
            .push(update.message.unwrap_or_default());
    }
}

/// The per-call inner tool binds the poll observer to the call it serves, so
/// two concurrent video calls each report their own job state.
#[tokio::test(start_paused = true)]
async fn per_call_observer_reports_each_calls_own_job_state() {
    use tinyagents_harness::tinyinference_video::JobState;
    let root = tempfile::tempdir().unwrap();
    let states = std::sync::Mutex::new(vec![JobState::InProgress, JobState::Pending]);
    let wrapped = MediaArtifactTool::new(
        PollingTool {
            observer: std::sync::Arc::new(|_| {}),
            state: JobState::Pending,
        },
        ArtifactKind::Video,
        root.path().join("workspace"),
        root.path().join("Files"),
    )
    .with_per_call_inner(move |observer, _chat| PollingTool {
        observer,
        state: states.lock().unwrap().pop().unwrap(),
    });
    let (a, b) = (Recorder::default(), Recorder::default());
    let (ra, rb) = tokio::join!(
        wrapped.execute_with_context(json!({}), tinytools::ToolCallOptions::default(), Some(&a)),
        wrapped.execute_with_context(json!({}), tinytools::ToolCallOptions::default(), Some(&b)),
    );
    ra.unwrap();
    rb.unwrap();
    let (ma, mb) = (a.0.lock().unwrap().clone(), b.0.lock().unwrap().clone());
    assert_eq!(ma, vec!["Generating video — 15s elapsed (job pending)"]);
    assert_eq!(mb, vec!["Generating video — 15s elapsed (job in_progress)"]);
}
