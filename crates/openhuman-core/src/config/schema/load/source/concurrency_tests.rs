use super::*;
use crate::core::runtime::{context::CoreContext, ContextOverlay, DispatchCtx, DomainSet};
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

#[path = "revision_loader_tests.rs"]
mod revision_loader_tests;

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
async fn external_config_conflict_refreshes_the_context_before_the_next_read() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = MemoryStorage::new();
    let _forced = ForcedDocumentSource::new(
        scoped(&storage, "tenant-refresh-config-rpc"),
        Scope::new("tenant-refresh-config-rpc").unwrap(),
        keys(13),
    );
    let seed = config_at(tmp.path(), "initial");
    seed.save().await.unwrap();
    let config = Config::load_from_config_path(&seed.config_path, &seed.workspace_dir)
        .await
        .unwrap();
    let ctx = CoreContext::for_test_with_config(DomainSet::full(), config);

    let mut external = Config::load_from_config_path(&seed.config_path, &seed.workspace_dir)
        .await
        .unwrap();
    external.default_model = Some("external-winner".into());
    external.save().await.unwrap();

    let controller = crate::core::all::all_registered_controllers()
        .into_iter()
        .find(|controller| controller.rpc_method_name() == "openhuman.config_update_model_settings")
        .expect("config model settings RPC is registered");
    let conflict = CoreContext::scope(ctx.clone(), async {
        (controller.handler)(
            DispatchCtx::new(ctx.clone()),
            json!({"default_temperature": 0.8})
                .as_object()
                .unwrap()
                .clone(),
        )
        .await
        .unwrap_err()
    })
    .await;
    assert!(crate::core::StructuredRpcError::decode(&conflict).is_some());

    let refreshed = CoreContext::scope(ctx.clone(), async {
        DispatchCtx::new(ctx.clone()).load_config().await.unwrap()
    })
    .await;
    assert_eq!(refreshed.default_model.as_deref(), Some("external-winner"));
}

#[tokio::test]
async fn external_config_conflict_refresh_keeps_derived_runtime_overrides() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = MemoryStorage::new();
    let _forced = ForcedDocumentSource::new(
        scoped(&storage, "tenant-refresh-overlay"),
        Scope::new("tenant-refresh-overlay").unwrap(),
        keys(14),
    );
    let mut seed = config_at(tmp.path(), "initial");
    seed.output_language = Some("seed-language".into());
    seed.save().await.unwrap();
    let mut runtime_config = Config::load_from_config_path(&seed.config_path, &seed.workspace_dir)
        .await
        .unwrap();
    let parent = CoreContext::for_test_with_config(DomainSet::full(), runtime_config.clone());
    assert!(runtime_config.mcp_client.enabled);
    assert!(!runtime_config.autonomy.enabled);
    assert!(runtime_config.memory.agent_id.is_none());
    assert!(runtime_config.composio.host_credential.is_none());
    runtime_config.memory.agent_id = Some("runtime-memory-agent".into());
    runtime_config.memory.root = Some("team:runtime".into());
    runtime_config.mcp_client.enabled = false;
    runtime_config.autonomy.enabled = true;
    runtime_config.output_language = None;
    runtime_config.composio.pin_host_credential(
        crate::config::schema::ComposioHostCredential::direct("overlay-only-test-secret"),
    );
    let ctx = parent.derive_with(ContextOverlay::new(
        runtime_config,
        DomainSet::full(),
        crate::tools::toolpacks::ToolGroups::none(),
    ));

    let mut external = Config::load_from_config_path(&seed.config_path, &seed.workspace_dir)
        .await
        .unwrap();
    external.default_model = Some("external-winner".into());
    external.default_temperature = 0.4;
    external.save().await.unwrap();

    let controller = crate::core::all::all_registered_controllers()
        .into_iter()
        .find(|controller| controller.rpc_method_name() == "openhuman.config_update_model_settings")
        .expect("config model settings RPC is registered");
    let conflict = CoreContext::scope(ctx.clone(), async {
        (controller.handler)(
            DispatchCtx::new(ctx.clone()),
            json!({"default_temperature": 0.8})
                .as_object()
                .unwrap()
                .clone(),
        )
        .await
        .unwrap_err()
    })
    .await;
    assert!(crate::core::StructuredRpcError::decode(&conflict).is_some());

    let refreshed = CoreContext::scope(ctx.clone(), async {
        DispatchCtx::new(ctx.clone()).load_config().await.unwrap()
    })
    .await;
    assert_eq!(refreshed.default_model.as_deref(), Some("external-winner"));
    assert!((refreshed.default_temperature - 0.4).abs() < f64::EPSILON);
    assert!(refreshed.output_language.is_none());
    assert_eq!(
        refreshed.memory.agent_id.as_deref(),
        Some("runtime-memory-agent")
    );
    assert_eq!(refreshed.memory.root.as_deref(), Some("team:runtime"));
    assert!(!refreshed.mcp_client.enabled);
    assert!(refreshed.autonomy.enabled);
    assert!(refreshed.composio.host_credential.is_some());

    CoreContext::scope(ctx.clone(), async {
        (controller.handler)(
            DispatchCtx::new(ctx.clone()),
            json!({"default_temperature": 0.6})
                .as_object()
                .unwrap()
                .clone(),
        )
        .await
        .expect("a save after conflict refresh should use the advanced revision");
    })
    .await;

    let saved_context_config = CoreContext::scope(ctx.clone(), async {
        DispatchCtx::new(ctx.clone()).load_config().await.unwrap()
    })
    .await;
    let grandchild = ctx.derive_with(ContextOverlay::new(
        saved_context_config,
        DomainSet::full(),
        crate::tools::toolpacks::ToolGroups::none(),
    ));

    let mut later_external = Config::load_from_config_path(&seed.config_path, &seed.workspace_dir)
        .await
        .unwrap();
    later_external.default_model = Some("external-after-save".into());
    later_external.memory.agent_id = Some("external-agent".into());
    later_external.memory.root = Some("team:external".into());
    later_external.mcp_client.enabled = true;
    later_external.autonomy.enabled = false;
    later_external.save().await.unwrap();
    let later_conflict = CoreContext::scope(grandchild.clone(), async {
        (controller.handler)(
            DispatchCtx::new(grandchild.clone()),
            json!({"default_temperature": 0.7})
                .as_object()
                .unwrap()
                .clone(),
        )
        .await
        .unwrap_err()
    })
    .await;
    assert!(crate::core::StructuredRpcError::decode(&later_conflict).is_some());

    let after_later_conflict = CoreContext::scope(grandchild.clone(), async {
        DispatchCtx::new(grandchild.clone())
            .load_config()
            .await
            .unwrap()
    })
    .await;
    assert_eq!(
        after_later_conflict.default_model.as_deref(),
        Some("external-after-save")
    );
    assert_eq!(
        after_later_conflict.memory.agent_id.as_deref(),
        Some("runtime-memory-agent")
    );
    assert_eq!(
        after_later_conflict.memory.root.as_deref(),
        Some("team:runtime")
    );
    assert!(!after_later_conflict.mcp_client.enabled);
    assert!(after_later_conflict.autonomy.enabled);
    assert!(after_later_conflict.composio.host_credential.is_some());
}

