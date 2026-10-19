use super::*;

#[tokio::test]
async fn choosing_backend_resets_a_legacy_nondefault_stt_engine() {
    let _guard = crate::config::TEST_ENV_LOCK.lock().await;
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    std::fs::write(
        &config_path,
        r#"[voice_server]
stt_engine = "elevenlabs"
"#,
    )
    .unwrap();
    unsafe {
        std::env::set_var("OPENHUMAN_WORKSPACE", dir.path());
    }

    let params = Map::from_iter([("stt_provider".to_string(), Value::String("cloud".into()))]);
    handle_voice_set_providers(DispatchCtx::process_default(), params)
        .await
        .expect("setting the backend provider should succeed");

    let config = crate::config::Config::load_from_config_path(&config_path, dir.path())
        .await
        .unwrap();
    assert_eq!(config.voice_server.stt_engine, crate::config::SttEngine::Backend);
    assert_eq!(crate::voice::effective_stt_provider(&config), "cloud");

    unsafe {
        std::env::remove_var("OPENHUMAN_WORKSPACE");
    }
}
