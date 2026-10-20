//! The tinyflows stores: cron jobs (with their run history and policies), the
//! flow catalog with its drafts, and the flow checkpoints.
//!
//! tinyflows used to own these as SQLite files and JSON drafts under the
//! workspace (`cron/jobs.db`, `agents/<id>/cron/jobs.db`, `flows/flows.db`,
//! `flows/drafts/*.json`, `flows/checkpoints.db`). It no longer has a storage
//! backend of its own, so the host copies what those files hold into the
//! port-backed stores with `tinyflows-import`, which keeps every record under
//! the id it had, writes only what is absent, and reads copies of the files
//! (the originals are never opened for writing).
//!
//! Each store has a marker like the small stores' (see the parent module), so
//! it is copied once: a flow or job deleted afterwards is not resurrected from
//! the untouched legacy file. A failed import leaves no marker and is retried.
//!
//! Where the records land:
//!
//! - the workspace's `cron/jobs.db` and `flows/` go to [`Scope::local`], the
//!   operator's scope;
//! - an embedded agent's `agents/<id>/cron/jobs.db` goes to that agent's scope
//!   ([`crate::storage::scope_for_agent`]), the one its cron calls resolve to.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Context, Result};

use super::{marker_spec, marker_step, Outcome, Report, MARKER_OWNER};
use crate::config::Config;
use crate::storage::documents::Repo;
use crate::storage::import_state;
use crate::storage::{Scope, StorageBackend};

/// The marker name of the flow catalog (flows, revisions, runs, state,
/// suggestions, drafts) and checkpoints.
pub const FLOWS_DOMAIN: &str = "flows";

/// The marker name of the workspace's own cron store; an agent's is
/// `cron/<agent id>`.
pub const CRON_DOMAIN: &str = "cron";

/// The legacy cron database under `scope_dir` (the workspace, or an agent's
/// directory under it).
pub fn legacy_jobs_db(scope_dir: &Path) -> PathBuf {
    scope_dir.join("cron").join("jobs.db")
}

/// Runs `body` unless the marker for `domain` says it already finished, then
/// records the marker. `body` returns `None` when there is nothing to import
/// (no marker is written then, so a file that appears later is still seen) and
/// otherwise `(found, written, skipped)`. A run that skipped records writes no
/// marker, so the import is retried at the next boot once the operator has
/// repaired them (records already written are counted, not duplicated).
pub(super) fn marked(
    backend: &Arc<dyn StorageBackend>,
    workspace: &Path,
    domain: &str,
    body: impl FnOnce() -> Result<Option<(usize, usize, usize)>>,
) -> Result<Outcome> {
    let step = marker_step(domain, workspace);
    let markers = Repo::on(
        backend,
        &crate::storage::tenant_scope(None),
        "storage::import",
        marker_spec,
    )?;
    if import_state::is_done(&markers, MARKER_OWNER, &step)? {
        return Ok(Outcome::AlreadyDone);
    }
    let claimed = import_state::with_claim(&markers, MARKER_OWNER, &step, || {
        if import_state::is_done(&markers, MARKER_OWNER, &step)? {
            return Ok(Outcome::AlreadyDone);
        }
        let Some((found, written, skipped)) = body()? else {
            return Ok(Outcome::NoSource);
        };
        if skipped > 0 {
            tracing::warn!(
                target: "openhuman::storage_import",
                domain,
                skipped,
                "[storage::import] legacy records were skipped; the import stays open for retry"
            );
            return Ok(Outcome::Incomplete {
                found,
                written,
                skipped,
            });
        }
        import_state::set_done(&markers, MARKER_OWNER, &step)?;
        Ok(Outcome::Imported { found, written })
    })?;
    Ok(claimed.unwrap_or(Outcome::InProgress))
}

#[cfg(feature = "flows")]
#[derive(Debug, Default, Clone, Copy)]
struct ImportTotals {
    found: usize,
    written: usize,
    skipped: usize,
}

