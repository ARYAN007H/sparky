use super::*;
use crate::storage::layout::test_fixtures::{fixture_docs, v1_profile};

#[tokio::test(flavor = "multi_thread")]
async fn the_import_brings_legacy_files_into_the_v1_database() {
    let fx = v1_profile();
    let summary = import(&fx.target, &fx.config).await.unwrap();
    assert_eq!(summary.memory_state, vec!["jobs".to_owned()]);
    assert!(
        summary.domains >= 3,
        "approval, devices, ... ran: {summary:?}"
    );

    let jobs = crate::storage::layout::test_fixtures::jobs_value(&v1_db(&fx.target));
    assert_eq!(jobs, Some(serde_json::json!({"jobs": [1, 2, 3]})));
    // the sources are only read
    assert!(fx.workspace().join("memory/jobs.json").is_file());
    // nothing else was disturbed
    assert_eq!(fixture_docs(&v1_db(&fx.target)).len(), 3);
}

#[tokio::test(flavor = "multi_thread")]
async fn importing_twice_changes_nothing() {
    let fx = v1_profile();
    import(&fx.target, &fx.config).await.unwrap();
    migrate(&fx.target).await.unwrap();
    dbg_docs(&v1_db(&fx.target));
    let first = super::super::db::table_counts(&v1_db(&fx.target)).unwrap();
    import(&fx.target, &fx.config).await.unwrap();
    migrate(&fx.target).await.unwrap();
    dbg_docs(&v1_db(&fx.target));
    let second = super::super::db::table_counts(&v1_db(&fx.target)).unwrap();
    eprintln!("DBG first={first:?} second={second:?}");
    assert_eq!(second, first);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_damaged_state_file_fails_the_import_before_anything_is_deleted() {
    let fx = v1_profile();
    crate::storage::layout::test_fixtures::write(
        &fx.workspace().join("memory/broken.json"),
        "{ not json",
    );
    let error = import(&fx.target, &fx.config)
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("nothing was archived or deleted"), "{error}");
}

#[tokio::test(flavor = "multi_thread")]
async fn no_workspace_means_nothing_to_import() {
    let dir = tempfile::TempDir::new().unwrap();
    let target = Target::new(dir.path(), "u1");
    assert_eq!(
        import(&target, &Config::default()).await.unwrap(),
        ImportSummary::default()
    );
    migrate(&target).await.unwrap();
    assert!(!target.workspace().exists());
}

fn dbg_docs(p: &std::path::Path) {
    let c = crate::storage::rusqlite::Connection::open(p).unwrap();
    let cols: Vec<String> = c.prepare("PRAGMA table_info(docs)").unwrap().query_map([], |r| r.get::<_, String>(1)).unwrap().map(|x| x.unwrap()).collect();
    eprintln!("DBGCOLS {cols:?}");
    let col = if cols.iter().any(|x| x == "collection") { "collection" } else { &cols[0] };
    let _ = col; let q = "SELECT scope||' '||coll, count(*) FROM docs GROUP BY 1 ORDER BY 1".to_string();
    let rows: Vec<(String, i64)> = c.prepare(&q).unwrap().query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))).unwrap().map(|x| x.unwrap()).collect();
    eprintln!("DBGDOCS {rows:?}");
}
