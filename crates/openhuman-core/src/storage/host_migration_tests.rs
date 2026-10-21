//! Boot prepares only the host's composed owner collections.
//!
//! Record conversion is tested beside each typed owner importer. Seeding
//! arbitrary v1-named collections into a v2 backend did not exercise those
//! source readers or prove a migration; it only relied on the retired global
//! schema registry.

use super::{StorageBackend, boot::prepare_and_migrate, schema::owner_collections};
use crate::storage::Scope;

#[tokio::test]
async fn boot_prepares_every_composed_owner_collection_in_shared_scopes() {
    let backend = crate::storage::test_support::memory_backend();
    let expected = owner_collections().unwrap();

    let report = prepare_and_migrate(&backend).await.unwrap();

    assert_eq!(report.scopes, 2);
    assert_eq!(report.collections, expected.len() * report.scopes);
    for scope in [Scope::local(), Scope::new("cluster").unwrap()] {
        let storage = backend.for_scope(&scope).unwrap();
        for collection in &expected {
            // A query fails if boot did not ensure the owner declaration.
            storage
                .documents()
                .query_all(&collection.name, &tinystoragedrivers_types::Query::all())
                .await
                .unwrap_or_else(|error| panic!("{} in {scope}: {error}", collection.name));
        }
    }
}
