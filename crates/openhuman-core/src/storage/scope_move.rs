//! The one-time move of v1 scopes to the one scope grammar.
//!
//! v1 addressed one tenant five ways: a raw agent id (hashed to `sha256:<hex>`
//! when it was not a valid scope), `<profile>~<agent>` for a SaaS session,
//! `profile:<id>` / `profile-sha256:<hex>` for domain stores,
//! `module:<id>@<base>` / `<profile>~module:<id>` / `module-sha256:<hex>` for
//! loadable modules, and `default` for the agent-less session. v2 has
//! `local`, `u:<id>` and a principal under either (`/agent:<id>`,
//! `/module:<id>`), see [`super::scope`].
//!
//! A v2 backend does not enumerate legacy scopes. The host therefore reads an
//! explicit SQLite source through [`crate::storage::import::scope_move`] and
//! copies its typed inventory into mapped v2 scopes. The source is retained
//! unchanged for operator verification; unknown scope names are reported and
//! unsupported fidelity cases fail explicitly.

use std::collections::HashSet;
use std::sync::Arc;

use sha2::Digest;
use tinystoragedrivers_types::{Capability, CollectionSpec, ErrorKind, WriteOptions};

use super::scope::{scope_for_agent, scope_for_profile, tenant_scope};
use crate::storage::{
    DocumentStore, DocumentStoreExt, Precondition, Scope, StorageBackend, StorageError,
};

/// The ids a hashed v1 scope can be matched against.
#[derive(Debug, Default, Clone)]
pub struct Known {
    /// Agent ids recorded by earlier processes ([`super::agents`]).
    pub agents: Vec<String>,
    /// Provisioned profile ids (the profile registry).
    pub profiles: Vec<String>,
    /// Registry module ids.
    pub modules: Vec<String>,
}

/// What a scope string maps to.
#[derive(Debug, PartialEq, Eq)]
pub enum Mapping {
    /// Already in the v2 grammar; nothing to move.
    V2,
    /// Move everything here.
    Move(Scope),
    /// A hashed or malformed v1 scope no known id explains.
    Unknown,
}

fn sha_hex(bytes: &[u8]) -> String {
    hex::encode(sha2::Sha256::digest(bytes))
}

fn unescape_profile(escaped: &str) -> String {
    escaped.replace("%7E", "~").replace("%25", "%")
}

fn module_scope(profile: Option<&str>, id: &str) -> Mapping {
    match tenant_scope(profile).module(id) {
        Ok(scope) => Mapping::Move(scope),
        Err(_) => Mapping::Unknown,
    }
}

/// The v2 scope the v1 scope `src` maps to.
pub fn map_scope(src: &str, known: &Known) -> Mapping {
    if Scope::parse(src).is_ok() {
        return Mapping::V2;
    }
    if src == "default" {
        return Mapping::Move(scope_for_agent("default"));
    }
    if let Some(id) = src.strip_prefix("profile:") {
        return Mapping::Move(scope_for_profile(id));
    }
    if let Some(hex) = src.strip_prefix("profile-sha256:") {
        return known
            .profiles
            .iter()
            .find(|id| sha_hex(id.as_bytes()) == hex)
            .map_or(Mapping::Unknown, |id| Mapping::Move(scope_for_profile(id)));
    }
    if let Some(hex) = src.strip_prefix("module-sha256:") {
        // sha256(module id, 0, base scope); the base is `local` or an agent.
        let bases: Vec<&str> = std::iter::once("local")
            .chain(known.agents.iter().map(String::as_str))
            .collect();
        for id in &known.modules {
            for base in &bases {
                let mut digest = sha2::Sha256::new();
                digest.update(id.as_bytes());
                digest.update([0]);
                digest.update(base.as_bytes());
                if hex::encode(digest.finalize()) == hex {
                    return module_scope(None, id);
                }
            }
        }
        return Mapping::Unknown;
    }
    if let Some(hex) = src.strip_prefix("sha256:") {
        return known
            .agents
            .iter()
            .find(|id| sha_hex(id.as_bytes()) == hex)
            .map_or(Mapping::Unknown, |id| Mapping::Move(scope_for_agent(id)));
    }
    if let Some(rest) = src.strip_prefix("module:") {
        // `module:<id>@<base>`: modules are tenant-level, whichever agent was acting.
        let id = rest.rsplit_once('@').map_or(rest, |(id, _)| id);
        return module_scope(None, id);
    }
    if let Some((profile, agent)) = src.split_once('~') {
        let profile = unescape_profile(profile);
        if let Some(module) = agent.strip_prefix("module:") {
            return module_scope(Some(&profile), module);
        }
        let agent = if agent.is_empty() { "default" } else { agent };
        return match scope_for_profile(&profile).agent(agent) {
            Ok(scope) => Mapping::Move(scope),
            Err(_) => Mapping::Unknown,
        };
    }
    if src == "cluster" {
        return Mapping::V2;
    }
    // Any other literal v1 scope is a raw agent id.
    Mapping::Move(scope_for_agent(src))
}

