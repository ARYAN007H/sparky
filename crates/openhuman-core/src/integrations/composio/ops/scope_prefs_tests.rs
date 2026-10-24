use super::*;

fn config_in(dir: &tempfile::TempDir) -> Config {
    Config {
        workspace_dir: dir.path().to_path_buf(),
        ..Config::default()
    }
}

#[test]
fn wire_conversion_round_trips_every_flag() {
    for read in [false, true] {
        for write in [false, true] {
            for admin in [false, true] {
                let pref = UserScopePref { read, write, admin };
                assert_eq!(from_wire(to_wire(pref)), pref);
            }
        }
    }
}

#[test]
fn row_key_normalises_the_toolkit() {
    assert_eq!(UserScopePref::key(" GitHub "), "github");
    assert_eq!(
        UserScopePref::key("   "),
        "",
        "an all-whitespace toolkit has no row"
    );
}

#[tokio::test]
async fn save_rejects_an_empty_toolkit() {
    let dir = tempfile::tempdir().unwrap();
    let config = config_in(&dir);
    assert!(save(
        &crate::core::runtime::DispatchCtx::ambient(),
        &config,
        "  ",
        UserScopePref::default()
    )
    .await
    .is_err());
}

/// A stored preference is never lost: either the module took it (the read
/// returns it and the stored value is emptied) or the module was unreachable
/// (the read fails closed and the value is left exactly as it was so the
/// migration retries).
#[tokio::test]
async fn a_legacy_pref_either_reaches_the_module_or_stays_stored() {
    let dir = tempfile::tempdir().unwrap();
    let config = config_in(&dir);
    let legacy = serde_json::json!({"github":{"read":true,"write":false,"admin":true}});
    file_store::save(
        &crate::core::runtime::DispatchCtx::ambient(),
        &config,
        USER_SCOPES_FILE,
        &legacy,
    )
    .await
    .unwrap();
    let stored = UserScopePref {
        read: true,
        write: false,
        admin: true,
    };

    let pref = load(
        &crate::core::runtime::DispatchCtx::ambient(),
        &config,
        "github",
    )
    .await;
    let done = migrated().lock().unwrap().contains(&config.workspace_dir);
    let left: LegacyScopeFile = file_store::load(
        &crate::core::runtime::DispatchCtx::ambient(),
        &config,
        USER_SCOPES_FILE,
    )
    .await
    .unwrap();
    if done {
        assert_eq!(pref, Ok(stored), "the migrated row must decide visibility");
        assert!(left.is_empty(), "the stored value is emptied once moved");
    } else {
        assert!(pref.is_err(), "an unreachable module fails closed: {pref:?}");
        assert_eq!(left.get("github"), Some(&stored));
    }
}

#[tokio::test]
async fn load_rejects_an_empty_toolkit_instead_of_answering_the_default() {
    let dir = tempfile::tempdir().unwrap();
    let config = config_in(&dir);
    assert!(load(&crate::core::runtime::DispatchCtx::ambient(), &config, "  ")
        .await
        .is_err());
}

#[tokio::test]
async fn a_workspace_with_no_legacy_file_is_marked_migrated() {
    let dir = tempfile::tempdir().unwrap();
    let config = config_in(&dir);
    migrate_legacy(&crate::core::runtime::DispatchCtx::ambient(), &config).await;
    assert!(migrated().lock().unwrap().contains(&config.workspace_dir));
}
