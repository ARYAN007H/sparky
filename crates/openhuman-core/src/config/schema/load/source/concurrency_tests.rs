use super::*;
use crate::core::runtime::{context::CoreContext, DispatchCtx, DomainSet};
use serde_json::json;

#[tokio::test]
async fn agent_registry_rpc_preserves_document_conflicts() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = MemoryStorage::new();
    let _forced = ForcedDocumentSource::new(
        scoped(&storage, "tenant-rpc"),
        Scope::new("tenant-rpc").unwrap(),
        keys(9),
    );
    let seed = config_at(tmp.path(), "initial");
    seed.save().await.unwrap();
    let stale = Config::load_from_config_path(&seed.config_path, &seed.workspace_dir)
        .await
        .unwrap();
    let mut winner = Config::load_from_config_path(&seed.config_path, &seed.workspace_dir)
        .await
        .unwrap();
    winner.default_model = Some("winner".into());
    winner.save().await.unwrap();

    let controller = crate::core::all::all_registered_controllers()
        .into_iter()
        .find(|controller| controller.rpc_method_name() == "openhuman.agent_registry_update")
        .expect("agent registry RPC is registered");
    let ctx = CoreContext::for_test_with_config(DomainSet::full(), stale);
    let error = CoreContext::scope(ctx.clone(), async move {
        (controller.handler)(
            DispatchCtx::new(ctx),
            json!({"id": "orchestrator", "enabled": true})
                .as_object()
                .unwrap()
                .clone(),
        )
        .await
        .unwrap_err()
    })
    .await;
    let structured = crate::core::StructuredRpcError::decode(&error)
        .expect("agent registry RPC preserves the structured config conflict");
    assert_eq!(structured.data.as_ref().unwrap()["kind"], "ConfigConflict");
    assert_eq!(structured.data.as_ref().unwrap()["retryable"], true);
}

#[tokio::test]
async fn memory_engine_rpc_preserves_document_conflicts() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = MemoryStorage::new();
    let _forced = ForcedDocumentSource::new(
        scoped(&storage, "tenant-memory-rpc"),
        Scope::new("tenant-memory-rpc").unwrap(),
        keys(10),
    );
    let seed = config_at(tmp.path(), "initial");
    seed.save().await.unwrap();
    let stale = Config::load_from_config_path(&seed.config_path, &seed.workspace_dir)
        .await
        .unwrap();
    let mut winner = Config::load_from_config_path(&seed.config_path, &seed.workspace_dir)
        .await
        .unwrap();
    winner.default_model = Some("winner".into());
    winner.save().await.unwrap();

    let controller = crate::core::all::all_registered_controllers()
        .into_iter()
        .find(|controller| controller.rpc_method_name() == "openhuman.memory_engine_set")
        .expect("memory engine RPC is registered");
    let ctx = CoreContext::for_test_with_config(DomainSet::full(), stale);
    let error = CoreContext::scope(ctx.clone(), async move {
        (controller.handler)(
            DispatchCtx::new(ctx),
            json!({"engine": "none"}).as_object().unwrap().clone(),
        )
        .await
        .unwrap_err()
    })
    .await;
    let structured = crate::core::StructuredRpcError::decode(&error)
        .expect("memory RPC preserves the structured config conflict");
    assert_eq!(structured.data.as_ref().unwrap()["kind"], "ConfigConflict");
    assert_eq!(structured.data.as_ref().unwrap()["retryable"], false);
}

#[tokio::test]
async fn document_source_write_uses_its_last_read_revision() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = MemoryStorage::new();
    let path = tmp.path().join("config.toml");
    let first = document_source(&storage, "tenant-source-cas", &path);
    let stale = document_source(&storage, "tenant-source-cas", &path);

    first.write("default_model = \"seed\"\n").await.unwrap();
    first.read().await.unwrap();
    stale.read().await.unwrap();
    first.write("default_model = \"winner\"\n").await.unwrap();

    let error = stale
        .write("output_language = \"fr\"\n")
        .await
        .expect_err("a source snapshot cannot overwrite a newer document revision");
    assert_typed_conflict(error);
}

#[tokio::test]
async fn an_unobserved_document_source_write_is_create_only() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = MemoryStorage::new();
    let path = tmp.path().join("config.toml");
    let seed = document_source(&storage, "tenant-source-create-only", &path);
    let blind = document_source(&storage, "tenant-source-create-only", &path);

    seed.write("default_model = \"seed\"\n").await.unwrap();
    let error = blind
        .write("default_model = \"blind-write\"\n")
        .await
        .expect_err("a new source handle must not replace an existing document");
    assert_typed_conflict(error);
}

fn assert_typed_conflict(error: anyhow::Error) {
    let storage_error = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<tinystoragedrivers::StorageError>())
        .expect("CAS failure retains the typed storage error");
    assert_eq!(
        storage_error.kind(),
        tinystoragedrivers::ErrorKind::Conflict
    );
}