/// What a run did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MoveReport {
    /// Source scopes moved to a v2 scope.
    pub moved: usize,
    /// Source scopes already recorded as moved.
    pub already_done: usize,
    /// v1 scopes no known id maps; left in place.
    pub unknown: Vec<String>,
    /// Documents copied.
    pub documents: u64,
    /// Stream entries copied.
    pub entries: u64,
    /// Blobs copied.
    pub blobs: u64,
}

/// Copies the rows in a source inventory to their mapped v2 scopes.
///
/// The source is deliberately not deleted. Physical inventory readers are
/// read-only, and source cleanup belongs to the operator after verifying the
/// target. Unknown scope names are returned for explicit operator handling.
///
/// # Errors
///
/// Fails closed on non-zero stream bases, timestamped seals, expiring blobs
/// (the target ports cannot preserve those source metadata fields), malformed
/// source data, or a target write error. Re-running is idempotent for
/// documents/blobs and verifies stream prefixes before appending missing
/// entries.
#[cfg(feature = "legacy-import")]
pub async fn copy_inventory(
    backend: &Arc<dyn StorageBackend>,
    known: &Known,
    inventory: &crate::storage::import::scope_move::LegacyScopeInventory,
) -> anyhow::Result<MoveReport> {
    use anyhow::bail;

    let mut report = MoveReport::default();
    let mut source_scopes: Vec<&str> = inventory.scopes.iter().map(String::as_str).collect();
    for row in &inventory.documents {
        source_scopes.push(&row.scope);
    }
    for row in &inventory.streams {
        source_scopes.push(&row.scope);
    }
    for row in &inventory.blobs {
        source_scopes.push(&row.scope);
    }
    source_scopes.sort_unstable();
    source_scopes.dedup();
    for source in source_scopes {
        let target_scope = match map_scope(source, known) {
            Mapping::V2 => continue,
            Mapping::Unknown => {
                report.unknown.push(source.to_owned());
                continue;
            }
            Mapping::Move(scope) => scope,
        };
        let target = backend.for_scope(&target_scope)?;
        let docs = target.documents();
        for row in inventory.documents.iter().filter(|row| row.scope == source) {
            docs.ensure_collection(&CollectionSpec::new(&row.collection))
                .await?;
            let mut options = WriteOptions::new();
            options.schema = row.meta.schema;
            options.created_ms = Some(row.meta.created_ms);
            options.expires_ms = row.meta.expires_ms;
            match docs
                .put_with(
                    &row.collection,
                    &row.id,
                    row.body.clone(),
                    Precondition::Absent,
                    options,
                )
                .await
            {
                Ok(_) => report.documents += 1,
                Err(error)
                    if matches!(error.kind(), ErrorKind::Conflict | ErrorKind::AlreadyExists) => {}
                Err(error) => return Err(error.into()),
            }
        }
        let streams = target.streams();
        for row in inventory.streams.iter().filter(|row| row.scope == source) {
            if row.base != 0 {
                bail!(
                    "cannot preserve non-zero base {} for legacy stream {source}/{} with the v2 append port",
                    row.base,
                    row.name
                );
            }
            if row.sealed_ms.is_some() {
                bail!(
                    "target stream port cannot preserve the seal timestamp for legacy stream {source}/{}",
                    row.name
                );
            }
            let existing = streams.len(&row.name).await?;
            if existing > row.entries.len() as u64 {
                bail!(
                    "target stream {source}/{} is longer than the legacy source",
                    row.name
                );
            }
            if existing > 0 {
                let prefix = streams.read_window(&row.name, 0, existing as usize).await?;
                if prefix.len() != existing as usize
                    || prefix
                        .iter()
                        .zip(&row.entries)
                        .any(|(old, (seq, value))| old.offset != *seq || old.value != *value)
                {
                    bail!(
                        "target stream {source}/{} conflicts with the legacy prefix",
                        row.name
                    );
                }
            }
            let tail = &row.entries[existing as usize..];
            if !tail.is_empty() {
                streams
                    .append_batch(
                        &row.name,
                        tail.iter().map(|(_, value)| value.clone()).collect(),
                    )
                    .await?;
                report.entries += tail.len() as u64;
            }
            if row.sealed {
                match streams.seal(&row.name).await {
                    Ok(_) => {}
                    Err(error) if matches!(error.kind(), ErrorKind::Unsupported(_)) => bail!(
                        "target cannot preserve sealed state for legacy stream {source}/{}",
                        row.name
                    ),
                    Err(error) => return Err(error.into()),
                }
            }
        }
        let blobs = target.blobs();
        for row in inventory.blobs.iter().filter(|row| row.scope == source) {
            if row.expires_ms.is_some() {
                bail!(
                    "target blob port cannot preserve absolute expiry for legacy blob {source}/{}",
                    row.key
                );
            }
            if blobs.head(&row.key).await?.is_none() {
                blobs
                    .put(&row.key, row.bytes.clone(), row.content_type.as_deref())
                    .await
                    .with_context(|| format!("copy legacy blob {source}/{}", row.key))?;
                report.blobs += 1;
            }
        }
        report.moved += 1;
    }
    Ok(report)
}

