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
use std::path::Path;
#[cfg(feature = "legacy-import")]
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;

use sha2::Digest;
use tinystoragedrivers_types::{Capability, CollectionSpec, ErrorKind, WriteOptions};

use super::scope::{scope_for_agent, scope_for_profile, tenant_scope};
use crate::storage::import::Record;
use crate::storage::{Precondition, Scope, StorageBackend, StorageError};

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
    /// Source documents whose `updated_ms` is kept only in the immutable
    /// inventory because the target `WriteOptions` cannot restore it.
    pub updated_times_not_restored: u64,
}

const IMPORT_DOMAIN: &str = "scope_move";
const IMPORT_OWNER: &str = "storage";

fn marker_collections() -> Vec<CollectionSpec> {
    crate::storage::import_state::collection_specs()
}

/// Selects the historical v1 scope database from the configured profile or
/// workspace layout and imports its inventory through the normal boot/rebind
/// importer pipeline. The destination backend is never used to discover
/// source scopes.
///
/// A profile-layout source is `<profile>/workspace/storage/storage.db`, as
/// selected by [`crate::storage::layout::legacy::v1_db`]. An unprofiled v1
/// workspace uses the default backend's directory database at
/// `<workspace>/storage/storage.db`.
#[cfg(feature = "legacy-import")]
pub fn run_from_config(
    backend: &Arc<dyn StorageBackend>,
    config: &crate::config::Config,
) -> crate::storage::import::Report {
    let result = (|| {
        let source = legacy_scope_source(config);
        if !source.is_file() {
            if let Some(url) = crate::storage::explicit_url(config) {
                let parsed = crate::storage::StorageUrl::parse(&url)?;
                if parsed.driver() != "sqlite" {
                    anyhow::bail!(
                        "legacy scope inventory source driver `{}` is unsupported; only the configured v1 SQLite source can be read",
                        parsed.driver()
                    );
                }
            }
            return Ok(crate::storage::import::Outcome::NoSource);
        }

        let known = known_scope_ids(config)?;
        let path = source.clone();
        let inventory = crate::storage::import::scope_move::LegacyScopeInventory::read(&path)
            .with_context(|| format!("read legacy scope inventory {}", path.display()))?;
        import_inventory(backend, config, &source, &known, &inventory)
    })();

    crate::storage::import::Report {
        domain: IMPORT_DOMAIN,
        outcome: result,
    }
}

#[cfg(not(feature = "legacy-import"))]
pub fn run_from_config(
    _backend: &Arc<dyn StorageBackend>,
    _config: &crate::config::Config,
) -> crate::storage::import::Report {
    crate::storage::import::Report {
        domain: IMPORT_DOMAIN,
        outcome: Err(anyhow::anyhow!(
            "legacy scope inventory import requires the `legacy-import` feature"
        )),
    }
}

#[cfg(feature = "legacy-import")]
fn legacy_scope_source(config: &crate::config::Config) -> PathBuf {
    if let Some(target) = crate::storage::layout::Target::from_config(config) {
        return crate::storage::layout::legacy::v1_db(&target);
    }
    config
        .workspace_dir
        .join(crate::storage::config::DEFAULT_DIR)
        .join("storage.db")
}

#[cfg(feature = "legacy-import")]
fn known_scope_ids(config: &crate::config::Config) -> anyhow::Result<Known> {
    let mut profiles = Vec::new();
    if let Some(target) = crate::storage::layout::Target::from_config(config) {
        profiles.push(target.user_id);
    }
    if let Some(host) = crate::profiles::host::host() {
        let hosted = crate::storage::block_on_anyhow(async move {
            host.known_profiles().await.map_err(anyhow::Error::msg)
        })?;
        profiles.extend(hosted);
    }
    profiles.sort();
    profiles.dedup();

    let agents = crate::agent_registry::host_definition::AgentDefinitionRegistry::current()
        .map(|registry| {
            registry
                .list()
                .into_iter()
                .map(|definition| definition.id.clone())
                .collect()
        })
        .unwrap_or_default();
    let modules = crate::modules::registry::ALL
        .iter()
        .map(|module| module.id.to_owned())
        .collect();
    Ok(Known {
        agents,
        profiles,
        modules,
    })
}

