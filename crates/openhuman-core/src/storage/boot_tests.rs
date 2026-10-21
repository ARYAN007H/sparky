use super::*;

#[tokio::test]
async fn boot_prepares_the_composed_owner_collections_on_host_scopes() {
    let backend: Arc<dyn StorageBackend> =
        Arc::new(tinystoragedrivers_testkit::MemoryBackend::new());

    let report = prepare_and_migrate(&backend).await.unwrap();
    let declared = super::super::schema::owner_collections().unwrap();
    assert_eq!(report.scopes, 2);
    assert_eq!(report.collections, declared.len() * report.scopes);

    // Owner declarations are idempotent: a retry after an interrupted boot
    // may safely ensure every collection again.
    let retry = prepare_and_migrate(&backend).await.unwrap();
    assert_eq!(retry, report);
}

#[tokio::test]
async fn tenant_scopes_use_the_same_owner_collection_declarations() {
    let backend = tinystoragedrivers_testkit::MemoryBackend::new();
    let scope = super::super::scope::scope_for_profile("profile-a");
    let storage = backend.for_scope(&scope).unwrap();

    let prepared = prepare_scope(&storage).await.unwrap();

    assert_eq!(
        prepared,
        super::super::schema::owner_collections().unwrap().len()
    );
}
