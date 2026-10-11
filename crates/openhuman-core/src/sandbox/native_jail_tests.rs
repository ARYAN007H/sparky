//! Host-side request mapping for TinyBox native-jail execution.

use super::*;

#[test]
fn jail_status_is_ready_only_above_the_isolation_floor() {
    assert_eq!(
        super::super::ops::local_status_for_jail(true, true),
        SandboxStatus::Ready
    );
    assert_eq!(
        super::super::ops::local_status_for_jail(false, true),
        SandboxStatus::Inactive
    );
    assert_eq!(
        super::super::ops::local_status_for_jail(true, false),
        SandboxStatus::Inactive
    );
}

#[cfg(feature = "modules")]
#[test]
fn shell_request_uses_the_platform_shell_and_caller_known_process_resource() -> anyhow::Result<()> {
    let resource = contract::ResourceId("known-resource".to_owned());
    let cwd = tempfile::tempdir()?;
    let env = BTreeMap::from([("LANG".to_owned(), "C".to_owned())]);
    let request = shell_request(&resource, "printf 'one two'", cwd.path(), env.clone())?;

    let expected = platform_shell::build_std_command("printf 'one two'");
    let expected_argv = std::iter::once(expected.get_program().to_str().unwrap().to_owned())
        .chain(expected.get_args().map(|arg| arg.to_str().unwrap().to_owned()))
        .collect::<Vec<_>>();
    assert_eq!(request.resource, resource);
    assert_eq!(request.argv, expected_argv);
    assert_eq!(request.cwd.as_deref(), cwd.path().to_str());
    assert_eq!(request.env, env);
    assert_eq!(request.stdin, None);
    Ok(())
}

#[cfg(feature = "modules")]
#[test]
fn native_jail_request_preserves_grants_and_requires_enforcement() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let readonly = tempfile::tempdir()?;
    let readwrite = tempfile::tempdir()?;
    let policy = SandboxPolicy {
        backend: super::super::types::SandboxBackendKind::Local,
        workspace_root: root.path().to_path_buf(),
        state_dir: root.path().to_path_buf(),
        read_only_mounts: vec![readonly.path().to_path_buf()],
        read_write_mounts: vec![readwrite.path().to_path_buf()],
        allow_network: false,
        env_passthrough: Vec::new(),
        docker_overrides: None,
    };
    let request = jail_request(&policy, vec!["/scratch".to_owned()])?;

    assert_eq!(request.read_only, [readonly.path().to_str().unwrap()]);
    assert_eq!(request.read_write, [readwrite.path().to_str().unwrap(), "/scratch"]);
    assert!(!request.allow_net);
    assert!(request.allow_subprocess);
    assert_eq!(request.enforcement, contract::JailEnforcement::Enforced);
    assert!(request.require_process_isolation);
    Ok(())
}

#[cfg(feature = "modules")]
#[test]
fn native_jail_environment_keeps_the_sandbox_allowlist_and_scratch_grant() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let scratch = tempfile::tempdir()?;
    let policy = SandboxPolicy {
        backend: super::super::types::SandboxBackendKind::Local,
        workspace_root: root.path().to_path_buf(),
        state_dir: root.path().to_path_buf(),
        read_only_mounts: Vec::new(),
        read_write_mounts: Vec::new(),
        allow_network: true,
        env_passthrough: Vec::new(),
        docker_overrides: None,
    };
    let extra = HashMap::from([(
        OsString::from("CUSTOM_SAFE"),
        OsString::from("value"),
    )]);
    let (env, read_write) = execution_environment(&policy, &extra, scratch.path())?;

    assert_eq!(env.get("CUSTOM_SAFE").map(String::as_str), Some("value"));
    assert_eq!(env.get("TMPDIR").map(String::as_str), scratch.path().to_str());
    assert!(read_write.iter().any(|path| path == scratch.path().to_str().unwrap()));
    assert!(!env.keys().any(|name| name.contains("TOKEN") || name.contains("SECRET")));
    Ok(())
}
