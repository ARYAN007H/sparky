use super::*;
use crate::desktop::notifications::store as notifications;
use crate::desktop::notifications::types::{
    CoreNotificationCategory, CoreNotificationEvent, IntegrationNotification, NotificationSettings,
    NotificationStatus,
};
use crate::integrations::composio::contract::NormalizedTask;
use crate::integrations::task_sources::store as task_sources;
use crate::integrations::task_sources::types::{FilterSpec, ProviderSlug, SourceTarget};
use crate::security::approval::store as approvals;
use crate::security::approval::types::{
    ApprovalDecision, ApprovalSourceContext, ExecutionOutcome, PendingApproval,
};
use crate::security::devices::store as devices;
use crate::storage::rusqlite;
use crate::storage::{MemoryStorage, Scope};
use chrono::{Duration, Utc};
use std::collections::BTreeMap;
use tempfile::TempDir;
use tinystoragedrivers::{DocumentStoreExt, Query};

fn backend() -> Arc<dyn StorageBackend> {
    Arc::new(MemoryStorage::new())
}

/// A workspace and the two configs that address it: `classic` writes the
/// legacy tables (how the fixtures are built), `plain` is the config the
/// importer runs with.
fn workspace() -> (TempDir, Config, Config) {
    let dir = TempDir::new().unwrap();
    let mut classic = Config {
        workspace_dir: dir.path().to_path_buf(),
        ..Config::default()
    };
    classic.storage.url = Some("classic".into());
    let plain = Config {
        workspace_dir: dir.path().to_path_buf(),
        ..Config::default()
    };
    (dir, classic, plain)
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
    let specs = (source.collections)();
    let collection = collection.to_string();
    crate::storage::block_on(async move {
        for spec in &specs {
            docs.ensure_collection(spec).await?;
        }
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

fn request(id: &str, expires_in: i64) -> PendingApproval {
    PendingApproval {
        request_id: id.to_string(),
        tool_name: "composio".to_string(),
        action_summary: format!("summary {id}"),
        args_redacted: serde_json::json!({ "action": "execute", "id": id }),
        created_at: Utc::now(),
        expires_at: Some(Utc::now() + Duration::minutes(expires_in)),
        source_context: Some(ApprovalSourceContext::Flow {
            flow_id: "flow-1".into(),
            run_id: "run-1".into(),
            node_id: None,
        }),
        tool_call_id: Some(format!("call-{id}")),
        agent_id: Some("agent-7".into()),
    }
}

fn legacy_approvals(classic: &Config) {
    approvals::insert_pending(classic, &request("pending-1", 10), "sess").unwrap();
    approvals::insert_pending(classic, &request("approved-1", 10), "sess").unwrap();
    approvals::insert_pending(classic, &request("denied-1", 10), "sess").unwrap();
    approvals::decide(classic, "approved-1", ApprovalDecision::ApproveOnce).unwrap();
    approvals::record_execution(classic, "approved-1", ExecutionOutcome::Success, None).unwrap();
    approvals::decide(classic, "denied-1", ApprovalDecision::Deny).unwrap();
    approvals::insert_flow_trust(classic, "flow-1", "shell").unwrap();
    approvals::insert_flow_trust(classic, "flow-1", "http").unwrap();
}

#[test]
fn approvals_move_into_the_backend_and_the_legacy_file_is_untouched() {
    let (dir, classic, plain) = workspace();
    legacy_approvals(&classic);
    let before = tree(dir.path());
    assert!(before.keys().any(|name| name.ends_with("approval.db")));
    let backend = backend();
    let source = crate::security::approval::import_source();

    let (found, written) = imported(run_with(&backend, &plain, &[source]));

    assert_eq!((found, written), (5, 5), "3 requests and 2 flow grants");
    let requests = dump(&backend, &source, "approvals");
    assert_eq!(
        requests.keys().map(String::as_str).collect::<Vec<_>>(),
        ["approved-1", "denied-1", "pending-1"]
    );
    let pending = &requests["pending-1"];
    assert_eq!(pending["tool_name"], "composio");
    assert_eq!(pending["action_summary"], "summary pending-1");
    assert_eq!(pending["pending"], true);
    assert_eq!(pending["agent_id"], "agent-7");
    assert_eq!(pending["tool_call_id"], "call-pending-1");
    assert!(pending["expires_ts"].is_i64(), "the expiry key is carried");
    let approved = &requests["approved-1"];
    assert_eq!(approved["pending"], false);
    assert_eq!(approved["decision"], "approve_once");
    assert!(approved.get("executed_at").is_some());
    assert_eq!(requests["denied-1"]["decision"], "deny");
    assert_eq!(dump(&backend, &source, "approval_flow_trust").len(), 2);

    assert_eq!(
        tree(dir.path()),
        before,
        "no legacy file was modified, created or removed"
    );
}

#[test]
fn a_finished_import_does_not_run_again_or_resurrect_deleted_records() {
    let (_dir, classic, plain) = workspace();
    legacy_approvals(&classic);
    let backend = backend();
    let source = crate::security::approval::import_source();
    imported(run_with(&backend, &plain, &[source]));
    assert!(is_done(&backend, &plain, &source).unwrap());

    // The user removes a record; the untouched legacy file still has it.
    let docs = Arc::clone(backend.for_scope(&Scope::local()).unwrap().documents());
    crate::storage::block_on(async move {
        docs.delete("approvals", "denied-1", Precondition::None)
            .await
    })
    .unwrap();

    assert_eq!(
        outcome(run_with(&backend, &plain, &[source])),
        Outcome::AlreadyDone
    );
    assert!(!dump(&backend, &source, "approvals").contains_key("denied-1"));
}

#[test]
fn an_existing_record_is_never_overwritten() {
    let (_dir, classic, plain) = workspace();
    legacy_approvals(&classic);
    let backend = backend();
    let source = crate::security::approval::import_source();
    // A record the new code already wrote (a resumed import).
    let docs = Arc::clone(backend.for_scope(&Scope::local()).unwrap().documents());
    crate::storage::block_on(async move {
        docs.ensure_collection(&(source.collections)().remove(0))
            .await?;
        docs.put(
            "approvals",
            "pending-1",
            serde_json::json!({ "tool_name": "newer" }),
            Precondition::Absent,
        )
        .await
    })
    .unwrap();

    let (found, written) = imported(run_with(&backend, &plain, &[source]));

    assert_eq!((found, written), (5, 4));
    assert_eq!(
        dump(&backend, &source, "approvals")["pending-1"]["tool_name"],
        "newer"
    );
}

#[test]
fn a_workspace_without_a_legacy_file_imports_nothing_and_leaves_no_marker() {
    let (dir, _classic, plain) = workspace();
    let backend = backend();
    let source = crate::security::devices::import_source();

    assert_eq!(
        outcome(run_with(&backend, &plain, &[source])),
        Outcome::NoSource
    );

    assert!(!is_done(&backend, &plain, &source).unwrap());
    assert!(tree(dir.path()).is_empty(), "nothing was created");
}

#[test]
fn a_failed_import_leaves_no_marker_and_is_retried() {
    let (dir, classic, plain) = workspace();
    let source = crate::security::devices::import_source();
    let db = dir.path().join(source.db);
    std::fs::create_dir_all(db.parent().unwrap()).unwrap();
    std::fs::write(&db, b"this is not a database").unwrap();
    let backend = backend();

    let report = run_with(&backend, &plain, &[source]);
    assert!(report[0].outcome.is_err());
    assert!(!is_done(&backend, &plain, &source).unwrap());
    assert_eq!(std::fs::read(&db).unwrap(), b"this is not a database");

    // Once the file is readable the next boot imports it.
    std::fs::remove_file(&db).unwrap();
    devices::insert_device(&classic, "chan-a", "Phone", "pk-a", "hash-a").unwrap();
    let (found, written) = imported(run_with(&backend, &plain, &[source]));
    assert_eq!((found, written), (1, 1));
}

#[test]
fn devices_keep_every_column_including_revocation() {
    let (dir, classic, plain) = workspace();
    devices::insert_device(&classic, "chan-a", "Phone", "pk-a", "hash-a").unwrap();
    devices::insert_device(&classic, "chan-b", "Tablet", "pk-b", "hash-b").unwrap();
    devices::insert_device(&classic, "chan-c", "Old laptop", "pk-c", "hash-c").unwrap();
    devices::touch_device(&classic, "chan-a").unwrap();
    devices::revoke_device(&classic, "chan-c").unwrap();
    let before = tree(dir.path());
    let backend = backend();
    let source = crate::security::devices::import_source();

    let (found, written) = imported(run_with(&backend, &plain, &[source]));

    assert_eq!((found, written), (3, 3));
    let found = dump(&backend, &source, "paired_devices");
    assert_eq!(found["chan-a"]["label"], "Phone");
    assert_eq!(found["chan-a"]["device_pubkey"], "pk-a");
    assert_eq!(found["chan-a"]["core_session_token_hash"], "hash-a");
    assert!(found["chan-a"]["last_seen_at"].is_string());
    assert!(found["chan-b"]["last_seen_at"].is_null());
    assert_eq!(found["chan-b"]["revoked"], false);
    assert_eq!(found["chan-c"]["revoked"], true);
    assert_eq!(tree(dir.path()), before);
}

fn note(id: &str, provider: &str, secs_ago: i64) -> IntegrationNotification {
    IntegrationNotification {
        id: id.to_string(),
        provider: provider.to_string(),
        account_id: Some("acct".into()),
        title: format!("title {id}"),
        body: format!("body {id}"),
        raw_payload: serde_json::json!({ "k": id }),
        importance_score: None,
        triage_action: None,
        triage_reason: None,
        status: NotificationStatus::Unread,
        received_at: Utc::now() - Duration::seconds(secs_ago),
        scored_at: None,
    }
}

fn event(id: &str, ts: u64) -> CoreNotificationEvent {
    CoreNotificationEvent {
        id: id.to_string(),
        category: CoreNotificationCategory::Agents,
        title: "Cron job completed".into(),
        body: "done".into(),
        deep_link: None,
        timestamp_ms: ts,
        actions: None,
        workspace: None,
        workspace_revision: None,
    }
}

fn legacy_notifications(classic: &Config) {
    notifications::insert(classic, &note("n-old", "slack", 60)).unwrap();
    notifications::insert(classic, &note("n-new", "gmail", 0)).unwrap();
    notifications::update_triage(classic, "n-new", 0.8, "react", "important").unwrap();
    notifications::mark_read(classic, "n-old").unwrap();
    notifications::upsert_settings(
        classic,
        &NotificationSettings {
            provider: "slack".into(),
            enabled: false,
            importance_threshold: 0.5,
            route_to_orchestrator: false,
        },
    )
    .unwrap();
    notifications::insert_core_notification(classic, &event("e1", 1_000)).unwrap();
    notifications::insert_core_notification(classic, &event("e2", 2_000)).unwrap();
    notifications::mark_core_notification_read(classic, "e1").unwrap();
}

#[test]
fn notifications_are_filed_under_the_real_workspace_key() {
    let (dir, classic, plain) = workspace();
    legacy_notifications(&classic);
    let before = tree(dir.path());
    let backend = backend();
    let source = crate::desktop::notifications::import_source();

    let (found, written) = imported(run_with(&backend, &plain, &[source]));

    assert_eq!((found, written), (5, 5));
    let items = dump(&backend, &source, "integration_notifications");
    assert_eq!(items.len(), 2);
    assert_eq!(items["n-new"]["triage_action"], "react");
    assert_eq!(items["n-new"]["raw_payload"], "{\"k\":\"n-new\"}");
    assert_eq!(items["n-old"]["status"], "read");
    let settings = dump(&backend, &source, "notification_settings");
    assert_eq!(settings["slack"]["enabled"], false);

    let key = format!(
        "path:{}",
        hex::encode(plain.workspace_dir.as_os_str().as_encoded_bytes())
    );
    let core = dump(&backend, &source, "core_notifications");
    assert_eq!(core.len(), 2);
    for (id, doc) in &core {
        assert!(id.starts_with(&format!("{}:{key}/", key.len())), "{id}");
        assert_eq!(doc["workspace"], key.as_str());
    }
    assert_eq!(core[&format!("{}:{key}/e1", key.len())]["read"], true);
    assert_eq!(core[&format!("{}:{key}/e2", key.len())]["read"], false);
    assert_eq!(tree(dir.path()), before);
}

#[test]
fn records_the_previous_default_wrote_as_documents_are_imported_and_rekeyed() {
    // The earlier default kept driver document tables inside notifications.db
    // under a fixed workspace key; writing through it builds that file.
    let (_dir, _classic, plain) = workspace();
    notifications::insert(&plain, &note("n-doc", "gmail", 5)).unwrap();
    notifications::insert_core_notification(&plain, &event("e-doc", 5_000)).unwrap();
    let backend = backend();
    let source = crate::desktop::notifications::import_source();

    let (found, written) = imported(run_with(&backend, &plain, &[source]));

    // The notification, its content-dedup entry and the core event.
    assert_eq!((found, written), (3, 3));
    assert!(dump(&backend, &source, "integration_notifications").contains_key("n-doc"));
    let key = format!(
        "path:{}",
        hex::encode(plain.workspace_dir.as_os_str().as_encoded_bytes())
    );
    let core = dump(&backend, &source, "core_notifications");
    let id = format!("{}:{key}/e-doc", key.len());
    assert_eq!(core.len(), 1);
    assert_eq!(core[&id]["workspace"], key.as_str());
    assert_eq!(core[&id]["timestamp_ms"], 5_000);
}

#[test]
fn a_document_record_wins_over_a_legacy_row_with_the_same_id() {
    // Import once through the old default (rows move into document tables and
    // the legacy tables are retired), then change a record there.
    let (_dir, classic, plain) = workspace();
    legacy_approvals(&classic);
    approvals::list_pending(&plain).unwrap();
    approvals::decide(&plain, "pending-1", ApprovalDecision::Deny).unwrap();
    let backend = backend();
    let source = crate::security::approval::import_source();

    imported(run_with(&backend, &plain, &[source]));

    let requests = dump(&backend, &source, "approvals");
    assert_eq!(requests["pending-1"]["decision"], "deny");
    assert_eq!(requests["pending-1"]["pending"], false);
}

fn filter() -> FilterSpec {
    FilterSpec::Github {
        repo: Some("tinyhumansai/openhuman".into()),
        labels: vec!["bug".into()],
        assignee_is_me: true,
        state: Some("open".into()),
        fetch_mode: Default::default(),
        extra: serde_json::json!({}),
    }
}

fn task(external_id: &str, title: &str) -> NormalizedTask {
    NormalizedTask {
        external_id: external_id.into(),
        provider: "github".into(),
        title: title.into(),
        ..Default::default()
    }
}

#[test]
fn task_sources_and_their_dedup_ledger_are_imported() {
    let (dir, classic, plain) = workspace();
    let first = task_sources::add_source(
        &classic,
        ProviderSlug::Github,
        Some("conn-1".into()),
        Some("My issues".into()),
        filter(),
        1800,
        SourceTarget::AgentTodoProactive,
        25,
    )
    .unwrap();
    let second = task_sources::add_source(
        &classic,
        ProviderSlug::Github,
        None,
        None,
        filter(),
        600,
        SourceTarget::AgentTodoProactive,
        10,
    )
    .unwrap();
    task_sources::mark_ingested(&classic, &first.id, &task("t-1", "First")).unwrap();
    task_sources::mark_ingested(&classic, &first.id, &task("t-2", "Second")).unwrap();
    task_sources::mark_ingested(&classic, &second.id, &task("t-1", "Other")).unwrap();
    let before = tree(dir.path());
    let backend = backend();
    let source = crate::integrations::task_sources::import_source();

    let (found, written) = imported(run_with(&backend, &plain, &[source]));

    assert_eq!((found, written), (5, 5), "2 sources and 3 ledger entries");
    let sources = dump(&backend, &source, "task_sources");
    assert_eq!(sources.len(), 2);
    assert_eq!(sources[&first.id]["name"], "My issues");
    assert_eq!(sources[&first.id]["connection_id"], "conn-1");
    assert_eq!(sources[&first.id]["interval_secs"], 1800);
    assert_eq!(sources[&first.id]["max_tasks_per_fetch"], 25);
    assert_eq!(dump(&backend, &source, "ingested_tasks").len(), 3);
    assert_eq!(tree(dir.path()), before);
}

#[test]
fn rows_still_in_the_write_ahead_log_are_read() {
    let (dir, _classic, plain) = workspace();
    let source = crate::security::devices::import_source();
    let db = dir.path().join(source.db);
    std::fs::create_dir_all(db.parent().unwrap()).unwrap();
    // A writer that has not checkpointed: its rows exist only in the -wal.
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    conn.pragma_update(None, "wal_autocheckpoint", 0).unwrap();
    conn.execute_batch(
        "CREATE TABLE paired_devices (
             channel_id TEXT PRIMARY KEY, label TEXT NOT NULL, device_pubkey TEXT NOT NULL,
             core_session_token_hash TEXT NOT NULL, shared_secret_encrypted BLOB,
             created_at TEXT NOT NULL, last_seen_at TEXT, revoked INTEGER NOT NULL DEFAULT 0
         );
         INSERT INTO paired_devices (channel_id, label, device_pubkey, core_session_token_hash, created_at)
         VALUES ('wal-chan', 'In the log', 'pk', 'hash', '2026-01-01T00:00:00Z');",
    )
    .unwrap();
    assert!(dir.path().join("devices/devices.db-wal").is_file());
    let backend = backend();

    let (found, written) = imported(run_with(&backend, &plain, &[source]));

    assert_eq!((found, written), (1, 1));
    assert_eq!(
        dump(&backend, &source, "paired_devices")["wal-chan"]["label"],
        "In the log"
    );
    drop(conn);
}

#[test]
fn markers_are_kept_per_workspace_on_a_shared_backend() {
    let (_dir_a, classic_a, plain_a) = workspace();
    let (_dir_b, _classic_b, plain_b) = workspace();
    devices::insert_device(&classic_a, "chan-a", "Phone", "pk-a", "hash-a").unwrap();
    let backend = backend();
    let source = crate::security::devices::import_source();

    imported(run_with(&backend, &plain_a, &[source]));

    assert!(is_done(&backend, &plain_a, &source).unwrap());
    assert!(!is_done(&backend, &plain_b, &source).unwrap());
}

#[tokio::test]
async fn without_an_installed_backend_the_host_import_does_nothing() {
    let (_dir, classic, plain) = workspace();
    legacy_approvals(&classic);
    assert!(crate::storage::installed().is_none());

    assert!(run(&plain).await.is_empty());
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
