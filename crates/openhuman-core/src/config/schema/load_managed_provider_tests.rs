use super::*;

#[tokio::test]
async fn bare_core_advances_schema_without_inventing_hosted_defaults() {
    let tmp = tempfile::tempdir().unwrap();
    let mut config = load_or_init_for_workspace(tmp.path()).await;
    assert!(config.default_model.is_none());
    assert!(config
        .cloud_providers
        .iter()
        .all(|provider| provider.slug != "openhuman"));

    config.schema_version = 7;
    config.default_model = Some("reasoning-v1".to_string());
    crate::config::migrations::run_pending(&mut config).await;
    assert_eq!(config.schema_version, crate::config::migrations::CURRENT_SCHEMA_VERSION);
    assert_eq!(config.default_model.as_deref(), Some("reasoning-v1"));
}
