use std::path::PathBuf;

use serde_json::json;
use tinystoragedrivers::{
    Checkpoint, CollectionSpec, DocumentStoreExt, Precondition, Scope, StorageConfig,
    StorageBackend,
};

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

    // These are the collection names and v1 document bodies written by the
    // pre-port profile owners. The old TSD public writer supplies the actual
    // scoped SQLite storage layout and metadata.
    let profile = backend.for_scope(&Scope::new("profile:u1")?)?;
    let docs = profile.documents();
    for (collection, id, body) in [
        (
            "config",
            "runtime",
            json!({"workspace": "/legacy/workspace", "theme": "dark", "schema_version": 1}),
        ),
        (
            "cost_records",
            "cost-0001",
            json!({"usage": {"timestamp": "2025-02-03T04:05:06.789+02:00", "input_tokens": 12, "output_tokens": 3}, "model": "fixture-model", "ts_ms": 1738548306789}),
        ),
        (
            "artifacts",
            "artifact-1",
            json!({"id": "artifact-1", "title": "Fixture plan", "created_at": "2025-02-03T04:05:06Z", "thread_id": "thread-1"}),
        ),
    ] {
        docs.ensure_collection(&CollectionSpec::new(collection))
            .await?;
        docs.put(collection, id, body, Precondition::Absent).await?;
    }

    let cluster = backend.for_scope(&Scope::new("cluster")?)?;
    let docs = cluster.documents();
    docs.ensure_collection(&CollectionSpec::new("profiles"))
        .await?;
    docs.put(
        "profiles",
        "u1",
        json!({"profile_id": "u1", "created_at": 1738548306_i64, "layout_version": 1}),
        Precondition::Absent,
    )
    .await?;
    docs.ensure_collection(&CollectionSpec::new("leases"))
        .await?;
    docs.put(
        "leases",
        "profile:u1",
        json!({"owner": "desktop-legacy", "epoch": 7, "expires_at_ms": 1738548366789_i64, "endpoint": "local"}),
        Precondition::Absent,
    )
    .await?;

    // One old module-scope source row demonstrates that the inventory reader
    // preserves identities needed by the host's explicit known-module map.
    let module = backend.for_scope(&Scope::new("module:tinybox@local")?)?;
    let docs = module.documents();
    docs.ensure_collection(&CollectionSpec::new("tinybox_jail_index"))
        .await?;
    docs.put(
        "tinybox_jail_index",
        "jail-1",
        json!({"id": "jail-1", "created_at": "2025-02-03T04:05:06Z", "image": "fixture"}),
        Precondition::Absent,
    )
    .await?;

    backend.checkpoint(Checkpoint::Truncate).await?;
    println!("written {}", path.display());
    Ok(())
}
