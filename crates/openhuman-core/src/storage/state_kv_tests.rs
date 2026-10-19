use super::*;
use crate::storage::{MemoryStorage, Scope, StorageBackend};
use std::collections::BTreeMap;

fn state(storage: &MemoryStorage, scope: &str) -> StateKv {
    StateKv::over(
        &storage.for_scope(&Scope::new(scope).unwrap()).unwrap(),
        "test_owner",
        "state",
    )
}

#[test]
fn import_file_copies_a_legacy_file_once_and_leaves_it_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let legacy = dir.path().join("jobs.json");
    std::fs::write(&legacy, r#"{"a": 1}"#).unwrap();
    let kv = state(&MemoryStorage::new(), "local");
    kv.import_file("jobs", &legacy).unwrap();
    assert_eq!(
        kv.get::<BTreeMap<String, u32>>("jobs").unwrap().unwrap()["a"],
        1
    );
    assert!(legacy.exists(), "the legacy file is not touched");

    // A later change is not overwritten by a re-import, nor resurrected after a delete.
    kv.put("jobs", &BTreeMap::from([("a".to_string(), 9u32)]))
        .unwrap();
    std::fs::write(&legacy, r#"{"a": 5}"#).unwrap();
    kv.import_file("jobs", &legacy).unwrap();
    assert_eq!(
        kv.get::<BTreeMap<String, u32>>("jobs").unwrap().unwrap()["a"],
        9
    );
    kv.delete("jobs").unwrap();
    kv.import_file("jobs", &legacy).unwrap();
    assert!(kv.get::<serde_json::Value>("jobs").unwrap().is_none());
}

#[test]
fn import_file_does_not_replace_an_existing_value() {
    let dir = tempfile::tempdir().unwrap();
    let legacy = dir.path().join("x.json");
    std::fs::write(&legacy, "7").unwrap();
    let kv = state(&MemoryStorage::new(), "local");
    kv.put("x", &1u32).unwrap();
    kv.import_file("x", &legacy).unwrap();
    assert_eq!(kv.get::<u32>("x").unwrap(), Some(1));
}

#[test]
fn import_file_records_a_missing_or_corrupt_file_without_failing() {
    let dir = tempfile::tempdir().unwrap();
    let kv = state(&MemoryStorage::new(), "local");
    kv.import_file("none", &dir.path().join("missing.json"))
        .unwrap();
    let corrupt = dir.path().join("bad.json");
    std::fs::write(&corrupt, "{not json").unwrap();
    kv.import_file("bad", &corrupt).unwrap();
    assert!(kv.get::<serde_json::Value>("bad").unwrap().is_none());
    // A file that shows up after the import was recorded is not imported.
    std::fs::write(dir.path().join("missing.json"), "1").unwrap();
    kv.import_file("none", &dir.path().join("missing.json"))
        .unwrap();
    assert!(kv.get::<u32>("none").unwrap().is_none());
}
