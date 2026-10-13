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
    assert!(save(&config, "  ", UserScopePref::default()).await.is_err());
}

/// A stored preference is never lost: either the module took it (the read
/// returns it and the legacy file is emptied) or the module was unreachable
/// (the read fails open to the default and the file is left exactly as it was
/// so the migration retries).
#[tokio::test]
async fn a_legacy_pref_either_reaches_the_module_or_stays_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let config = config_in(&dir);
    let path = file_store::path(&config, USER_SCOPES_FILE);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let legacy = r#"{"github":{"read":true,"write":false,"admin":true}}"#;
    std::fs::write(&path, legacy).unwrap();
    let stored = UserScopePref {
        read: true,
        write: false,
        admin: true,
    };

    let pref = load_or_default(&config, "github").await;
    let done = migrated().lock().unwrap().contains(&config.workspace_dir);
    if done {
        assert_eq!(pref, stored, "the migrated row must decide visibility");
        let left: LegacyScopeFile =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(left.is_empty(), "the legacy file is emptied once moved");
    } else {
        assert_eq!(
            pref,
            UserScopePref::default(),
            "unreachable module fails open"
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), legacy);
    }
}

#[tokio::test]
async fn an_unreadable_legacy_file_defers_the_migration_without_erasing_it() {
    let dir = tempfile::tempdir().unwrap();
    let config = config_in(&dir);
    let path = file_store::path(&config, USER_SCOPES_FILE);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"not json").unwrap();

    migrate_legacy(&config).await;
    assert_eq!(std::fs::read(&path).unwrap(), b"not json");
}

#[tokio::test]
async fn a_workspace_with_no_legacy_file_is_marked_migrated() {
    let dir = tempfile::tempdir().unwrap();
    let config = config_in(&dir);
    migrate_legacy(&config).await;
    assert!(migrated().lock().unwrap().contains(&config.workspace_dir));
}
