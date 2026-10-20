use super::*;

#[test]
fn register_all_is_idempotent() {
    let _registry = crate::registry_test_lock();

    let options = InstallOptions::default();
    register_all(&options);
    register_all(&options);
}

#[tokio::test]
async fn boot_registration_supplies_hosted_model_default_and_normalizer_before_context_exists() {
    let _registry = crate::registry_test_lock();
    register_all(&InstallOptions::default());

    let config = openhuman_embed::__host::config::Config::default();
    assert_eq!(
        config.default_model.as_deref(),
        Some(crate::inference::tiers::MODEL_MANAGED_DEFAULT)
    );

    let mut legacy = openhuman_embed::__host::config::Config::default();
    legacy.default_model = Some("reasoning-v1".to_string());
    let tmp = tempfile::tempdir().unwrap();
    legacy.workspace_dir = tmp.path().join("workspace");
    legacy.config_path = tmp.path().join("config.toml");
    tokio::fs::create_dir_all(&legacy.workspace_dir)
        .await
        .unwrap();
    openhuman_embed::__host::config::migrations::run_pending(&mut legacy).await;
    assert_eq!(
        legacy.default_model.as_deref(),
        Some(crate::inference::tiers::MODEL_MANAGED_DEFAULT)
    );
}
