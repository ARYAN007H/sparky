//! A short lease that serialises the one-time legacy importers across
//! processes.
//!
//! Every importer already writes its records with `Precondition::Absent` and
//! its completion marker the same way, so two processes importing one
//! workspace converge on the same documents. What the marker alone cannot
//! prevent is both of them doing the whole read-and-copy at the same time.
//! [`with_claim`] puts a claim document beside the marker (`<marker>#claim`)
//! with `Precondition::Absent` before the import starts; the process that
//! loses the race skips the import (the winner writes the marker) instead of
//! repeating it. A claim is released when the import ends, success or failure,
//! and a claim older than [`CLAIM_TTL_MS`] (its owner crashed) is taken over
//! with a version-checked write, so a dead process never blocks the import.

use anyhow::Result;
use serde_json::json;
use tinystoragedrivers::{ErrorKind, Precondition};

use super::documents::Repo;

/// How long a claim blocks other processes before it counts as abandoned.
pub const CLAIM_TTL_MS: i64 = 10 * 60 * 1000;

const LOG_PREFIX: &str = "[storage::import_claim]";

/// The claim document id for a marker id.
pub fn claim_id(marker: &str) -> String {
    format!("{marker}#claim")
}

/// Runs `body` while holding the claim for `marker` in `collection`, which
/// `repo` must declare. Returns `None` without running `body` when another
/// live process holds the claim.
///
/// # Errors
///
/// A storage error while claiming, or whatever `body` returns. The claim is
/// released either way.
pub fn with_claim<T>(
    repo: &Repo,
    collection: &'static str,
    marker: &str,
    body: impl FnOnce() -> Result<T>,
) -> Result<Option<T>> {
    with_claim_at(
        repo,
        collection,
        marker,
        chrono::Utc::now().timestamp_millis(),
        CLAIM_TTL_MS,
        body,
    )
}

/// [`with_claim`] with an explicit clock and lease length, for tests.
pub fn with_claim_at<T>(
    repo: &Repo,
    collection: &'static str,
    marker: &str,
    now_ms: i64,
    ttl_ms: i64,
    body: impl FnOnce() -> Result<T>,
) -> Result<Option<T>> {
    let id = claim_id(marker);
    let Some(held) = acquire(repo, collection, &id, now_ms, ttl_ms)? else {
        tracing::debug!(
            target: "openhuman::storage_import",
            claim = %id,
            "{LOG_PREFIX} another process holds the import claim; skipping"
        );
        return Ok(None);
    };
    let result = body();
    let release_id = id.clone();
    let released = repo.run(move |docs| async move {
        docs.delete(collection, &release_id, Precondition::Version(held))
            .await
    });
    if let Err(error) = released {
        // The lease expires on its own; releasing is an optimisation.
        tracing::warn!(
            target: "openhuman::storage_import",
            claim = %id,
            error = %format!("{error:#}"),
            "{LOG_PREFIX} could not release the import claim; it expires on its own"
        );
    }
    result.map(Some)
}

/// Takes the claim, returning the version of the claim document, or `None`
/// when a live claim belongs to someone else.
fn acquire(
    repo: &Repo,
    collection: &'static str,
    id: &str,
    now_ms: i64,
    ttl_ms: i64,
) -> Result<Option<tinystoragedrivers::Version>> {
    let id = id.to_string();
    repo.run(move |docs| async move {
        let doc = json!({ "claimed_at_ms": now_ms, "pid": std::process::id() });
        // A few rounds cover a claim that is released between our two reads.
        for _ in 0..4 {
            match docs
                .put(collection, &id, doc.clone(), Precondition::Absent)
                .await
            {
                Ok(version) => return Ok(Some(version)),
                Err(error) if error.kind() == ErrorKind::Conflict => {}
                Err(error) => return Err(error),
            }
            let Some(stored) = docs.get(collection, &id).await? else {
                continue;
            };
            let claimed = stored
                .doc
                .get("claimed_at_ms")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0);
            if now_ms.saturating_sub(claimed) < ttl_ms {
                return Ok(None);
            }
            match docs
                .put(collection, &id, doc.clone(), stored.unchanged())
                .await
            {
                Ok(version) => return Ok(Some(version)),
                // Someone else took the stale claim first.
                Err(error) if error.kind() == ErrorKind::Conflict => return Ok(None),
                Err(error) => return Err(error),
            }
        }
        Ok(None)
    })
}

#[cfg(test)]
#[path = "import_claim_tests.rs"]
mod tests;
