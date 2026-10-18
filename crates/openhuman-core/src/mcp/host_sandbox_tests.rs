use super::*;

fn config_in(dir: &Path) -> Config {
    let mut config = Config::default();
    config.workspace_dir = dir.to_path_buf();
    config
}

#[cfg(feature = "modules")]
#[test]
fn sandbox_workspace_is_a_dedicated_directory_inside_the_core_workspace() {
    let tmp = tempfile::tempdir().unwrap();
    let sandbox = stdio_sandbox(&config_in(tmp.path())).expect("a sandbox");
    let expected = tmp.path().join(WORKSPACE_DIR).canonicalize().unwrap();
    assert_eq!(sandbox.workspace, expected.to_string_lossy());
    assert!(expected.is_dir());
    assert!(sandbox.allow_subprocess);
}

#[cfg(feature = "modules")]
#[test]
fn configured_grants_pass_the_host_floor() {
    let tmp = tempfile::tempdir().unwrap();
    let granted = tmp.path().join("tools");
    std::fs::create_dir_all(&granted).unwrap();
    let mut config = config_in(tmp.path());
    config.runtime.local_jail.extra_read_only = vec![
        granted.to_string_lossy().into_owned(),
        "/etc".into(),
        "relative/path".into(),
        "/no/such/dir/anywhere".into(),
    ];
    let sandbox = stdio_sandbox(&config).expect("a sandbox");
    let granted = granted.canonicalize().unwrap().to_string_lossy().into_owned();
    assert_eq!(sandbox.read_only, vec![granted]);
}

#[test]
fn isolation_floor_is_required_except_for_the_macos_desktop() {
    assert!(require_process_isolation(true));
    assert_eq!(require_process_isolation(false), !cfg!(target_os = "macos"));
}

#[test]
fn a_grant_that_contains_a_credential_store_is_not_admitted() {
    let home = Path::new("/home/user");
    assert!(exposes_credentials(Path::new("/home/user"), Some(home)));
    assert!(exposes_credentials(Path::new("/home/user/.ssh"), Some(home)));
    assert!(!exposes_credentials(Path::new("/home/user/tools"), Some(home)));
    assert!(!exposes_credentials(Path::new("/home/user"), None));
}