#[cfg(feature = "flows")]
impl ImportTotals {
    fn add(&mut self, response: tinyflows_bus::catalog::ImportLegacyBatchResponse) {
        for tally in response.tallies {
            let written = tally.written as usize;
            let existing = tally.existing as usize;
            let invalid = tally.invalid as usize;
            self.found += written + existing + invalid;
            self.written += written;
            self.skipped += invalid;
        }
        for problem in response.problems {
            tracing::warn!(
                target: "openhuman::storage_import",
                domain = FLOWS_DOMAIN,
                kind = ?problem.kind,
                identity = ?problem.identity,
                message = %problem.message,
                "[storage::import] a legacy flow record was skipped"
            );
        }
    }
}

#[cfg(feature = "flows")]
async fn send_legacy_batch(
    proxy: &tinybus::Proxy,
    storage: &tinyflows_bus::catalog::CatalogStorage,
    records: Vec<tinyflows_bus::catalog::LegacyImportRecord>,
) -> Result<()> {
    use tinyflows_bus::catalog::{self, ImportLegacyBatchRequest, ImportLegacyBatchResponse};

    let request = ImportLegacyBatchRequest {
        storage: storage.clone(),
        records,
    };
    let response: ImportLegacyBatchResponse = proxy
        .call(catalog::members::IMPORT_LEGACY_BATCH, &(request,))
        .await
        .context("send a bounded legacy flow batch to TinyFlows")?;
    Ok(())
}

#[cfg(feature = "flows")]
fn flow_sources_present(dir: &Path) -> bool {
    ["flows.db", "checkpoints.db", "checkpoints", "drafts"]
        .iter()
        .any(|name| dir.join(name).exists())
}

/// Copies flow catalog and checkpoint records through TinyFlows' bounded
/// Catalog import contract. Every callback batch completes before the next is
/// read, and the marker is written only after both readers finish cleanly.
///
/// # Errors
///
/// When a legacy file cannot be read, a bus call fails, or the owner store
/// rejects the batch. The legacy files are untouched either way.
#[cfg(feature = "flows")]
pub fn import_flows(
    backend: &Arc<dyn StorageBackend>,
    scope: &Scope,
    config: &Config,
) -> Result<Outcome> {
    let backend_for_migration = Arc::clone(backend);
    let scope_for_migration = scope.clone();
    let config_for_migration = config.clone();
    crate::storage::block_on_anyhow(async move {
        migrate_catalog_storage(
            &backend_for_migration,
            &config_for_migration,
            &scope_for_migration,
        )
        .await
    })?;
    record_catalog_migration(config, scope, Ok(()));
    import_flows_after_migration(backend, scope, config)
}

