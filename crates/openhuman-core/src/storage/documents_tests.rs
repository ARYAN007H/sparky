use super::*;
use serde_json::json;
use tinystoragedrivers::{IndexSpec, Precondition};

use crate::storage::{MemoryStorage, Scope, StorageBackend};

const THINGS: &str = "things";

fn specs() -> Vec<CollectionSpec> {
    vec![CollectionSpec::new(THINGS).index(IndexSpec::new("by_state", ["state"]))]
}

fn repo(storage: &MemoryStorage, scope: &str) -> Repo {
    let scoped = storage.for_scope(&Scope::new(scope).unwrap()).unwrap();
    Repo::over(&scoped, "test", specs)
}

#[test]
fn run_declares_the_collections_and_returns_the_result() {
    let storage = MemoryStorage::new();
    let repo = repo(&storage, "local");
    repo.run(|docs| async move {
        docs.put(
            THINGS,
            "a",
            json!({ "state": "open" }),
            Precondition::Absent,
        )
        .await
        .map(|_| ())
    })
    .unwrap();
    let state = repo
        .run(|docs| async move { docs.get(THINGS, "a").await })
        .unwrap()
        .map(|stored| text(&stored.doc, "state").map(str::to_string));
    assert_eq!(state, Some(Some("open".to_string())));
}

#[test]
fn storage_errors_name_the_domain() {
    let storage = MemoryStorage::new();
    let error = repo(&storage, "local")
        .run(|_| async { Err::<(), _>(StorageError::conflict("raced")) })
        .unwrap_err();
    assert!(error.to_string().starts_with("[test] storage:"), "{error}");
}

#[test]
fn scopes_do_not_see_each_other() {
    let storage = MemoryStorage::new();
    repo(&storage, "alice")
        .run(|docs| async move {
            docs.put(THINGS, "a", json!({}), Precondition::Absent)
                .await
                .map(|_| ())
        })
        .unwrap();
    let seen = repo(&storage, "bob")
        .run(|docs| async move { docs.get(THINGS, "a").await })
        .unwrap();
    assert!(seen.is_none());
}

fn backend() -> Arc<dyn StorageBackend> {
    Arc::new(MemoryStorage::new())
}

#[test]
fn a_collection_is_declared_once_per_backend_scope_and_name() {
    let backend = backend();
    assert!(!is_declared(&backend, "local", THINGS));
    mark_declared(&backend, "local", THINGS);
    assert!(is_declared(&backend, "local", THINGS));
    assert!(!is_declared(&backend, "agent-b", THINGS), "another scope");
    assert!(
        !is_declared(&backend, "local", "other"),
        "another collection"
    );
    assert!(
        !is_declared(&self::backend(), "local", THINGS),
        "another backend"
    );
}

#[test]
fn a_repo_with_an_origin_declares_a_collection_only_on_its_first_run() {
    let backend = backend();
    let scoped = backend.for_scope(&Scope::new("cached").unwrap()).unwrap();
    let mut repo = Repo::over(&scoped, "test", specs);
    repo.origin = Some(Origin {
        backend: Arc::clone(&backend),
        scope: "cached".to_string(),
    });
    assert!(!is_declared(&backend, "cached", THINGS));
    repo.run(|docs| async move { docs.get(THINGS, "a").await })
        .unwrap();
    assert!(is_declared(&backend, "cached", THINGS));
    // Second run still works with the declaration skipped.
    repo.run(|docs| async move { docs.get(THINGS, "a").await })
        .unwrap();
}

fn repo_on(backend: &Arc<dyn StorageBackend>, scope: &str) -> Repo {
    let scoped = backend.for_scope(&Scope::new(scope).unwrap()).unwrap();
    let mut repo = Repo::over(&scoped, "test", specs);
    repo.origin = Some(Origin {
        backend: Arc::clone(backend),
        scope: scope.to_string(),
    });
    repo
}

#[test]
fn one_time_work_runs_once_per_backend_and_scope_and_retries_failures() {
    let runs = std::cell::Cell::new(0);
    let count = || {
        runs.set(runs.get() + 1);
        Ok(())
    };
    let first = backend();
    let repo = repo_on(&first, "once");
    assert!(repo
        .once_per_backend("k", || Err(anyhow::anyhow!("boom")))
        .is_err());
    repo.once_per_backend("k", count).unwrap();
    repo.once_per_backend("k", count).unwrap();
    assert_eq!(runs.get(), 1, "a failure is retried, a success is recorded");
    repo.once_per_backend("other", count).unwrap();
    repo_on(&first, "once-b")
        .once_per_backend("k", count)
        .unwrap();
    repo_on(&backend(), "once")
        .once_per_backend("k", count)
        .unwrap();
    assert_eq!(runs.get(), 4, "another key, scope or backend runs afresh");

    // Without an origin there is no identity to key on: it always runs.
    let plain = Repo::over(
        &first.for_scope(&Scope::new("once").unwrap()).unwrap(),
        "test",
        specs,
    );
    plain.once_per_backend("k", count).unwrap();
    plain.once_per_backend("k", count).unwrap();
    assert_eq!(runs.get(), 6);
}
