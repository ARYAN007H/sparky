use super::*;

#[tokio::test]
async fn choosing_backend_resets_a_legacy_nondefault_stt_engine() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = crate::config::Config::default();
    config.config_path = dir.path().join("config.toml");
    config.workspace_dir = dir.path().join("workspace");
    config.voice_server.stt_engine = crate::config::SttEngine::Elevenlabs;
    config.save().await.unwrap();
    let ctx = DispatchCtx::new(crate::core::runtime::CoreContext::for_test_with_config(
        crate::core::runtime::DomainSet::full(),
        config.clone(),
    ));

    let params = Map::from_iter([("stt_provider".to_string(), Value::String("cloud".into()))]);
    handle_voice_set_providers(ctx, params)
        .await
        .expect("setting the backend provider should succeed");

    let persisted =
        crate::config::Config::load_from_config_path(&config.config_path, &config.workspace_dir)
            .await
            .unwrap();
    assert_eq!(
        persisted.voice_server.stt_engine,
        crate::config::SttEngine::Backend
    );
    assert_eq!(crate::voice::effective_stt_provider(&persisted), "cloud");
}
