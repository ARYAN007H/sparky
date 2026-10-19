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
    let granted = granted
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .into_owned();
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
    assert!(exposes_credentials(
        Path::new("/home/user/.ssh"),
        Some(home)
    ));
    assert!(!exposes_credentials(
        Path::new("/home/user/tools"),
        Some(home)
    ));
    assert!(!exposes_credentials(Path::new("/home/user"), None));
}

#[test]
fn enforcement_is_declared_only_where_the_host_accepts_a_best_effort_backend() {
    assert_eq!(enforcement(true), StdioSandboxEnforcement::Enforced);
    assert_eq!(enforcement(false), StdioSandboxEnforcement::Declared);
    assert_eq!(
        enforcement(require_process_isolation(true)),
        StdioSandboxEnforcement::Enforced,
        "a SaaS tenant is always enforced"
    );
    let desktop = enforcement(require_process_isolation(false));
    if cfg!(target_os = "macos") {
        assert_eq!(desktop, StdioSandboxEnforcement::Declared);
    } else {
        assert_eq!(desktop, StdioSandboxEnforcement::Enforced);
    }
}

#[cfg(feature = "modules")]
#[test]
fn built_sandbox_enforcement_follows_the_isolation_requirement() {
    let tmp = tempfile::tempdir().unwrap();
    let sandbox = stdio_sandbox(&config_in(tmp.path())).expect("a sandbox");
    assert_eq!(
        sandbox.enforcement,
        enforcement(sandbox.require_process_isolation)
    );
}

#[cfg(feature = "modules")]
#[test]
fn toolchain_grants_are_keyed_by_what_resolution_reads_not_by_workspace() {
    let one = tempfile::tempdir().unwrap();
    let two = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    // Two agents differ only by workspace: they share one resolution.
    assert_eq!(
        grants_key(&config_in(one.path()), Some(home.path())),
        grants_key(&config_in(two.path()), Some(home.path()))
    );
    // A different jail configuration or home must not reuse it.
    let mut wider = config_in(one.path());
    wider.runtime.local_jail.extra_read_only = vec!["/opt/tools".into()];
    assert_ne!(
        grants_key(&config_in(one.path()), Some(home.path())),
        grants_key(&wider, Some(home.path()))
    );
    assert_ne!(
        grants_key(&config_in(one.path()), Some(home.path())),
        grants_key(&config_in(one.path()), None)
    );
}

#[cfg(feature = "modules")]
#[test]
fn cached_toolchain_grants_are_reused_until_they_expire() {
    let key = "cached_toolchain_grants_are_reused_until_they_expire".to_string();
    assert!(cached_grants(&key).is_none());
    let grants = (
        vec![PathBuf::from("/opt/node")],
        vec![PathBuf::from("/opt/cache")],
    );
    GRANT_CACHE
        .lock()
        .insert(key.clone(), (std::time::Instant::now(), grants.clone()));
    assert_eq!(cached_grants(&key), Some(grants.clone()));
    let stale = std::time::Instant::now() - GRANT_CACHE_TTL - std::time::Duration::from_secs(1);
    GRANT_CACHE.lock().insert(key.clone(), (stale, grants));
    assert!(
        cached_grants(&key).is_none(),
        "an expired entry is resolved again"
    );
}