#[cfg(feature = "legacy-import")]
fn import_inventory(
    backend: &Arc<dyn StorageBackend>,
    config: &crate::config::Config,
    source_path: &Path,
    known: &Known,
    inventory: &crate::storage::import::scope_move::LegacyScopeInventory,
) -> anyhow::Result<crate::storage::import::Outcome> {
    let mut report = MoveReport::default();
    let mut scopes = inventory.scopes.clone();
    scopes.extend(inventory.documents.iter().map(|row| row.scope.clone()));
    scopes.extend(inventory.streams.iter().map(|row| row.scope.clone()));
    scopes.extend(inventory.blobs.iter().map(|row| row.scope.clone()));
    scopes.sort();
    scopes.dedup();
    if scopes.is_empty() {
        return Ok(crate::storage::import::Outcome::Imported {
            found: 0,
            written: 0,
        });
    }

    let mut all_already_done = true;
    let mut in_progress = 0usize;
    let mut found = 0usize;
    let mut written = 0usize;
    for source_scope in scopes {
        let target_scope = match map_scope(&source_scope, known) {
            Mapping::Move(scope) => scope,
            Mapping::V2 => Scope::new(&source_scope)?,
            Mapping::Unknown => {
                report.unknown.push(source_scope);
                all_already_done = false;
                continue;
            }
        };
        let scope_backend = backend_for_target_scope(backend, &target_scope, known, config)?;
        let marker = crate::storage::documents::Repo::on(
            &scope_backend,
            &target_scope,
            "storage::scope_move",
            marker_collections,
        )?;
        let step = scope_move_step(source_path, &source_scope);
        if crate::storage::import_state::is_done(&marker, IMPORT_OWNER, &step)? {
            report.already_done += 1;
            let scope_found = inventory_scope_counts(inventory, &source_scope);
            found += scope_found;
            continue;
        }

        all_already_done = false;
        let scope_inventory = inventory_for_scope(inventory, &source_scope);
        let scope_known = known.clone();
        let scope_backend_for_claim = Arc::clone(&scope_backend);
        let marker_for_claim = marker.clone();
        let result = crate::storage::import_state::with_claim(
            &marker,
            IMPORT_OWNER,
            &step,
            move |condition| {
                let claimed_marker = marker_for_claim.conditioned(condition)?;
                if crate::storage::import_state::is_done(&claimed_marker, IMPORT_OWNER, &step)? {
                    return Ok(None);
                }
                let claim_bound = crate::storage::import_state::backend_with_condition(
                    scope_backend_for_claim,
                    condition.clone(),
                );
                let copied = crate::storage::block_on_anyhow(copy_inventory(
                    &claim_bound,
                    &scope_known,
                    &scope_inventory,
                ))?;
                crate::storage::import_state::set_done(&claimed_marker, IMPORT_OWNER, &step)?;
                Ok(Some(copied))
            },
        )?;

        match result {
            None => in_progress += 1,
            Some(None) => {
                report.already_done += 1;
                found += inventory_scope_counts(inventory, &source_scope);
            }
            Some(Some(scope_report)) => {
                let scope_found = inventory_scope_counts(inventory, &source_scope);
                found += scope_found;
                written += usize::try_from(
                    scope_report.documents + scope_report.entries + scope_report.blobs,
                )
                .unwrap_or(usize::MAX);
                report.moved += scope_report.moved;
                report.documents += scope_report.documents;
                report.entries += scope_report.entries;
                report.blobs += scope_report.blobs;
                report.updated_times_not_restored += scope_report.updated_times_not_restored;
            }
        }
    }

    if !report.unknown.is_empty() {
        return Ok(crate::storage::import::Outcome::Incomplete {
            found,
            written,
            skipped: report.unknown.len(),
        });
    }
    if in_progress > 0 {
        if all_already_done {
            return Ok(crate::storage::import::Outcome::InProgress);
        }
        return Ok(crate::storage::import::Outcome::Incomplete {
            found,
            written,
            skipped: in_progress,
        });
    }
    if all_already_done {
        Ok(crate::storage::import::Outcome::AlreadyDone)
    } else {
        Ok(crate::storage::import::Outcome::Imported { found, written })
    }
}

