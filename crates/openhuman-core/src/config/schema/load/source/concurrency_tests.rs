use super::*;
use crate::core::runtime::{DispatchCtx, DomainSet, context::CoreContext};
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
async fn sequential_config_rpcs_use_the_latest_context_snapshot() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = MemoryStorage::new();
    let _forced = ForcedDocumentSource::new(
        scoped(&storage, "tenant-sequential-config-rpc"),
        Scope::new("tenant-sequential-config-rpc").unwrap(),
        keys(11),
    );
    let seed = config_at(tmp.path(), "initial");
    seed.save().await.unwrap();
    let config = Config::load_from_config_path(&seed.config_path, &seed.workspace_dir)
        .await
        .unwrap();
    let ctx = CoreContext::for_test_with_config(DomainSet::full(), config);
    let controller = crate::core::all::all_registered_controllers()
        .into_iter()
        .find(|controller| controller.rpc_method_name() == "openhuman.config_update_model_settings")
        .expect("config model settings RPC is registered");

    for patch in [
        json!({"default_model": "model-a"}),
        json!({"default_temperature": 0.8}),
    ] {
        let dispatch = DispatchCtx::new(ctx.clone());
        CoreContext::scope(ctx.clone(), async {
            (controller.handler)(dispatch, patch.as_object().unwrap().clone())
                .await
                .expect(
                    "sequential config settings updates should use the latest context snapshot",
                );
        })
        .await;
    }

    let saved = Config::load_from_config_path(&seed.config_path, &seed.workspace_dir)
        .await
        .unwrap();
    assert_eq!(saved.default_model.as_deref(), Some("model-a"));
    assert!((saved.default_temperature - 0.8).abs() < f64::EPSILON);
}

#[tokio::test]
async fn config_snapshot_cannot_be_saved_into_a_different_tenant_scope() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = MemoryStorage::new();
    let path = tmp.path().join("config.toml");
    let workspace = tmp.path().join("workspace");

    let (from_tenant_a, tenant_a_version) = {
        let _source = ForcedDocumentSource::new(
            scoped(&storage, "tenant-source-a"),
            Scope::new("tenant-source-a").unwrap(),
            keys(12),
        );
        config_at(tmp.path(), "tenant-a").save().await.unwrap();
        let config = Config::load_from_config_path(&path, &workspace)
            .await
            .unwrap();
        let version = config.document_snapshot.lock().unwrap().version;
        (config, version)
    };

    let (tenant_b_version, tenant_b_model) = {
        let _source = ForcedDocumentSource::new(
            scoped(&storage, "tenant-source-b"),
            Scope::new("tenant-source-b").unwrap(),
            keys(12),
        );
        config_at(tmp.path(), "tenant-b").save().await.unwrap();
        let config = Config::load_from_config_path(&path, &workspace)
            .await
            .unwrap();
        let version = config.document_snapshot.lock().unwrap().version;
        let docs = Arc::clone(scoped(&storage, "tenant-source-b").documents());
        let stored = docs
            .get(
                super::super::CONFIG_COLLECTION,
                &super::super::storage_schema::document_id("tenant-source-b").unwrap(),
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(version, Some(stored.version));
        (version, config.default_model.clone())
    };
    let tenant_a_version = tenant_a_version.expect("tenant A config is document-backed");
    let tenant_b_version = tenant_b_version.expect("tenant B config is document-backed");
    assert_eq!(tenant_a_version, tenant_b_version);
    let mut from_tenant_a = from_tenant_a;
    assert_eq!(from_tenant_a.default_model.as_deref(), Some("tenant-a"));
    assert_eq!(tenant_b_model.as_deref(), Some("tenant-b"));

    from_tenant_a.default_model = Some("must-not-cross-tenant-boundary".into());
    let error = {
        let _source = ForcedDocumentSource::new(
            scoped(&storage, "tenant-source-b"),
            Scope::new("tenant-source-b").unwrap(),
            keys(12),
        );
        from_tenant_a.save().await.unwrap_err()
    };
    let storage_error = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<tinystoragedrivers::StorageError>())
        .expect("a save into a different source returns a typed conflict");
    assert_eq!(
        storage_error.kind(),
        tinystoragedrivers::ErrorKind::Conflict
    );

    let tenant_b = {
        let _source = ForcedDocumentSource::new(
            scoped(&storage, "tenant-source-b"),
            Scope::new("tenant-source-b").unwrap(),
            keys(12),
        );
        Config::load_from_config_path(&path, &workspace)
            .await
            .unwrap()
    };
    assert_eq!(tenant_b.default_model.as_deref(), Some("tenant-b"));
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