/// Reads an explicit SQLite source and copies its known legacy scopes.
///
/// The synchronous inventory read runs on a blocking worker. MongoDB and
/// other non-SQLite legacy sources need their own read-only inventory adapter;
/// they are not enumerated from the v2 target backend.
#[cfg(feature = "legacy-import")]
pub async fn run_from_source(
    backend: &Arc<dyn StorageBackend>,
    source_db: &std::path::Path,
    known: &Known,
) -> anyhow::Result<MoveReport> {
    use anyhow::Context;

    let path = source_db.to_owned();
    let inventory = tokio::task::spawn_blocking(move || {
        crate::storage::import::scope_move::LegacyScopeInventory::read(&path)
    })
    .await
    .context("join the legacy scope inventory reader")??;
    copy_inventory(backend, known, &inventory).await
}

/// Legacy compatibility entry point retained to produce an actionable error.
/// v2 backends cannot enumerate old scopes; callers must supply the old SQLite
/// path to [`crate::storage::import::scope_move::LegacyScopeInventory::read`]
/// and then pass that inventory to [`copy_inventory`].
#[deprecated(note = "use an explicit read-only legacy scope inventory")]
pub async fn run(_backend: &Arc<dyn StorageBackend>) -> Result<MoveReport, StorageError> {
    Err(StorageError::unsupported(
        Capability::CollectionEnumeration,
        "v2 backends do not enumerate legacy scopes; supply an explicit legacy database inventory",
    ))
}

/// The scopes of `scopes` that are not in the v2 grammar (for tests and the
/// boot log).
pub fn v1_scopes(scopes: &[Scope]) -> HashSet<&str> {
    scopes
        .iter()
        .filter(|scope| scope.parsed().is_err())
        .map(Scope::as_str)
        .collect()
}

#[cfg(test)]
#[path = "scope_move_tests.rs"]
mod tests;
