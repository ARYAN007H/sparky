use super::*;

#[tokio::test]
async fn profile_context_conflict_refresh_keeps_host_owned_restrictions() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = MemoryStorage::new();
    let _forced = ForcedDocumentSource::new(
        scoped(&storage, "tenant-profile-refresh"),
        Scope::new("tenant-profile-refresh").unwrap(),
        keys(16),
    );
    let operator = CoreContext::for_test(DomainSet::full(), None);
    let mut saas = crate::core::runtime::SaasConfig::new(tmp.path());
    saas.max_tool_iterations = Some(4);
    let host = crate::profiles::ProfileHost::new(saas, operator);
    let id = crate::profiles::ProfileId::for_user(
        "profile-refresh",
        crate::profiles::ProfileIdMode::Raw,
    )
    .unwrap();
    host.provision(&id).await.unwrap();
    let profile = host.open(&id).await.unwrap();
    let context = Arc::clone(profile.context());

    let initial = CoreContext::scope(context.clone(), async {
        DispatchCtx::new(context.clone())
            .load_config()
            .await
            .unwrap()
    })
    .await;
    assert!(initial.autonomy.enabled);
    assert!(initial.autonomy.workspace_only);
    assert_eq!(initial.memory.agent_id.as_deref(), Some(id.as_str()));
    assert_eq!(initial.agent.max_tool_iterations_override, Some(4));
    CoreContext::scope(context.clone(), initial.save())
        .await
        .unwrap();

    let mut stale = CoreContext::scope(context.clone(), async {
        DispatchCtx::new(context.clone())
            .load_config()
            .await
            .unwrap()
    })
    .await;
    let mut external = Config::load_from_config_path(&stale.config_path, &stale.workspace_dir)
        .await
        .unwrap();
    external.default_model = Some("external-winner".into());
    external.autonomy.enabled = false;
    external.autonomy.workspace_only = false;
    external.memory.agent_id = Some("external-agent".into());
    external.memory.root = Some("team:external".into());
    external.agent.max_tool_iterations_override = Some(99);
    external.agent.channel_permissions.clear();
    external.save().await.unwrap();

    stale.default_temperature = 0.8;
    let error = CoreContext::scope(context.clone(), stale.save())
        .await
        .expect_err("the stale profile snapshot must conflict with the external write");
    assert!(error.chain().any(|cause| {
        cause
            .downcast_ref::<tinystoragedrivers::StorageError>()
            .is_some_and(|error| error.kind() == tinystoragedrivers::ErrorKind::Conflict)
    }));

    let refreshed = CoreContext::scope(context.clone(), async {
        DispatchCtx::new(context.clone())
            .load_config()
            .await
            .unwrap()
    })
    .await;
    assert_eq!(refreshed.default_model.as_deref(), Some("external-winner"));
    assert!(refreshed.autonomy.enabled);
    assert!(refreshed.autonomy.workspace_only);
    assert_eq!(refreshed.memory.agent_id.as_deref(), Some(id.as_str()));
    assert_eq!(refreshed.agent.max_tool_iterations_override, Some(4));
    assert!(!refreshed.agent.channel_permissions.is_empty());
    let nested = context.derive_with(ContextOverlay::new(
        refreshed,
        context.domains(),
        crate::tools::toolpacks::ToolGroups::none(),
    ));
    let mut nested_stale = CoreContext::scope(nested.clone(), async {
        DispatchCtx::new(nested.clone())
            .load_config()
            .await
            .unwrap()
    })
    .await;
    let mut later_external =
        Config::load_from_config_path(&nested_stale.config_path, &nested_stale.workspace_dir)
            .await
            .unwrap();
    later_external.default_model = Some("external-after-nested".into());
    later_external.autonomy.enabled = false;
    later_external.memory.agent_id = Some("later-external-agent".into());
    later_external.agent.channel_permissions.clear();
    later_external.save().await.unwrap();

    nested_stale.default_temperature = 0.9;
    CoreContext::scope(nested.clone(), nested_stale.save())
        .await
        .expect_err("a nested profile snapshot must conflict with the later external write");
    let nested_refreshed = CoreContext::scope(nested.clone(), async {
        DispatchCtx::new(nested.clone())
            .load_config()
            .await
            .unwrap()
    })
    .await;
    assert_eq!(
        nested_refreshed.default_model.as_deref(),
        Some("external-after-nested")
    );
    assert!(nested_refreshed.autonomy.enabled);
    assert_eq!(
        nested_refreshed.memory.agent_id.as_deref(),
        Some(id.as_str())
    );
    assert!(!nested_refreshed.agent.channel_permissions.is_empty());
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
                super::super::storage_schema::CONFIG_COLLECTION,
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
async fn config_snapshot_is_bound_to_its_backend_instance() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("config.toml");
    let workspace = tmp.path().join("workspace");
    let storage_a = MemoryStorage::new();
    let storage_b = MemoryStorage::new();
    let config_a = {
        let _source = ForcedDocumentSource::new(
            scoped(&storage_a, "tenant-same-anchor"),
            Scope::new("tenant-same-anchor").unwrap(),
            keys(14),
        );
        config_at(tmp.path(), "backend-a").save().await.unwrap();
        Config::load_from_config_path(&path, &workspace)
            .await
            .unwrap()
    };
    let (config_b, version_b) = {
        let _source = ForcedDocumentSource::new(
            scoped(&storage_b, "tenant-same-anchor"),
            Scope::new("tenant-same-anchor").unwrap(),
            keys(14),
        );
        config_at(tmp.path(), "backend-b").save().await.unwrap();
        let config = Config::load_from_config_path(&path, &workspace)
            .await
            .unwrap();
        let version = config.document_snapshot.lock().unwrap().version;
        (config, version)
    };
    let version_a = config_a.document_snapshot.lock().unwrap().version;
    assert_eq!(
        version_a, version_b,
        "fixture uses colliding document versions"
    );
    let anchor_a = config_a.document_snapshot.lock().unwrap().anchor.clone();
    let anchor_b = config_b.document_snapshot.lock().unwrap().anchor.clone();
    assert_ne!(anchor_a, anchor_b, "source anchors retain backend identity");

    let error = {
        let _source = ForcedDocumentSource::new(
            scoped(&storage_b, "tenant-same-anchor"),
            Scope::new("tenant-same-anchor").unwrap(),
            keys(14),
        );
        let mut from_a = config_a;
        from_a.default_model = Some("must-not-cross-backend-instance".into());
        from_a.save().await.unwrap_err()
    };
    assert_typed_conflict(error);
}

#[tokio::test]
async fn a_file_snapshot_can_seed_an_absent_document_after_storage_bootstrap() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("config.toml");
    let workspace = tmp.path().join("workspace");
    let file_config = config_at(tmp.path(), "file-seed");
    file_config.save().await.unwrap();
    let mut snapshot = Config::load_from_config_path(&path, &workspace)
        .await
        .unwrap();
    assert!(matches!(
        snapshot.document_snapshot.lock().unwrap().anchor.clone(),
        Some(ConfigSourceAnchor::File(_))
    ));

    let storage = MemoryStorage::new();
    let _source = ForcedDocumentSource::new(
        scoped(&storage, "tenant-file-bootstrap"),
        Scope::new("tenant-file-bootstrap").unwrap(),
        keys(15),
    );
    snapshot.default_model = Some("imported-to-document".into());
    snapshot.save().await.unwrap();
    let loaded = Config::load_from_config_path(&path, &workspace)
        .await
        .unwrap();
    assert_eq!(
        loaded.default_model.as_deref(),
        Some("imported-to-document")
    );
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
