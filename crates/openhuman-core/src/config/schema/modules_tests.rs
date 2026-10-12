use super::*;

#[test]
fn source_defaults_to_release() {
    assert_eq!(ModulesConfig::default().source, ModuleSourceMode::Release);
}

#[test]
fn legacy_path_overrides_still_parse() {
    let config: ModulesConfig = toml::from_str(
        r#"
        [[overrides]]
        id = "tinydocs"
        path = "/dev/libtinydocs.so"
        "#,
    )
    .expect("legacy {id, path} entry");
    assert_eq!(config.source, ModuleSourceMode::Release);
    assert_eq!(config.overrides[0].path.as_deref(), Some("/dev/libtinydocs.so"));
    assert_eq!(config.overrides[0].source, None);
    assert!(config.overrides[0].validate().is_ok());
}

#[test]
fn source_modes_parse_from_config() {
    let config: ModulesConfig = toml::from_str(
        r#"
        source = "linked"
        [[overrides]]
        id = "tinydocs"
        source = "source"
        "#,
    )
    .expect("source config");
    assert_eq!(config.source, ModuleSourceMode::Linked);
    assert_eq!(config.overrides[0].source, Some(ModuleSourceMode::Source));
}

#[test]
fn unknown_override_fields_are_rejected() {
    let parsed: Result<ModulesConfig, _> = toml::from_str(
        r#"
        [[overrides]]
        id = "tinydocs"
        sha256 = "00"
        "#,
    );
    assert!(parsed.is_err());
}

#[test]
fn an_override_needs_exactly_one_of_path_and_source() {
    let both = ModuleOverride {
        id: "x".into(),
        path: Some("p".into()),
        source: Some(ModuleSourceMode::Source),
    };
    assert!(both.validate().is_err());
    let neither = ModuleOverride {
        id: "x".into(),
        path: None,
        source: None,
    };
    assert!(neither.validate().is_err());
}

#[test]
fn mode_names_round_trip() {
    for mode in [
        ModuleSourceMode::Release,
        ModuleSourceMode::Linked,
        ModuleSourceMode::Source,
    ] {
        assert_eq!(ModuleSourceMode::parse(mode.as_str()), Some(mode));
    }
    assert_eq!(ModuleSourceMode::parse(" LINKED "), Some(ModuleSourceMode::Linked));
    assert_eq!(ModuleSourceMode::parse("nope"), None);
}
