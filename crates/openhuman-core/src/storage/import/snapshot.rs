//! A private, throw-away copy of one legacy database.
//!
//! Reading a SQLite file opens it for writing as far as the legacy readers
//! are concerned (they create missing tables and migrate an old schema), and
//! the storage driver declares its own tables on open. Neither may touch a
//! user's real file, so the importer reads a byte copy in a scratch
//! directory laid out like the workspace.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tempfile::TempDir;
use tinystoragedrivers::CollectionSpec;

use super::Record;

/// A copy of one legacy database (with its `-wal` / `-shm` files, so
/// uncheckpointed writes are seen) inside a scratch workspace. Removed on
/// drop.
pub struct Snapshot {
    dir: TempDir,
    db: PathBuf,
}

impl Snapshot {
    /// Copies `source` (whose workspace-relative path is `relative`) into a
    /// scratch workspace.
    ///
    /// # Errors
    ///
    /// When the scratch directory or a copy cannot be written, or `source`
    /// cannot be read.
    pub fn of(source: &Path, relative: &str) -> Result<Self> {
        let dir = tempfile::Builder::new()
            .prefix("openhuman-storage-import-")
            .tempdir()
            .context("create the scratch directory")?;
        let db = dir.path().join(relative);
        if let Some(parent) = db.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        std::fs::copy(source, &db).with_context(|| format!("copy {}", source.display()))?;
        for suffix in ["-wal", "-shm"] {
            let extra = companion(source, suffix);
            if extra.is_file() {
                std::fs::copy(&extra, companion(&db, suffix))
                    .with_context(|| format!("copy {}", extra.display()))?;
            }
        }
        Ok(Self { dir, db })
    }

    /// The scratch workspace the copy lives in.
    pub fn workspace(&self) -> &Path {
        self.dir.path()
    }

    /// The copied database file.
    pub fn db(&self) -> &Path {
        &self.db
    }

    /// Every record in the copy's driver document tables, in the collections
    /// `collections` names. A file with none yields nothing.
    ///
    /// # Errors
    ///
    /// When the copy cannot be opened with the SQLite driver or read.
    #[cfg(feature = "storage-sqlite")]
    pub fn driver_docs(&self, collections: fn() -> Vec<CollectionSpec>) -> Result<Vec<Record>> {
        use tinystoragedrivers::{DocumentStoreExt, Query, Scope, StorageBackend};

        let storage = tinystoragedrivers::sqlite::SqliteStorage::open(&self.db)
            .with_context(|| format!("open the copy {}", self.db.display()))?;
        let scoped = storage
            .for_scope(&Scope::local())
            .context("open the local scope of the copy")?;
        let docs = std::sync::Arc::clone(scoped.documents());
        let specs = collections();
        let found = crate::storage::block_on(async move {
            let mut out = Vec::new();
            for spec in specs {
                docs.ensure_collection(&spec).await?;
                for stored in docs.query_all(&spec.name, &Query::all()).await? {
                    out.push(Record {
                        collection: spec.name.clone(),
                        id: stored.id,
                        doc: stored.doc,
                    });
                }
            }
            Ok(out)
        })?;
        Ok(found)
    }

    /// Without the SQLite driver there is no way to read driver tables.
    #[cfg(not(feature = "storage-sqlite"))]
    pub fn driver_docs(&self, _collections: fn() -> Vec<CollectionSpec>) -> Result<Vec<Record>> {
        Ok(Vec::new())
    }
}

fn companion(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}
