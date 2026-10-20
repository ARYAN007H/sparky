use super::*;

/// A bare-core first install persists the generic defaults and remains stable
/// when reopened. Hosted provider seeding is covered by TinyHumans' registered
/// normalizer tests, not by core's config creation path.
#[tokio::test]
async fn a_brand_new_workspace_persists_and_reopens_generic_config() {
    let tmp = tempfile::tempdir().unwrap();
    let first = load_or_init_for_workspace(tmp.path()).await;
    assert!(first.default_model.is_none());
    assert!(first
        .cloud_providers
        .iter()
        .all(|provider| provider.slug != "openhuman"));

    let first_text = tokio::fs::read_to_string(&first.config_path).await.unwrap();
    assert!(first_text.contains("schema_version"));

    let second = load_or_init_for_workspace(tmp.path()).await;
    assert!(second.default_model.is_none());
    assert!(second
        .cloud_providers
        .iter()
        .all(|provider| provider.slug != "openhuman"));
}

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
    assert_eq!(
        config.schema_version,
        crate::config::migrations::CURRENT_SCHEMA_VERSION
    );
    assert_eq!(config.default_model.as_deref(), Some("reasoning-v1"));
}
