use std::path::PathBuf;

use chrono::{DateTime, Utc};
use openhuman::artifacts::types::{ArtifactKind, ArtifactMeta, ArtifactStatus};
use openhuman::platform::cost::types::{CostRecord, CostSource, TokenUsage, UsageScope};
use openhuman::profiles::{ProfileId, types::ProfileMeta};
use serde_json::{Value, json};
use tinystoragedrivers::{CollectionSpec, Precondition, Scope, StorageConfig, WriteOptions};

const PROFILE_ID: &str = "u1";
const CREATED_AT_MS: i64 = 1_738_548_306_000;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: legacy-v1-profile-writer <output-storage.db>")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let url = format!("sqlite:{}", path.display());
    let backend = tinystoragedrivers::open(&StorageConfig::parse(&url)?).await?;

    // These transformations are the pre-schema-v2 owner writers from
    // OpenHuman commit 2d3e22636e^: cost records retained their typed body and
    // timestamp, artifact documents stored created_ms, args were wrapped,
    // and the profile registry kept created_at in document metadata. The
    // actual disk layout and writes are supplied by the historical public
    // TSD v1 API pinned in this package.
    let profile_scope = backend.for_scope(&Scope::new("profile:u1")?)?;
    let docs = profile_scope.documents();

    let cost = CostRecord {
        id: "cost-0001".into(),
        session_id: "session-fixture".into(),
        usage: TokenUsage {
            model: "fixture-model".into(),
            input_tokens: 12,
            output_tokens: 3,
            total_tokens: 15,
            cached_input_tokens: 0,
            cache_creation_tokens: 0,
            reasoning_tokens: 0,
            cost_usd: 0.000_015,
            cost_source: CostSource::Estimated,
            run_id: None,
            root_run_id: None,
            scope: UsageScope::default(),
            timestamp: DateTime::parse_from_rfc3339("2025-02-03T04:05:06.789+02:00")?
                .with_timezone(&Utc),
        },
    };
    let mut cost_body = serde_json::to_value(cost)?;
    cost_body["ts_ms"] = 1_738_548_306_789_i64.into();
    put(docs, "cost_records", "cost-0001", cost_body).await?;

    let artifact = ArtifactMeta {
        id: "artifact-1".into(),
        kind: ArtifactKind::Document,
        title: "Fixture plan".into(),
        path: "artifact-1/plan.md".into(),
        file: None,
        file_root: None,
        size_bytes: 4,
        status: ArtifactStatus::Ready,
        created_at: DateTime::parse_from_rfc3339("2025-02-03T04:05:06Z")?.with_timezone(&Utc),
        error: None,
        thread_id: Some("thread-1".into()),
        tool_call_id: None,
    };
    let mut artifact_body = serde_json::to_value(artifact)?;
    let created_at = artifact_body
        .as_object_mut()
        .and_then(|fields| fields.remove("created_at"))
        .ok_or("ArtifactMeta serialization omitted created_at")?;
    artifact_body["created_ms"] = created_at
        .as_str()
        .and_then(|text| DateTime::parse_from_rfc3339(text).ok())
        .map(|at| at.timestamp_millis())
        .ok_or("ArtifactMeta created_at was not RFC3339")?
        .into();
    put(docs, "artifacts", "artifact-1", artifact_body).await?;
    put(
        docs,
        "artifact_args",
        "artifact-1",
        json!({"args": {"title": "Fixture plan", "format": "markdown"}}),
    )
    .await?;

    let cluster = backend.for_scope(&Scope::new("cluster")?)?;
    let profile = ProfileMeta {
        profile_id: ProfileId::parse(PROFILE_ID)?,
        created_at: u64::try_from(CREATED_AT_MS / 1000)?,
        layout_version: 1,
    };
    let mut profile_body = serde_json::to_value(profile)?;
    profile_body
        .as_object_mut()
        .ok_or("ProfileMeta did not serialize as an object")?
        .remove("created_at");
    let docs = cluster.documents();
    docs.ensure_collection(&CollectionSpec::new("profiles")).await?;
    docs.put_with(
        "profiles",
        PROFILE_ID,
        profile_body,
        Precondition::Absent,
        WriteOptions::new().created_ms(CREATED_AT_MS),
    )
    .await?;

    backend.checkpoint(tinystoragedrivers::Checkpoint::Truncate).await?;
    println!("written {}", path.display());
    Ok(())
}

async fn put(
    docs: &dyn tinystoragedrivers::DocumentStore,
    collection: &str,
    id: &str,
    body: Value,
) -> Result<(), tinystoragedrivers::StorageError> {
    docs.ensure_collection(&CollectionSpec::new(collection)).await?;
    docs.put(collection, id, body, Precondition::Absent).await?;
    Ok(())
}
