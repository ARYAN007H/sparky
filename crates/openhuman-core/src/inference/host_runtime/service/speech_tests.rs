use super::*;

fn enabled_config(tmp: &tempfile::TempDir) -> Config {
    let mut config = Config::default();
    config.workspace_dir = tmp.path().join("workspace");
    config.config_path = tmp.path().join("config.toml");
    config.local_ai.runtime_enabled = true;
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    config
}

#[tokio::test]
async fn tts_rejects_empty_text_before_resolving_piper() {
    let tmp = tempfile::tempdir().unwrap();
    let config = enabled_config(&tmp);
    let service = LocalAiService::new(&crate::inference::local_runtime_config(&config));

    let error = tts(&service, &config, " \n\t ", None).await.unwrap_err();

    assert!(error.contains("text is required"), "unexpected error: {error}");
}

#[tokio::test]
async fn tts_rejects_output_outside_allowed_roots() {
    let tmp = tempfile::tempdir().unwrap();
    let config = enabled_config(&tmp);
    let service = LocalAiService::new(&crate::inference::local_runtime_config(&config));
    let output = tmp.path().join("outside").join("speech.wav");

    let error = tts(
        &service,
        &config,
        "hello",
        Some(output.to_str().unwrap()),
    )
    .await
    .unwrap_err();

    assert!(error.contains("output_path"), "unexpected error: {error}");
    assert!(!output.parent().unwrap().exists());
}
