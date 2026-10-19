use super::*;
use serde_json::json;

#[test]
fn storage_paths_are_stable_and_unambiguous_for_workspace_and_scope() {
    let key = RegistrationKey {
        workspace_dir: PathBuf::from("/workspace/a"),
        scope: "u:b".into(),
    };
    let same = RegistrationKey {
        workspace_dir: PathBuf::from("/workspace/a"),
        scope: "u:b".into(),
    };
    let different_workspace = RegistrationKey {
        workspace_dir: PathBuf::from("/workspace/a/u:b"),
        scope: "local".into(),
    };
    assert_eq!(path_for(&key), path_for(&same));
    assert_ne!(path_for(&key), path_for(&different_workspace));
}

#[test]
fn document_records_cannot_address_other_shared_or_graph_keys() {
    for collection in [
        "tsd.kv",
        "tsd.leases",
        "tsd.counters",
        "graph.checkpoints",
        "graph.threads",
        "graph.writes",
    ] {
        assert!(allowed_record(collection, "other-owner/key").is_err(), "{collection}");
        assert!(allowed_record(collection, "flows/thread/checkpoint").is_ok(), "{collection}");
    }
    for (collection, id) in [
        ("tsd.migrations", "other/definitions_v2"),
        ("tsd.migrations", "flows/unknown/migration"),
        ("account.credentials", "flows/secret"),
    ] {
        assert!(allowed_record(collection, id).is_err());
    }
}

#[test]
fn graph_query_and_delete_filters_are_confined_to_the_flows_namespace() {
    let all = flow_namespace_filter("graph.checkpoints", Filter::All);
    assert!(all.matches("flows/thread/1", &json!({"ns":"flows"})));
    assert!(!all.matches("other/thread/1", &json!({"ns":"other"})));

    let conflicting = flow_namespace_filter(
        "graph.writes",
        Filter::eq("ns", "other"),
    );
    assert!(!conflicting.matches("flows/thread/1", &json!({"ns":"flows"})));
    assert!(!conflicting.matches("other/thread/1", &json!({"ns":"other"})));

    let widened = flow_namespace_filter(
        "graph.threads",
        Filter::one_of("ns", ["flows", "other"]),
    );
    assert!(widened.matches("flows/thread", &json!({"ns":"flows"})));
    assert!(!widened.matches("other/thread", &json!({"ns":"other"})));
}

#[tokio::test]
async fn cancelling_the_caller_after_serve_keeps_one_cached_registration() {
    use crate::core::runtime::{ContextOverlay, CoreContext, DispatchCtx, DomainSet};
    use crate::tools::toolpacks::ToolGroups;

    let workspace = tempfile::tempdir().expect("isolated workspace");
    let config = Config {
        workspace_dir: workspace.path().to_path_buf(),
        ..Config::default()
    };
    let root = CoreContext::for_test_with_config(DomainSet::full(), config.clone());
    let core = root.derive_with(
        ContextOverlay::new(config.clone(), DomainSet::full(), ToolGroups::default())
            .profile("catalog-storage-cancellation-test"),
    );
    let context = DispatchCtx::new(core);
    let scope = crate::storage::scope_for_profile("catalog-storage-cancellation-test");
    let key = RegistrationKey {
        workspace_dir: config.workspace_dir.clone(),
        scope: scope.as_str().to_owned(),
    };
    let path = path_for(&key);
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    REGISTRATION_GATES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(
            path.clone(),
            RegistrationGate {
                entered: entered_tx,
                release: release_rx,
            },
        );

    let caller_context = context.clone();
    let caller_config = config.clone();
    let caller = tokio::spawn(async move { register(&caller_context, &caller_config).await });
    entered_rx.await.expect("module task served the callback");
    caller.abort();
    let _ = caller.await;
    release_tx.send(()).expect("finish cache insertion");

    let first = register(&context, &config)
        .await
        .expect("the completed process task committed its registration");
    let second = register(&context, &config)
        .await
        .expect("the cache returns the same scoped registration");
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(first.storage().callback.object_path, path);
    assert_eq!(
        REGISTRATION_SERVE_COUNTS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&path),
        Some(&1),
        "retry must not serve a replacement after the cancelled caller"
    );

    let runtime = crate::modules::runtime::runtime()
        .await
        .expect("module runtime remains available");
    let proxy = runtime
        .connection()
        .proxy(
            module::HOST_BUS_NAME,
            &path,
            catalog::STORAGE_INTERFACE,
        )
        .expect("storage callback address is valid");
    let reply: CatalogStorageReply = proxy
        .call(catalog::storage::CAPABILITIES, &())
        .await
        .expect("cached callback remains served");
    assert!(matches!(reply, CatalogStorageReply::Value { .. }));
}