#[tokio::test]
async fn nested_context_refresh_does_not_pin_a_previous_external_value() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = MemoryStorage::new();
    let _forced = ForcedDocumentSource::new(
        scoped(&storage, "tenant-refresh-nested-overlay"),
        Scope::new("tenant-refresh-nested-overlay").unwrap(),
        keys(15),
    );
    let seed = config_at(tmp.path(), "initial");
    seed.save().await.unwrap();
    let base = Config::load_from_config_path(&seed.config_path, &seed.workspace_dir)
        .await
        .unwrap();
    let parent = CoreContext::for_test_with_config(DomainSet::full(), base.clone());
    let mut first_config = base;
    first_config.memory.root = Some("team:runtime".into());
    let first_context = parent.derive_with(ContextOverlay::new(
        first_config,
        DomainSet::full(),
        crate::tools::toolpacks::ToolGroups::none(),
    ));

    let controller = crate::core::all::all_registered_controllers()
        .into_iter()
        .find(|controller| controller.rpc_method_name() == "openhuman.config_update_model_settings")
        .expect("config model settings RPC is registered");
    let mut external = Config::load_from_config_path(&seed.config_path, &seed.workspace_dir)
        .await
        .unwrap();
    external.default_model = Some("external-first".into());
    external.save().await.unwrap();
    let first_conflict = CoreContext::scope(first_context.clone(), async {
        (controller.handler)(
            DispatchCtx::new(first_context.clone()),
            json!({"default_temperature": 0.8})
                .as_object()
                .unwrap()
                .clone(),
        )
        .await
        .unwrap_err()
    })
    .await;
    assert!(crate::core::StructuredRpcError::decode(&first_conflict).is_some());

    let after_first_refresh = CoreContext::scope(first_context.clone(), async {
        DispatchCtx::new(first_context.clone())
            .load_config()
            .await
            .unwrap()
    })
    .await;
    let nested = first_context.derive_with(ContextOverlay::new(
        after_first_refresh,
        DomainSet::full(),
        crate::tools::toolpacks::ToolGroups::none(),
    ));

    external = Config::load_from_config_path(&seed.config_path, &seed.workspace_dir)
        .await
        .unwrap();
    external.default_model = Some("external-second".into());
    external.save().await.unwrap();
    let second_conflict = CoreContext::scope(nested.clone(), async {
        (controller.handler)(
            DispatchCtx::new(nested.clone()),
            json!({"default_temperature": 0.7})
                .as_object()
                .unwrap()
                .clone(),
        )
        .await
        .unwrap_err()
    })
    .await;
    assert!(crate::core::StructuredRpcError::decode(&second_conflict).is_some());

    let refreshed = CoreContext::scope(nested.clone(), async {
        DispatchCtx::new(nested.clone())
            .load_config()
            .await
            .unwrap()
    })
    .await;
    assert_eq!(refreshed.default_model.as_deref(), Some("external-second"));
    assert_eq!(refreshed.memory.root.as_deref(), Some("team:runtime"));
}

#[path = "profile_refresh_tests.rs"]
mod profile_refresh_tests;
