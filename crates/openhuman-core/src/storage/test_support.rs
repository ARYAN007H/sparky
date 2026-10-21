//! Test-only storage backends backed by the real TSD testkit and drivers.

use std::sync::Arc;

use crate::storage::{StorageBackend, block_on_anyhow};

/// A real TSD in-memory backend for tests that do not require conditional
/// writes or persistence across backend instances.
pub fn memory_backend() -> Arc<dyn StorageBackend> {
    Arc::new(tinystoragedrivers_testkit::MemoryBackend::new())
}

/// An on-disk SQLite backend whose temporary directory remains alive as long
/// as the fixture is retained. Use it for persistence, conditional-write,
/// fence, and import-claim tests; the TSD memory backend does not emulate those
/// capabilities.
#[cfg(feature = "storage-sqlite")]
pub struct SqliteBackend {
    _directory: tempfile::TempDir,
    /// The actual SQLite backend opened from [`Self::url`].
    pub backend: Arc<dyn StorageBackend>,
    /// URL suitable for configuring the same isolated SQLite database.
    pub url: String,
}

/// Opens an isolated on-disk SQLite backend for tests that need its real
/// cross-process conditions or durable state.
#[cfg(feature = "storage-sqlite")]
pub fn sqlite_backend() -> anyhow::Result<SqliteBackend> {
    let directory = tempfile::tempdir()?;
    let url = format!("sqlite:{}", directory.path().join("storage").display());
    sqlite_backend_in(directory, url)
}

/// Opens a SQLite backend at `url`, retaining `directory` for the lifetime of
/// the returned backend fixture.
#[cfg(feature = "storage-sqlite")]
pub fn sqlite_backend_in(
    directory: tempfile::TempDir,
    url: impl Into<String>,
) -> anyhow::Result<SqliteBackend> {
    let url = url.into();
    let backend = block_on_anyhow({
        let url = url.clone();
        async move { crate::storage::open(&url).await }
    })?;
    Ok(SqliteBackend {
        _directory: directory,
        backend,
        url,
    })
}
