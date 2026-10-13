use super::*;

#[test]
fn marks_survive_a_reopen_and_are_written_atomically() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join(FILE);
    let store = RemixedIds::open(path.clone());
    assert!(!store.contains("r1"));
    store.mark("r1".into());
    store.mark("r1".into());
    store.mark("r2".into());
    assert!(store.contains("r1"));

    let reopened = RemixedIds::open(path);
    assert!(reopened.contains("r1") && reopened.contains("r2"));
    let entries: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
    assert_eq!(entries.len(), 1, "temp files left behind");
}

#[test]
fn an_oversized_file_is_capped_on_load_keeping_the_newest() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join(FILE);
    let ids: Vec<String> = (0..CAP + 50).map(|i| format!("id-{i}")).collect();
    std::fs::write(&path, serde_json::to_string(&ids).unwrap()).unwrap();
    let store = RemixedIds::open(path);
    assert!(!store.contains("id-0"));
    assert!(store.contains(&format!("id-{}", CAP + 49)));
}

#[test]
fn a_corrupt_file_starts_empty() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join(FILE);
    std::fs::write(&path, "not json").unwrap();
    assert!(!RemixedIds::open(path).contains("x"));
}

#[test]
fn one_store_serves_a_workspace() {
    let dir = tempfile::TempDir::new().unwrap();
    let a = for_workspace(dir.path()).unwrap();
    let b = for_workspace(dir.path()).unwrap();
    assert!(Arc::ptr_eq(&a, &b));
}