#[cfg(feature = "flows")]
fn import_flows_after_migration(
    backend: &Arc<dyn StorageBackend>,
    scope: &Scope,
    config: &Config,
) -> Result<Outcome> {
    use tinyflows_bus::catalog::{
        self, ImportLegacyBatchRequest, ImportLegacyBatchResponse, LegacyImportRecord,
    };

    let dir = config.workspace_dir.join("flows");
    marked(backend, &config.workspace_dir, FLOWS_DOMAIN, || {
        if !flow_sources_present(&dir) {
            return Ok(None);
        }
        let backend = Arc::clone(backend);
        let scope = scope.clone();
        let config = config.clone();
        let totals = Arc::new(Mutex::new(ImportTotals::default()));
        crate::storage::block_on_anyhow(async move {
            let registration = crate::flows::catalog_storage::register_backend_scope(
                &config,
                backend,
                scope,
            )
            .await
            .map_err(anyhow::Error::msg)?;
            let storage = registration.storage();
            let proxy = crate::modules::tinyflows::catalog_proxy_for_migration(&config)
                .await
                .map_err(anyhow::Error::msg)?;

            let flow_proxy = proxy.clone();
            let flow_storage = storage.clone();
            let flow_totals = Arc::clone(&totals);
            tinyflows_import::read_legacy_flow_catalog(&dir, move |records| {
                let proxy = flow_proxy.clone();
                let storage = flow_storage.clone();
                let totals = Arc::clone(&flow_totals);
                async move {
                    let request = ImportLegacyBatchRequest { storage, records };
                    let response: ImportLegacyBatchResponse = proxy
                        .call(catalog::members::IMPORT_LEGACY_BATCH, &(request,))
                        .await
                        .context("send a bounded legacy flow batch to TinyFlows")?;
                    totals
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .add(response);
                    Ok(())
                }
            })
            .await
            .context("read the legacy flow catalog")?;

            let checkpoint_proxy = proxy.clone();
            let checkpoint_storage = storage.clone();
            let checkpoint_totals = Arc::clone(&totals);
            tinyflows_import::read_legacy_checkpoint_batches(
                &dir.join("checkpoints.db"),
                &dir.join("checkpoints"),
                move |records: Vec<LegacyImportRecord>| {
                    let proxy = checkpoint_proxy.clone();
                    let storage = checkpoint_storage.clone();
                    let totals = Arc::clone(&checkpoint_totals);
                    async move {
                        let request = ImportLegacyBatchRequest { storage, records };
                        let response: ImportLegacyBatchResponse = proxy
                            .call(catalog::members::IMPORT_LEGACY_BATCH, &(request,))
                            .await
                            .context("send a bounded checkpoint batch to TinyFlows")?;
                        totals
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .add(response);
                        Ok(())
                    }
                },
            )
            .await
            .context("read the legacy flow checkpoints")?;

            let totals = *totals
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            Ok(Some((totals.found, totals.written, totals.skipped)))
        })
    })
}

/// Copies the cron jobs, runs and policies of `scope_dir/cron/jobs.db` into
/// `backend` under `scope`. `label` names the store in its marker: empty for
/// the workspace's own, the agent id for an agent's.
///
/// # Errors
///
/// When the legacy file exists but cannot be read, or the backend fails. The
/// legacy file is untouched either way.
pub fn import_cron(
    backend: &Arc<dyn StorageBackend>,
    scope: &Scope,
    config: &Config,
    scope_dir: &Path,
    label: &str,
) -> Result<Outcome> {
    use crate::cron::client::store_in;
    use crate::cron::documents_port::DocScope;
    use tinyflows_bus::schedule::{ImportedJob, StoreOperation};

    let domain = if label.is_empty() {
        CRON_DOMAIN.to_string()
    } else {
        format!("{CRON_DOMAIN}/{label}")
    };
    let jobs_db = legacy_jobs_db(scope_dir);
    marked(backend, &config.workspace_dir, &domain, || {
        if !jobs_db.is_file() {
            return Ok(None);
        }
        // The legacy file is decoded here (`tinyflows-import`'s reader, bus
        // records only) and replayed through the module's `import_job` /
        // `import_run` store operations, which write only what is absent and
        // file a flow-schedule job under the id its flow derives.
        let doc_scope = DocScope::on_backend(config, Arc::clone(backend), scope.clone());
        let config = config.clone();
        let report = crate::storage::block_on_anyhow({
            let jobs_db = jobs_db.clone();
            async move {
                let legacy = tinyflows_import::read_legacy_cron(&jobs_db)
                    .await
                    .context("read the legacy cron database")?;
                let mut report = legacy.report;
                let mut filed_as = std::collections::HashMap::new();
                for job in legacy.jobs {
                    let source_id = job.id.clone();
                    let imported: ImportedJob =
                        store_in(&config, &doc_scope, StoreOperation::ImportJob { job })
                            .await
                            .context("import a cron job")?;
                    report.wrote("jobs", imported.written);
                    filed_as.insert(source_id, imported.id);
                }
                for mut run in legacy.runs {
                    let Some(id) = filed_as.get(&run.job_id) else {
                        report.invalid(
                            "runs",
                            format!("run {}: job '{}' was not imported", run.id, run.job_id),
                        );
                        continue;
                    };
                    run.job_id = id.clone();
                    let written: bool =
                        store_in(&config, &doc_scope, StoreOperation::ImportRun { run })
                            .await
                            .context("import a cron run")?;
                    report.wrote("runs", written);
                }
                Ok(report)
            }
        })?;
        Ok(Some(counts(&domain, &report)))
    })
}