#[cfg(feature = "legacy-import")]
fn backend_for_target_scope(
    backend: &Arc<dyn StorageBackend>,
    target_scope: &Scope,
    known: &Known,
    config: &crate::config::Config,
) -> anyhow::Result<Arc<dyn StorageBackend>> {
    if target_scope.is_local() || target_scope.is_cluster() {
        return Ok(Arc::clone(backend));
    }
    let parsed = target_scope.parsed()?;
    let crate::storage::Tenant::User(_) = parsed.tenant else {
        anyhow::bail!(
            "scope {} is not an admitted tenant scope",
            target_scope.as_str()
        );
    };
    let original_id = known
        .profiles
        .iter()
        .find(|id| {
            crate::storage::scope_for_profile(id).tenant_scope() == target_scope.tenant_scope()
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no known profile id maps to scope {}",
                target_scope.as_str()
            )
        })?;
    let Some(host) = crate::profiles::host::host() else {
        if crate::storage::layout::Target::from_config(config)
            .is_some_and(|target| target.user_id == *original_id)
        {
            return Ok(Arc::clone(backend));
        }
        anyhow::bail!(
            "cannot copy profile scope {} without its admitted profile host",
            target_scope.as_str()
        );
    };
    let profile_id = crate::profiles::ProfileId::parse(original_id).map_err(anyhow::Error::msg)?;
    let profile = host.get(&profile_id).ok_or_else(|| {
        anyhow::anyhow!(
            "profile {original_id} is not currently admitted on this host; refusing scope copy"
        )
    })?;
    host.storage_backend_for(&profile)?
        .ok_or_else(|| anyhow::anyhow!("profile {original_id} has no captured storage backend"))
}

#[cfg(feature = "legacy-import")]
fn scope_move_step(source_path: &Path, source_scope: &str) -> String {
    use sha2::Digest;
    let source_hash = sha2::Sha256::digest(source_path.as_os_str().as_encoded_bytes());
    let scope_hash = sha2::Sha256::digest(source_scope.as_bytes());
    format!(
        "scope_move/{}/{}",
        hex::encode(&source_hash[..8]),
        hex::encode(&scope_hash[..8])
    )
}

#[cfg(feature = "legacy-import")]
fn inventory_for_scope(
    inventory: &crate::storage::import::scope_move::LegacyScopeInventory,
    source_scope: &str,
) -> crate::storage::import::scope_move::LegacyScopeInventory {
    crate::storage::import::scope_move::LegacyScopeInventory {
        scopes: vec![source_scope.to_owned()],
        documents: inventory
            .documents
            .iter()
            .filter(|row| row.scope == source_scope)
            .cloned()
            .collect(),
        streams: inventory
            .streams
            .iter()
            .filter(|row| row.scope == source_scope)
            .cloned()
            .collect(),
        blobs: inventory
            .blobs
            .iter()
            .filter(|row| row.scope == source_scope)
            .cloned()
            .collect(),
    }
}

