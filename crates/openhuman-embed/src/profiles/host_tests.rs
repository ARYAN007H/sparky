//! Immutable profile host configuration rejects broader capability requests.

use super::*;

#[test]
fn wildcard_profile_scope_is_refused_before_boot() {
    let config = SaasConfig::new("/srv/openhuman");
    let options = ProfileHostOptions {
        scope: Some(ToolScopeSpec::Wildcard),
        ..Default::default()
    };
    assert!(
        matches!(options.finish(&config), Err(ProfileError::Boot(message)) if message.contains("named or host-only"))
    );
}

#[test]
fn destructive_channel_ceiling_is_refused_before_boot() {
    let config = SaasConfig::new("/srv/openhuman");
    let options = ProfileHostOptions {
        channels: HashMap::from([("telegram".into(), PermissionLevel::Dangerous)]),
        ..Default::default()
    };
    assert!(
        matches!(options.finish(&config), Err(ProfileError::Boot(message)) if message.contains("dangerous"))
    );
}

#[test]
fn named_selection_never_overrides_hard_denies_or_default_readonly() {
    let config = SaasConfig::new("/srv/openhuman");
    let options = ProfileHostOptions {
        scope: Some(ToolScopeSpec::Named(vec![
            "memory".into(),
            "shell".into(),
            "delegate".into(),
            "curl".into(),
            "run_workflow".into(),
        ])),
        ..Default::default()
    };
    let capabilities = options.finish(&config).unwrap();
    assert!(capabilities.admits("memory"));
    for name in ["shell", "delegate", "curl", "run_workflow", "hidden"] {
        assert!(!capabilities.admits(name), "{name}");
    }
    assert_eq!(
        capabilities.channel_permission("telegram"),
        PermissionLevel::ReadOnly
    );
    assert!(!capabilities.authorizes_approval("memory"));
}