/// Every agent directory under the workspace that kept its own cron database,
/// as `(agent id, directory)`.
fn agent_cron_dirs(workspace: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(workspace.join("agents")) else {
        return Vec::new();
    };
    let mut dirs: Vec<(String, PathBuf)> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| legacy_jobs_db(&entry.path()).is_file())
        .filter_map(|entry| Some((entry.file_name().into_string().ok()?, entry.path())))
        .collect();
    dirs.sort();
    dirs
}

/// Imports every tinyflows legacy store of `config`'s workspace into
/// `backend`: the flow catalog and checkpoints (only in a build with `flows`),
/// the workspace's cron jobs and each agent's. One [`Report`] per store; a
/// failure in one does not stop the rest. The cron importer needs only the
/// `schedule` reader, so a build without `flows` still keeps its cron jobs.
pub fn run_with(backend: &Arc<dyn StorageBackend>, config: &Config) -> Vec<Report> {
    let mut reports = Vec::new();
    #[cfg(feature = "flows")]
    reports.push(finish(
        FLOWS_DOMAIN,
        import_flows(backend, &crate::storage::tenant_scope(None), config),
    ));
    #[cfg(not(feature = "flows"))]
    if config.workspace_dir.join("flows").exists() {
        tracing::warn!(
            target: "openhuman::storage_import",
            "[storage::import] built without `flows`; the legacy flow catalog is not imported"
        );
    }
    reports.push(finish(
        CRON_DOMAIN,
        import_cron(
            backend,
            &crate::storage::tenant_scope(None),
            config,
            &config.workspace_dir,
            "",
        ),
    ));
    for (agent, dir) in agent_cron_dirs(&config.workspace_dir) {
        let scope = crate::storage::scope_for_agent(&agent);
        reports.push(finish(
            CRON_DOMAIN,
            import_cron(backend, &scope, config, &dir, &agent),
        ));
    }
    reports
}

pub(super) fn finish(domain: &'static str, outcome: Result<Outcome>) -> Report {
    match &outcome {
        Ok(Outcome::Imported { found, written }) => tracing::info!(
            target: "openhuman::storage_import",
            domain,
            found,
            written,
            "[storage::import] imported the legacy store; the files were not modified"
        ),
        Ok(Outcome::Incomplete {
            found,
            written,
            skipped,
        }) => tracing::warn!(
            target: "openhuman::storage_import",
            domain,
            found,
            written,
            skipped,
            "[storage::import] some legacy records were skipped; the import remains open for retry"
        ),
        Ok(Outcome::AlreadyDone) => tracing::debug!(
            target: "openhuman::storage_import",
            domain,
            "[storage::import] already imported"
        ),
        Ok(Outcome::InProgress) => tracing::debug!(
            target: "openhuman::storage_import",
            domain,
            "[storage::import] another process is importing; skipped"
        ),
        Ok(Outcome::NoSource) => tracing::debug!(
            target: "openhuman::storage_import",
            domain,
            "[storage::import] no legacy file"
        ),
        Err(error) => tracing::error!(
            target: "openhuman::storage_import",
            domain,
            error = %format!("{error:#}"),
            "[storage::import] import failed; the legacy files are untouched and it is retried on the next boot"
        ),
    }
    Report { domain, outcome }
}

#[cfg(all(test, feature = "flows"))]
#[path = "tinyflows_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "tinyflows_cron_tests.rs"]
mod cron_tests;