#[cfg(feature = "legacy-import")]
fn inventory_scope_counts(
    inventory: &crate::storage::import::scope_move::LegacyScopeInventory,
    source_scope: &str,
) -> usize {
    inventory
        .documents
        .iter()
        .filter(|row| row.scope == source_scope)
        .count()
        + inventory
            .streams
            .iter()
            .filter(|row| row.scope == source_scope)
            .map(|row| row.entries.len())
            .sum::<usize>()
        + inventory
            .blobs
            .iter()
            .filter(|row| row.scope == source_scope)
            .count()
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
            // `inventory` is read from a separate legacy database. A scope
            // already in the v2 grammar keeps its identity, but its rows still
            // need copying (and owner conversion) into the configured target.
            Mapping::V2 => Scope::new(source)?,
            Mapping::Unknown => {
                report.unknown.push(source.to_owned());
                continue;
            }
            Mapping::Move(scope) => scope,
        };
        let target = backend.for_scope(&target_scope)?;
        let docs = target.documents();
        for row in inventory.documents.iter().filter(|row| row.scope == source) {
            if row.meta.updated_ms != 0 {
                report.updated_times_not_restored += 1;
            }
            let source_record = Record {
                collection: row.collection.clone(),
                id: row.id.clone(),
                doc: row.body.clone(),
                created_ms: Some(row.meta.created_ms),
                schema: Some(row.meta.schema),
                expires_ms: row.meta.expires_ms,
                updated_ms: Some(row.meta.updated_ms),
            };
            let (records, specs) = upgrade_legacy_scope_document(source_record)?;
            for record in records {
                let spec = specs
                    .iter()
                    .find(|spec| spec.name == record.collection)
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "owner conversion emitted undeclared collection `{}`",
                            record.collection
                        )
                    })?;
                docs.ensure_collection(spec).await?;
                let mut options = WriteOptions::new();
                options.schema = record.schema.unwrap_or_default();
                options.created_ms = record.created_ms;
                options.expires_ms = record.expires_ms;
                match docs
                    .put_with(
                        &record.collection,
                        &record.id,
                        record.doc,
                        Precondition::Absent,
                        options,
                    )
                    .await
                {
                    Ok(_) => report.documents += 1,
                    Err(error)
                        if matches!(
                            error.kind(),
                            ErrorKind::Conflict | ErrorKind::AlreadyExists
                        ) => {}
                    Err(error) => return Err(error.into()),
                }
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

/// Applies the owning domain's pure schema conversion before a legacy row is
/// written into the target backend. Unrecognized collections retain their
/// original body and metadata.
#[cfg(feature = "legacy-import")]
fn upgrade_legacy_scope_document(
    record: crate::storage::import::Record,
) -> anyhow::Result<(Vec<crate::storage::import::Record>, Vec<CollectionSpec>)> {
    let collection = record.collection.as_str();
    let (records, specs) = match collection {
        "artifacts" | "artifact_args" | "artifact_versions" => (
            crate::artifacts::upgrade_records(vec![record])?,
            crate::artifacts::collection_specs(),
        ),
        "config" => (
            crate::config::schema::upgrade_config_records(vec![record])?,
            crate::config::schema::config_storage_collections(),
        ),
        "cost_records" => (
            crate::platform::cost::upgrade_records(vec![record])?,
            crate::platform::cost::collection_specs(),
        ),
        "profiles" => (
            crate::profiles::upgrade_records(vec![record])?,
            crate::profiles::collection_specs(),
        ),
        "leases" => (
            crate::profiles::upgrade_lease_records(vec![record])?,
            vec![CollectionSpec::new(crate::storage::lease::LEASE_COLLECTION)],
        ),
        "threads_transcript_keys" => (
            crate::threads::store::upgrade_records(vec![record])?,
            crate::threads::store::collection_specs(),
        ),
        "subagent_sessions" => (
            crate::agent::orchestration::subagent_sessions::upgrade_records(vec![record])?,
            crate::agent::orchestration::subagent_sessions::collection_specs(),
        ),
        _ => {
            let spec = CollectionSpec::new(&record.collection);
            (vec![record], vec![spec])
        }
    };
    Ok((records, specs))
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
