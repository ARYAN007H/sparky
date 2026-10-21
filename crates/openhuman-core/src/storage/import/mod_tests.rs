#![cfg(feature = "storage-sqlite")]

use super::*;
use crate::storage::DocumentStoreExt as _;
use crate::storage::Scope;
use crate::storage::rusqlite;
use crate::storage::test_support::{SqliteBackend, sqlite_backend};
use std::collections::BTreeMap;
use std::ops::Deref;
use tempfile::TempDir;
use tinystoragedrivers::Query;

struct TestBackend(SqliteBackend);

impl Deref for TestBackend {
    type Target = Arc<dyn StorageBackend>;

    fn deref(&self) -> &Self::Target {
        &self.0.backend
    }
}

fn backend() -> TestBackend {
    TestBackend(sqlite_backend().unwrap())
}

fn workspace() -> (TempDir, Config) {
    let dir = TempDir::new().unwrap();
    let config = Config {
        workspace_dir: dir.path().to_path_buf(),
        ..Config::default()
    };
    (dir, config)
}

/// Every file under `root` with a digest of its bytes. SQLite's `-shm`
/// index and an empty `-wal` are connection bookkeeping a read-only open may
/// create; they hold no data and are left out.
fn tree(root: &Path) -> BTreeMap<String, String> {
    fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<String, String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, root, out);
            } else {
                let bytes = std::fs::read(&path).unwrap();
                let name = path.to_string_lossy();
                if name.ends_with("-shm") || (name.ends_with("-wal") && bytes.is_empty()) {
                    continue;
                }
                let digest = sha2::Sha256::digest(bytes);
                out.insert(
                    path.strip_prefix(root).unwrap().display().to_string(),
                    hex::encode(digest),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

/// All documents of `collection` in the backend's local scope, by id.
fn dump(
    backend: &Arc<dyn StorageBackend>,
    source: &Source,
    collection: &str,
) -> BTreeMap<String, serde_json::Value> {
    let docs = Arc::clone(backend.for_scope(&Scope::local()).unwrap().documents());
    let _ = source;
    let collection = collection.to_string();
    crate::storage::block_on(async move {
        docs.ensure_collection(&tinystoragedrivers::CollectionSpec::new(&collection))
            .await?;
        let found = docs.query_all(&collection, &Query::all()).await?;
        Ok(found.into_iter().map(|d| (d.id, d.doc)).collect())
    })
    .unwrap()
}

fn imported(report: Vec<Report>) -> (usize, usize) {
    assert_eq!(report.len(), 1);
    match report.into_iter().next().unwrap().outcome.unwrap() {
        Outcome::Imported { found, written } => (found, written),
        other => panic!("expected an import, got {other:?}"),
    }
}

fn outcome(report: Vec<Report>) -> Outcome {
    report.into_iter().next().unwrap().outcome.unwrap()
}

fn devices() -> Source {
    crate::security::devices::import_source()
}

const DEVICES_DDL: &str = "CREATE TABLE IF NOT EXISTS paired_devices (
     channel_id TEXT PRIMARY KEY, label TEXT NOT NULL, device_pubkey TEXT NOT NULL,
     core_session_token_hash TEXT NOT NULL, shared_secret_encrypted BLOB,
     created_at TEXT NOT NULL, last_seen_at TEXT, revoked INTEGER NOT NULL DEFAULT 0
 );";

/// Writes a `devices/devices.db` row the way the retired store did.
fn legacy_device(config: &Config, id: &str, label: &str, revoked: bool) {
    let db = config.workspace_dir.join(devices().db);
    std::fs::create_dir_all(db.parent().unwrap()).unwrap();
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute_batch(DEVICES_DDL).unwrap();
    conn.execute(
        "INSERT INTO paired_devices
            (channel_id, label, device_pubkey, core_session_token_hash, created_at, revoked)
         VALUES (?1, ?2, 'pk', 'hash', '2026-01-01T00:00:00+00:00', ?3)",
        rusqlite::params![id, label, i64::from(revoked)],
    )
    .unwrap();
}

#[test]
fn devices_move_into_the_backend_and_the_legacy_file_is_untouched() {
    let (dir, config) = workspace();
    legacy_device(&config, "chan-a", "Phone", false);
    legacy_device(&config, "chan-b", "Tablet", false);
    legacy_device(&config, "chan-c", "Old laptop", true);
    let before = tree(dir.path());
    let backend = backend();
    let source = devices();

    let (found, written) = imported(run_with(&backend, &config, &[source]));

    assert_eq!((found, written), (3, 3));
    let found = dump(&backend, &source, "devices.paired");
    assert_eq!(found["chan-a"]["label"], "Phone");
    assert_eq!(found["chan-a"]["core_session_token_hash"], "hash");
    assert_eq!(found["chan-a"]["revoked"], false);
    assert_eq!(found["chan-c"]["revoked"], true);
    assert_eq!(
        tree(dir.path()),
        before,
        "no legacy file was modified, created or removed"
    );
}

#[test]
fn a_finished_import_does_not_run_again_or_resurrect_deleted_records() {
    let (_dir, config) = workspace();
    legacy_device(&config, "chan-a", "Phone", false);
    legacy_device(&config, "chan-b", "Tablet", false);
    let backend = backend();
    let source = devices();
    imported(run_with(&backend, &config, &[source]));
    assert!(is_done(&backend, &config, &source).unwrap());

    // The user removes a record; the untouched legacy file still has it.
    let docs = Arc::clone(backend.for_scope(&Scope::local()).unwrap().documents());
    crate::storage::block_on(async move {
        docs.delete("devices.paired", "chan-b", Precondition::None)
            .await
    })
    .unwrap();

    assert_eq!(
        outcome(run_with(&backend, &config, &[source])),
        Outcome::AlreadyDone
    );
    assert!(!dump(&backend, &source, "devices.paired").contains_key("chan-b"));
}

#[test]
fn an_import_claimed_by_another_process_waits_for_it_and_does_not_repeat_it() {
    let (_dir, config) = workspace();
    legacy_device(&config, "chan-a", "Phone", false);
    let backend = backend();
    let source = devices();

    // Another process holds the claim for this domain and workspace, and
    // finishes (marked done, claim released) shortly after we start.
    let markers = Repo::on(&backend, &Scope::local(), "storage::import", marker_spec).unwrap();
    let step = marker_step(source.domain, &config.workspace_dir);
    let (held_tx, held_rx) = std::sync::mpsc::channel::<()>();
    let finisher = {
        let (markers, step) = (markers.clone(), step.clone());
        std::thread::spawn(move || {
            crate::storage::import_state::with_claim(
                &markers,
                MARKER_OWNER,
                &step.clone(),
                |condition| {
                    held_tx.send(()).unwrap();
                    std::thread::sleep(std::time::Duration::from_millis(400));
                    let claimed_markers = markers.conditioned(condition)?;
                    crate::storage::import_state::set_done(&claimed_markers, MARKER_OWNER, &step)
                },
            )
            .unwrap()
            .expect("the first claimant runs");
        })
    };
    held_rx.recv().unwrap();

    let started = std::time::Instant::now();
    assert_eq!(
        outcome(run_with(&backend, &config, &[source])),
        Outcome::AlreadyDone
    );
    assert!(
        started.elapsed() >= std::time::Duration::from_millis(300),
        "the loser must not proceed while the claim is held"
    );
    finisher.join().unwrap();
    assert!(dump(&backend, &source, "devices.paired").is_empty());
}

#[test]
fn a_marker_row_an_older_build_wrote_counts_as_a_finished_import() {
    let (_dir, config) = workspace();
    legacy_device(&config, "chan-a", "Phone", false);
    let backend = backend();
    let source = devices();
    let marker = marker_id(source.domain, &config.workspace_dir);
    let scoped = backend.for_scope(&Scope::local()).unwrap();
    let docs = Arc::clone(scoped.documents());
    crate::storage::block_on(async move {
        docs.ensure_collection(&tinystoragedrivers::CollectionSpec::new(MARKER_KIND))
            .await?;
        docs.put(
            MARKER_KIND,
            &marker,
            serde_json::json!({ "version": 1, "found": 1, "written": 1 }),
            Precondition::Absent,
        )
        .await
    })
    .unwrap();
    // Boot moves the marker row into `tsd.migrations`.
    let owned = scoped.clone();
    crate::storage::block_on(async move {
        crate::storage::import_state::migrate_legacy_markers(&owned).await
    })
    .unwrap();

    assert!(is_done(&backend, &config, &source).unwrap());
    assert_eq!(
        outcome(run_with(&backend, &config, &[source])),
        Outcome::AlreadyDone
    );
    assert!(dump(&backend, &source, "devices.paired").is_empty());
}

#[cfg(feature = "storage-sqlite")]
#[test]
fn historical_profile_fixture_boots_into_typed_owner_records_without_resurrection() {
    use crate::profiles::{ProfileId, ProfileRegistry};
    use crate::storage::layout::{Target, legacy};
    use crate::storage::{self, Scope, scope_for_profile};
    use sha2::Digest;
    use tinystoragedrivers::Query;

    let root = TempDir::new().unwrap();
    let target = Target::new(root.path(), "u1");
    std::fs::create_dir_all(target.workspace()).unwrap();
    std::fs::create_dir_all(&target.profile_dir).unwrap();
    let config_path = target.profile_dir.join("config.toml");
    std::fs::write(&config_path, "# legacy profile fixture\n").unwrap();

    // This checked-in file was produced by the historical public TSD v1
    // writer. Copy it to the same profile workspace path the boot importer
    // selects; do not synthesize rows in the test.
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/storage/import/fixtures/legacy-v1-profile/storage.db");
    let source_db = legacy::v1_db(&target);
    std::fs::create_dir_all(source_db.parent().unwrap()).unwrap();
    std::fs::copy(&fixture, &source_db).unwrap();
    let source_hash = sha2::Sha256::digest(std::fs::read(&source_db).unwrap());

    let destination_url = format!("sqlite:{}", target.storage_db().display());
    let destination =
        crate::storage::test_support::sqlite_backend_in(root, destination_url.clone()).unwrap();
    let backend = Arc::clone(&destination.backend);
    let _installed =
        crate::storage::test_support::install_for_test(&destination_url, Arc::clone(&backend));
    let config = Config {
        config_path,
        workspace_dir: target.workspace(),
        storage: crate::config::StorageConfig {
            url: Some(destination_url),
            ..Default::default()
        },
        ..Config::default()
    };

    // The boot pipeline includes scope_move. The other importers have no
    // source files in this fixture; this proof covers only historical TSD
    // documents and the schema owners they enter through scope_move.
    let first = run_all(&backend, &config);
    let moved = first
        .iter()
        .find(|report| report.domain == crate::storage::scope_move::IMPORT_DOMAIN)
        .expect("boot importer includes legacy scope movement");
    assert_eq!(
        moved.outcome.as_ref().unwrap(),
        &Outcome::Imported {
            found: 4,
            written: 4
        }
    );

    let profile_scope = backend.for_scope(&scope_for_profile("u1")).unwrap();
    let profile_docs = Arc::clone(profile_scope.documents());
    let (cost, artifact, args) = storage::block_on(async move {
        let cost = profile_docs
            .query_all("cost.records", &Query::all())
            .await?;
        let artifact = profile_docs
            .query_all("artifacts.records", &Query::all())
            .await?;
        let args = profile_docs
            .query_all("artifacts.args", &Query::all())
            .await?;
        Ok::<_, crate::storage::StorageError>((cost, artifact, args))
    })
    .unwrap();

    // Reconstitute the typed owner records from their v2 stored bodies. These
    // are the same timestamp restorations used by cost::storage_schema and
    // artifacts::storage_schema before their serde model decoders run.
    assert_eq!(cost.len(), 1);
    let mut cost_body = cost[0].doc.clone();
    let cost_ms = cost_body["ts_ms"].as_i64().unwrap();
    cost_body["usage"]["timestamp"] = serde_json::json!(
        chrono::DateTime::from_timestamp_millis(cost_ms)
            .unwrap()
            .to_rfc3339()
    );
    let cost_record: crate::platform::cost::types::CostRecord =
        serde_json::from_value(cost_body).unwrap();
    assert_eq!(cost_record.id, "cost-0001");
    assert_eq!(cost_record.session_id, "session-fixture");
    assert_eq!(cost_record.usage.model, "fixture-model");
    assert_eq!(cost_record.usage.input_tokens, 12);
    assert_eq!(cost_record.usage.output_tokens, 3);
    assert_eq!(cost_record.usage.timestamp.timestamp_millis(), cost_ms);

    assert_eq!(artifact.len(), 1);
    let mut artifact_body = artifact[0].doc.clone();
    let created_ms = artifact_body["created_ms"].as_i64().unwrap();
    artifact_body["created_at"] = serde_json::json!(
        chrono::DateTime::from_timestamp_millis(created_ms)
            .unwrap()
            .to_rfc3339()
    );
    let artifact_meta: crate::artifacts::types::ArtifactMeta =
        serde_json::from_value(artifact_body).unwrap();
    assert_eq!(artifact_meta.id, "artifact-1");
    assert_eq!(artifact_meta.title, "Fixture plan");
    assert_eq!(artifact_meta.thread_id.as_deref(), Some("thread-1"));
    assert_eq!(artifact_meta.created_at.timestamp_millis(), created_ms);
    assert_eq!(args.len(), 1);
    assert_eq!(
        args[0].doc,
        serde_json::json!({"title":"Fixture plan", "format":"markdown"})
    );

    let registry = ProfileRegistry::cluster(backend.as_ref()).unwrap();
    let profile = storage::block_on(registry.get(&ProfileId::parse("u1").unwrap()))
        .unwrap()
        .unwrap();
    assert_eq!(profile.profile_id.as_str(), "u1");
    assert_eq!(profile.created_at, 1_738_548_306);
    assert_eq!(profile.layout_version, 1);

    assert_eq!(
        sha2::Sha256::digest(std::fs::read(&source_db).unwrap()),
        source_hash,
        "boot import must leave the v1 source bytes unchanged"
    );

    let delete_scope = backend.for_scope(&Scope::new("u:u1").unwrap()).unwrap();
    let docs = Arc::clone(delete_scope.documents());
    storage::block_on(async move {
        docs.delete("cost.records", &cost[0].id, Precondition::None)
            .await
    })
    .unwrap();

    let second = run_all(&backend, &config);
    let moved = second
        .iter()
        .find(|report| report.domain == crate::storage::scope_move::IMPORT_DOMAIN)
        .unwrap();
    assert_eq!(moved.outcome.as_ref().unwrap(), &Outcome::AlreadyDone);
    let docs = Arc::clone(
        backend
            .for_scope(&scope_for_profile("u1"))
            .unwrap()
            .documents(),
    );
    let remaining =
        storage::block_on(async move { docs.query_all("cost.records", &Query::all()).await })
            .unwrap();
    assert!(
        remaining.is_empty(),
        "a deleted imported row must not return"
    );
    assert_eq!(
        sha2::Sha256::digest(std::fs::read(&source_db).unwrap()),
        source_hash,
        "repeat boot import must still leave the v1 source bytes unchanged"
    );
}

#[test]
fn processes_importing_one_workspace_at_once_import_it_once() {
    let (_dir, config) = workspace();
    for n in 0..20 {
        legacy_device(&config, &format!("chan-{n}"), "Phone", false);
    }
    let backend = backend();
    let source = devices();
    let barrier = Arc::new(std::sync::Barrier::new(6));
    let handles: Vec<_> = (0..6)
        .map(|_| {
            let (backend, config, barrier) =
                (Arc::clone(&backend), config.clone(), Arc::clone(&barrier));
            std::thread::spawn(move || {
                barrier.wait();
                outcome(run_with(&backend, &config, &[source]))
            })
        })
        .collect();
    let outcomes: Vec<Outcome> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    let imports = outcomes
        .iter()
        .filter(|o| matches!(o, Outcome::Imported { .. }))
        .count();
    assert_eq!(imports, 1, "exactly one process imports: {outcomes:?}");
    assert!(
        outcomes
            .iter()
            .all(|o| !matches!(o, Outcome::Imported { .. })
                || matches!(
                    o,
                    Outcome::Imported {
                        found: 20,
                        written: 20
                    }
                ))
    );
    assert!(is_done(&backend, &config, &source).unwrap());
    assert_eq!(dump(&backend, &source, "devices.paired").len(), 20);
}

#[test]
fn an_existing_record_is_never_overwritten() {
    let (_dir, config) = workspace();
    legacy_device(&config, "chan-a", "Phone", false);
    legacy_device(&config, "chan-b", "Tablet", false);
    let backend = backend();
    let source = devices();
    // A record the new code already wrote (a resumed import).
    let docs = Arc::clone(backend.for_scope(&Scope::local()).unwrap().documents());
    crate::storage::block_on(async move {
        docs.ensure_collection(&tinystoragedrivers::CollectionSpec::new("devices.paired"))
            .await?;
        docs.put(
            "devices.paired",
            "chan-a",
            serde_json::json!({ "label": "newer" }),
            Precondition::Absent,
        )
        .await
    })
    .unwrap();

    let (found, written) = imported(run_with(&backend, &config, &[source]));

    assert_eq!((found, written), (2, 1));
    assert_eq!(
        dump(&backend, &source, "devices.paired")["chan-a"]["label"],
        "newer"
    );
}

#[test]
fn a_workspace_without_a_legacy_file_imports_nothing_and_leaves_no_marker() {
    let (dir, config) = workspace();
    let backend = backend();
    let source = devices();

    assert_eq!(
        outcome(run_with(&backend, &config, &[source])),
        Outcome::NoSource
    );

    assert!(!is_done(&backend, &config, &source).unwrap());
    assert!(tree(dir.path()).is_empty(), "nothing was created");
}

#[test]
fn a_failed_import_leaves_no_marker_and_is_retried() {
    let (dir, config) = workspace();
    let source = devices();
    let db = dir.path().join(source.db);
    std::fs::create_dir_all(db.parent().unwrap()).unwrap();
    std::fs::write(&db, b"this is not a database").unwrap();
    let backend = backend();

    let report = run_with(&backend, &config, &[source]);
    assert!(report[0].outcome.is_err());
    assert!(!is_done(&backend, &config, &source).unwrap());
    assert_eq!(std::fs::read(&db).unwrap(), b"this is not a database");

    // Once the file is readable the next boot imports it.
    std::fs::remove_file(&db).unwrap();
    legacy_device(&config, "chan-a", "Phone", false);
    let (found, written) = imported(run_with(&backend, &config, &[source]));
    assert_eq!((found, written), (1, 1));
}

#[test]
fn records_the_previous_release_wrote_as_documents_are_imported_and_win_over_legacy_rows() {
    // The release before this one kept driver document tables inside the
    // legacy file. Build such a file: a legacy row, then a document with the
    // same id that was changed after the move.
    let (dir, config) = workspace();
    legacy_device(&config, "chan-a", "Phone", false);
    let source = devices();
    let db = dir.path().join(source.db);
    {
        use tinystoragedrivers::StorageBackend as _;
        let url = format!("sqlite:{}", db.display());
        let storage =
            crate::storage::block_on(async move { crate::storage::open(&url).await }).unwrap();
        let docs = Arc::clone(storage.for_scope(&Scope::local()).unwrap().documents());
        crate::storage::block_on(async move {
            docs.ensure_collection(&(source.collections)().remove(0))
                .await?;
            docs.put(
                "paired_devices",
                "chan-a",
                serde_json::json!({ "label": "Renamed", "revoked": true }),
                Precondition::Absent,
            )
            .await?;
            docs.put(
                "paired_devices",
                "chan-d",
                serde_json::json!({ "label": "Only a document" }),
                Precondition::Absent,
            )
            .await
        })
        .unwrap();
    }
    let backend = backend();

    let (found, written) = imported(run_with(&backend, &config, &[source]));

    // chan-a from the document table (newer copy) and the legacy row with the
    // same id (skipped), plus chan-d.
    assert_eq!((found, written), (3, 2));
    let devices = dump(&backend, &source, "devices.paired");
    assert_eq!(devices["chan-a"]["label"], "Renamed");
    assert_eq!(devices["chan-a"]["revoked"], true);
    assert_eq!(devices["chan-d"]["label"], "Only a document");
}

#[test]
fn rows_still_in_the_write_ahead_log_are_read() {
    let (dir, config) = workspace();
    let source = devices();
    let db = dir.path().join(source.db);
    std::fs::create_dir_all(db.parent().unwrap()).unwrap();
    // A writer that has not checkpointed: its rows exist only in the -wal.
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    conn.pragma_update(None, "wal_autocheckpoint", 0).unwrap();
    conn.execute_batch(DEVICES_DDL).unwrap();
    conn.execute_batch(
        "INSERT INTO paired_devices (channel_id, label, device_pubkey, core_session_token_hash, created_at)
         VALUES ('wal-chan', 'In the log', 'pk', 'hash', '2026-01-01T00:00:00Z');",
    )
    .unwrap();
    assert!(dir.path().join("devices/devices.db-wal").is_file());
    let backend = backend();

    let (found, written) = imported(run_with(&backend, &config, &[source]));

    assert_eq!((found, written), (1, 1));
    assert_eq!(
        dump(&backend, &source, "devices.paired")["wal-chan"]["label"],
        "In the log"
    );
    drop(conn);
}

#[test]
fn markers_are_kept_per_workspace_on_a_shared_backend() {
    let (_dir_a, config_a) = workspace();
    let (_dir_b, config_b) = workspace();
    legacy_device(&config_a, "chan-a", "Phone", false);
    let backend = backend();
    let source = devices();

    imported(run_with(&backend, &config_a, &[source]));

    assert!(is_done(&backend, &config_a, &source).unwrap());
    assert!(!is_done(&backend, &config_b, &source).unwrap());
}

#[tokio::test]
async fn without_an_installed_backend_the_host_import_does_nothing() {
    let (_dir, config) = workspace();
    legacy_device(&config, "chan-a", "Phone", false);
    assert!(crate::storage::installed().is_none());

    assert!(
        run(&crate::core::runtime::DispatchCtx::unbound(), &config)
            .await
            .is_empty()
    );
}

#[test]
fn every_registered_source_names_a_distinct_domain_and_file() {
    let all = sources();
    let mut domains: Vec<_> = all.iter().map(|s| s.domain).collect();
    let mut files: Vec<_> = all.iter().map(|s| s.db).collect();
    domains.sort_unstable();
    files.sort_unstable();
    domains.dedup();
    files.dedup();
    assert_eq!(domains.len(), all.len());
    assert_eq!(files.len(), all.len());
}
