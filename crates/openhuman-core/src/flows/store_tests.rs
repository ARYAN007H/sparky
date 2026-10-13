//! The catalog's storage and the import of the files tinyflows used to write
//! (`flows.db`, `checkpoints.db`, `drafts/*.json`) on first use.

use super::*;
use tempfile::TempDir;

fn fixture_workspace() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/storage/import/fixtures/tinyflows/workspace")
}

fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
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

fn config(tmp: &TempDir) -> Config {
    Config {
        workspace_dir: tmp.path().to_path_buf(),
        ..Config::default()
    }
}

#[test]
fn the_catalog_lives_in_its_own_workspace_database() {
    let tmp = TempDir::new().unwrap();
    let config = config(&tmp);
    assert!(crate::storage::installed().is_none());
    assert!(list_flows(&config).unwrap().0.is_empty());
    assert_eq!(db_path(&config), tmp.path().join("flows/catalog.db"));
    assert!(db_path(&config).is_file());
}

#[test]
fn legacy_flows_drafts_and_checkpoints_are_imported_on_first_use_and_left_untouched() {
    let tmp = TempDir::new().unwrap();
    let config = config(&tmp);
    copy_tree(&fixture_workspace(), tmp.path());
    let flows_db = dir(&config).join("flows.db");
    let before = std::fs::read(&flows_db).unwrap();

    let (flows, _) = list_flows(&config).unwrap();
    let mut ids: Vec<_> = flows.iter().map(|flow| flow.id.clone()).collect();
    ids.sort();
    assert_eq!(ids, ["flow-alpha", "flow-beta"]);
    assert_eq!(
        super::super::draft_store::list_drafts(&config)
            .unwrap()
            .len(),
        2
    );
    assert_eq!(list_revisions(&config, "flow-alpha", 10).unwrap().len(), 2);
    assert_eq!(
        get_flow_run(&config, "run-done").unwrap().unwrap().flow_id,
        "flow-alpha"
    );

    let checkpointer = crate::flows::tinyflows::open_flow_checkpointer(&config).unwrap();
    let thread = crate::storage::block_on_anyhow(async move {
        Ok(checkpointer.get_thread("run-1").await?.len())
    })
    .unwrap();
    assert!(thread > 0, "the interrupted run's checkpoints came across");

    assert_eq!(
        std::fs::read(&flows_db).unwrap(),
        before,
        "flows.db was modified"
    );

    // Imported once: a flow removed afterwards stays removed after a restart.
    remove_flow(&config, "flow-alpha").unwrap();
    crate::storage::local::forget(&db_path(&config));
    assert_eq!(list_flows(&config).unwrap().0.len(), 1);
}
