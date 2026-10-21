//! Prepare the host's known storage scopes before installing a backend.
//!
//! Tiny Storage Drivers v2 has no global schema registry or scope enumerator.
//! Collection declarations belong to each record owner and are composed by
//! [`super::schema::owner_collections`]. The host prepares the shared local
//! and cluster scopes here; tenant scopes are prepared when their owner is
//! created, using the same declarations.

use std::sync::Arc;

use tinystoragedrivers::{Scope, Storage, StorageBackend, StorageError};

/// What a boot preparation run did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BootReport {
    /// Known scopes prepared.
    pub scopes: usize,
    /// Collection declarations ensured across those scopes.
    pub collections: usize,
}

/// Ensures every composed owner collection in the host's shared scopes.
///
/// Per-profile and per-agent scopes are prepared when their owner is created;
/// no driver-specific scope enumeration is required or used.
///
/// # Errors
/// Returns a schema composition, scope binding, or collection preparation
/// error. Preparation is idempotent and may be retried on the next boot.
pub async fn prepare_and_migrate(
    backend: &Arc<dyn StorageBackend>,
) -> Result<BootReport, StorageError> {
    let collections = super::schema::owner_collections()?;
    let scopes = [Scope::new("local")?, Scope::new("cluster")?];
    let mut report = BootReport::default();
    for scope in scopes {
        let storage = backend.for_scope(&scope)?;
        for collection in &collections {
            storage.ensure_collection(collection).await?;
            report.collections += 1;
        }
        report.scopes += 1;
    }
    tracing::info!(
        target: "openhuman::storage",
        scopes = report.scopes,
        collections = report.collections,
        "[storage][boot] owner collections prepared"
    );
    Ok(report)
}

/// Prepares a tenant or module scope before its owner starts writing there.
///
/// # Errors
/// Returns the first owner declaration or backend error.
pub async fn prepare_scope(storage: &Storage) -> Result<usize, StorageError> {
    let collections = super::schema::owner_collections()?;
    for collection in &collections {
        storage.ensure_collection(collection).await?;
    }
    Ok(collections.len())
}

#[cfg(test)]
#[path = "boot_tests.rs"]
mod tests;
