use super::*;
use crate::cron::client::testing as cron_store;
use crate::storage::{MemoryStorage, Scope};
use serde_json::Value;
use tempfile::TempDir;
use tinyflows_drivers::catalog::FlowCatalogDocuments;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/storage/import/fixtures/tinyflows")
}

fn expected() -> Value {
    serde_json::from_slice(&std::fs::read(fixtures().join("expected.json")).unwrap()).unwrap()
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// A scratch workspace holding the committed legacy stores.
fn workspace() -> (TempDir, Config) {
    let dir = TempDir::new().unwrap();
    copy_tree(&fixtures().join("workspace"), dir.path());
    let config = Config {
        workspace_dir: dir.path().to_path_buf(),
        ..Config::default()
    };
    (dir, config)
}

fn backend() -> Arc<dyn StorageBackend> {
    Arc::new(MemoryStorage::new())
}

fn documents(
    backend: &Arc<dyn StorageBackend>,
    scope: &Scope,
) -> Arc<dyn tinystoragedrivers::DocumentStore> {
    Arc::clone(backend.for_scope(scope).unwrap().documents())
}

fn written(outcome: Outcome) -> usize {
    match outcome {
        Outcome::Imported { written, .. } => written,
        other => panic!("expected an import, got {other:?}"),
    }
}

#[test]
fn a_job_policy_kept_in_the_side_table_lands_on_the_job() {
    let (dir, config) = workspace();
    let jobs_db = legacy_jobs_db(dir.path());
    let shell_id = {
        let expected = expected();
        expected["cron"]["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|job| job["job_type"] == "shell")
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string()
    };
    let conn = crate::storage::rusqlite::Connection::open(&jobs_db).unwrap();
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS cron_job_policies (
             job_id TEXT PRIMARY KEY, retries INTEGER, single_flight INTEGER NOT NULL DEFAULT 0)",
    )
    .unwrap();
    conn.execute(
        "INSERT INTO cron_job_policies (job_id, retries, single_flight) VALUES (?1, 0, 1)",
        [&shell_id],
    )
    .unwrap();
    drop(conn);

    let backend = backend();
    import_cron(&backend, &Scope::local(), &config, dir.path(), "").unwrap();

    let job = cron_store::get_job(&backend, &Scope::local(), &shell_id).unwrap();
    assert_eq!(job.policy.retries, Some(0));
    assert!(job.policy.single_flight);
}

#[test]
fn an_import_runs_once_and_does_not_resurrect_a_deleted_flow() {
    let (_dir, config) = workspace();
    let backend = backend();
    assert!(written(import_flows(&backend, &Scope::local(), &config).unwrap()) > 0);

    let catalog = FlowCatalogDocuments::new(documents(&backend, &Scope::local()));
    let after = {
        let catalog = catalog.clone();
        crate::storage::block_on_anyhow(async move {
            catalog.remove_flow("flow-alpha").await?;
            Ok(catalog.list_flows().await?.0.len())
        })
        .unwrap()
    };
    assert_eq!(after, 1);

    let again = import_flows(&backend, &Scope::local(), &config).unwrap();
    assert_eq!(again, Outcome::AlreadyDone);
    let still =
        crate::storage::block_on_anyhow(async move { Ok(catalog.list_flows().await?.0.len()) })
            .unwrap();
    assert_eq!(still, 1, "the legacy flow came back");
}

#[test]
fn a_record_the_backend_already_holds_is_never_overwritten() {
    let (_dir, config) = workspace();
    let backend = backend();
    let catalog = FlowCatalogDocuments::new(documents(&backend, &Scope::local()));
    import_flows(&backend, &Scope::local(), &config).unwrap();
    let flow = {
        let catalog = catalog.clone();
        crate::storage::block_on_anyhow(async move { catalog.get_flow("flow-alpha").await })
            .unwrap()
    };
    let mut changed = flow.expect("imported");
    changed.name = "renamed after the import".into();
    let renamed = changed.name.clone();
    {
        let catalog = catalog.clone();
        crate::storage::block_on_anyhow(async move { catalog.upsert_flow(&changed).await })
            .unwrap();
    }

    // A second workspace marker (a different workspace on a shared backend)
    // runs the import again over the same backend.
    let (_other, other_config) = workspace();
    assert_eq!(
        written(import_flows(&backend, &Scope::local(), &other_config).unwrap()),
        0,
        "records already present are counted as existing, not written"
    );
    let name = crate::storage::block_on_anyhow(async move {
        Ok(catalog.get_flow("flow-alpha").await?.unwrap().name)
    })
    .unwrap();
    assert_eq!(name, renamed);
}

#[test]
fn a_workspace_without_legacy_files_imports_nothing_and_leaves_no_marker() {
    let dir = TempDir::new().unwrap();
    let config = Config {
        workspace_dir: dir.path().to_path_buf(),
        ..Config::default()
    };
    let backend = backend();
    assert_eq!(
        import_flows(&backend, &Scope::local(), &config).unwrap(),
        Outcome::NoSource
    );
    assert_eq!(
        import_cron(&backend, &Scope::local(), &config, dir.path(), "").unwrap(),
        Outcome::NoSource
    );
    // Both would be `AlreadyDone` had a marker been written.
    let (_ws, with_files) = workspace();
    let with_files = Config {
        workspace_dir: dir.path().to_path_buf(),
        ..with_files
    };
    copy_tree(&fixtures().join("workspace"), dir.path());
    assert!(written(import_flows(&backend, &Scope::local(), &with_files).unwrap()) > 0);
}

#[test]
fn an_agents_cron_database_lands_in_that_agents_scope() {
    let (dir, config) = workspace();
    let agent_dir = dir.path().join("agents").join("alpha");
    std::fs::create_dir_all(agent_dir.join("cron")).unwrap();
    std::fs::copy(legacy_jobs_db(dir.path()), legacy_jobs_db(&agent_dir)).unwrap();
    let backend = backend();

    let reports = run_with(&backend, &config);
    assert!(
        reports.iter().all(|report| report.outcome.is_ok()),
        "{reports:?}"
    );

    let count = |scope: Scope| cron_store::list_jobs(&backend, &scope).unwrap().len();
    assert_eq!(count(Scope::local()), 3);
    assert_eq!(count(crate::storage::scope_for_agent("alpha")), 3);
    assert_eq!(count(crate::storage::scope_for_agent("beta")), 0);
}

#[test]
fn a_corrupt_legacy_file_fails_without_a_marker_and_is_retried() {
    let (dir, config) = workspace();
    let jobs_db = legacy_jobs_db(dir.path());
    let good = std::fs::read(&jobs_db).unwrap();
    std::fs::write(&jobs_db, b"this is not a database").unwrap();
    let backend = backend();

    assert!(import_cron(&backend, &Scope::local(), &config, dir.path(), "").is_err());

    std::fs::write(&jobs_db, good).unwrap();
    let retried = import_cron(&backend, &Scope::local(), &config, dir.path(), "").unwrap();
    assert_eq!(written(retried), 7);
}

#[test]
fn a_run_that_skipped_records_leaves_no_marker_so_it_is_retried() {
    let dir = TempDir::new().unwrap();
    let backend = backend();
    let skipped = marked(&backend, dir.path(), "skipdomain", || Ok(Some((3, 2, 1)))).unwrap();
    assert_eq!(
        skipped,
        Outcome::Imported {
            found: 3,
            written: 2
        }
    );
    // No marker: the next boot runs the body again instead of `AlreadyDone`.
    let again = marked(&backend, dir.path(), "skipdomain", || Ok(Some((3, 3, 0)))).unwrap();
    assert_eq!(
        again,
        Outcome::Imported {
            found: 3,
            written: 3
        }
    );
    let done = marked(&backend, dir.path(), "skipdomain", || Ok(Some((3, 3, 0)))).unwrap();
    assert_eq!(done, Outcome::AlreadyDone);
}
