//! One-time, read-only import of tinyjuice's on-disk CCR tier into the
//! module's blob storage.
//!
//! The compress-cache-retrieve tier used to write each offloaded original to
//! `<workspace>/.tokenjuice/ccr/<token>`, expiring by file age. From v0.10.0
//! the module keeps them as expiring blobs `tinyjuice_ccr/<token>` in its own
//! scope and writes no files. A model that was handed a recovery handle before
//! the upgrade can still fetch the original afterwards.
//!
//! * **Legacy files are never modified or deleted.** They are only read.
//! * **Nothing is overwritten.** A blob that already exists is kept.
//! * **Lifetime is kept.** A file older than the configured TTL is dropped, as
//!   the old tier would have on its next read; a younger one is written with
//!   the time it has left. With no TTL configured originals are kept.
//! * **Runs once per directory.** A marker document (in a collection the
//!   module's grant does not include) records it. An unreadable file writes no
//!   marker, so the next start retries. A name that is not a CCR token, or a
//!   file that is not UTF-8 text, is skipped.
//! * **No expiry support, no import.** A backend whose blob store cannot
//!   expire blobs leaves the module on its in-memory cache (it refuses a TTL
//!   it cannot honour), so nothing is written for it to find.

use std::path::Path;
use std::time::{Duration, SystemTime};

use anyhow::{anyhow, Context, Result};
use serde_json::json;
use tinystoragedrivers::{Capability, ScopedStorage};

use super::storage_grant_tinyjuice::{CCR_BLOB_PREFIX, MODULE_ID};
use super::storage_import_support as support;

const LOG_PREFIX: &str = "[tinyjuice-import]";
/// `tinyjuice::cache::store::short_hash`: 16 bytes, hex encoded.
const TOKEN_LEN: usize = 32;
const CONTENT_TYPE: &str = "text/plain; charset=utf-8";
/// Host-only: not in the module's grant, so the module cannot see it.
const MARKER_COLLECTION: &str = "tinyjuice_host_import";

/// How one import attempt ended.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// No storage backend is installed; the module is not using this storage.
    NoBackend,
    /// The marker says this directory was already imported.
    AlreadyDone,
    /// The directory does not exist.
    NoSource,
    /// The backend cannot expire blobs, so the module does not use it.
    Unsupported,
    /// The files were read and their originals written.
    Imported {
        imported: usize,
        expired: usize,
        skipped: usize,
    },
}

/// Imports the CCR files under `root` into the module's scope of the installed
/// backend. `ttl` is the configured original lifetime; `None` keeps them.
pub(crate) async fn import_legacy(root: &Path, ttl: Option<Duration>) -> Result<Outcome> {
    let Some(scoped) = support::module_scope(MODULE_ID)? else {
        return Ok(Outcome::NoBackend);
    };
    import_into(&scoped, root, ttl, SystemTime::now()).await
}

/// One readable original: its token, text and age.
struct LegacyOriginal {
    token: String,
    text: String,
    age: Duration,
}

async fn import_into(
    scoped: &ScopedStorage,
    root: &Path,
    ttl: Option<Duration>,
    now: SystemTime,
) -> Result<Outcome> {
    let docs = scoped.documents();
    let marker = support::marker_id("ccr", root);
    if support::already_imported(docs, MARKER_COLLECTION, &marker).await? {
        return Ok(Outcome::AlreadyDone);
    }
    if !root.is_dir() {
        return Ok(Outcome::NoSource);
    }
    let blobs = scoped.blobs();
    if ttl.is_some() && !blobs.capabilities().contains(Capability::BlobTtl) {
        return Ok(Outcome::Unsupported);
    }

    let dir = root.to_owned();
    let (originals, mut skipped) = tokio::task::spawn_blocking(move || read_originals(&dir, now))
        .await
        .context("the legacy reader panicked")??;

    let (mut imported, mut expired) = (0usize, 0usize);
    for original in originals {
        let remaining = match ttl {
            Some(ttl) => match ttl.checked_sub(original.age).filter(|left| !left.is_zero()) {
                Some(left) => Some(left),
                None => {
                    expired += 1;
                    continue;
                }
            },
            None => None,
        };
        let key = format!("{CCR_BLOB_PREFIX}{}", original.token);
        let present = blobs
            .head(&key)
            .await
            .map_err(|error| anyhow!("check a blob: {:?}", error.kind()))?
            .is_some();
        if present {
            skipped += 1;
            continue;
        }
        let bytes = original.text.into_bytes();
        match remaining {
            Some(left) => {
                blobs
                    .put_expiring(&key, bytes, Some(CONTENT_TYPE), left)
                    .await
            }
            None => blobs.put(&key, bytes, Some(CONTENT_TYPE)).await,
        }
        .map_err(|error| anyhow!("write a blob: {:?}", error.kind()))?;
        imported += 1;
    }

    support::mark_imported(
        docs,
        MARKER_COLLECTION,
        &marker,
        json!({ "imported": imported, "expired": expired, "skipped": skipped }),
    )
    .await?;
    tracing::info!("{LOG_PREFIX} imported={imported} expired={expired} skipped={skipped}");
    Ok(Outcome::Imported {
        imported,
        expired,
        skipped,
    })
}

fn is_token(name: &str) -> bool {
    name.len() == TOKEN_LEN && name.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn read_originals(root: &Path, now: SystemTime) -> Result<(Vec<LegacyOriginal>, usize)> {
    let mut originals = Vec::new();
    let mut skipped = 0usize;
    let mut entries = std::fs::read_dir(root)
        .with_context(|| format!("list {}", root.display()))?
        .collect::<std::io::Result<Vec<_>>>()
        .with_context(|| format!("list {}", root.display()))?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let name = entry.file_name();
        let Some(token) = name.to_str().filter(|name| is_token(name)) else {
            skipped += 1;
            continue;
        };
        let path = entry.path();
        // Never follow a link: it could point outside the CCR directory.
        let meta = std::fs::symlink_metadata(&path)
            .with_context(|| format!("stat {}", path.display()))?;
        if !meta.file_type().is_file() {
            skipped += 1;
            continue;
        }
        let modified = meta
            .modified()
            .with_context(|| format!("stat {}", path.display()))?;
        let bytes = std::fs::read(&path).with_context(|| format!("read {}", path.display()))?;
        let Ok(text) = String::from_utf8(bytes) else {
            skipped += 1;
            continue;
        };
        originals.push(LegacyOriginal {
            // The module keys blobs by the lowercase token it minted.
            token: token.to_ascii_lowercase(),
            text,
            age: now.duration_since(modified).unwrap_or_default(),
        });
    }
    Ok((originals, skipped))
}

#[cfg(test)]
#[path = "storage_import_tinyjuice_tests.rs"]
mod tests;
