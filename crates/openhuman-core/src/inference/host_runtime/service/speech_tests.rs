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

    assert!(
        error.contains("text is required"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn tts_rejects_output_outside_allowed_roots() {
    let tmp = tempfile::tempdir().unwrap();
    let config = enabled_config(&tmp);
    let service = LocalAiService::new(&crate::inference::local_runtime_config(&config));
    let output = tmp.path().join("outside").join("speech.wav");

    let error = tts(&service, &config, "hello", Some(output.to_str().unwrap()))
        .await
        .unwrap_err();

    assert!(error.contains("output_path"), "unexpected error: {error}");
    assert!(!output.parent().unwrap().exists());
}

#[tokio::test]
async fn tts_output_path_allows_workspace_and_local_ai_models_roots() {
    let tmp = tempfile::tempdir().unwrap();
    let mut config = enabled_config(&tmp);
    config.autonomy.enabled = true;
    config.autonomy.workspace_only = true;
    config.action_dir = tmp.path().join("action");
    std::fs::create_dir_all(&config.action_dir).unwrap();
    let runtime = crate::inference::local_runtime_config(&config);
    let model_root = tinyinference_local::service::paths::config_root_dir(&runtime)
        .join("models")
        .join("local-ai");

    for output in [
        config.workspace_dir.join("speech.wav"),
        model_root.join("speech.wav"),
    ] {
        let resolved = validated_tts_output_path(&config, &runtime, Some(output.to_str().unwrap()))
            .await
            .unwrap();
        assert!(resolved.ends_with("speech.wav"));
    }
}

#[tokio::test]
async fn tts_output_path_rejects_traversal() {
    let tmp = tempfile::tempdir().unwrap();
    let config = enabled_config(&tmp);
    let runtime = crate::inference::local_runtime_config(&config);
    let traversal = config.workspace_dir.join("../outside/speech.wav");

    let error = validated_tts_output_path(&config, &runtime, Some(traversal.to_str().unwrap()))
        .await
        .unwrap_err();

    assert!(error.contains("output_path"));
}

#[cfg(unix)]
#[tokio::test]
async fn tts_output_path_rejects_symlink_escape() {
    let tmp = tempfile::tempdir().unwrap();
    let config = enabled_config(&tmp);
    let runtime = crate::inference::local_runtime_config(&config);
    let outside = tmp.path().join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    let link = config.workspace_dir.join("linked");
    std::os::unix::fs::symlink(&outside, &link).unwrap();
    let output = link.join("speech.wav");

    let error = validated_tts_output_path(&config, &runtime, Some(output.to_str().unwrap()))
        .await
        .unwrap_err();

    assert!(error.contains("output_path"));
}

#[cfg(unix)]
#[tokio::test]
async fn tts_output_path_rejects_final_component_symlink() {
    let tmp = tempfile::tempdir().unwrap();
    let config = enabled_config(&tmp);
    let runtime = crate::inference::local_runtime_config(&config);
    let outside_file = tmp.path().join("outside.wav");
    std::fs::write(&outside_file, b"outside target").unwrap();
    let output = config.workspace_dir.join("speech.wav");
    std::os::unix::fs::symlink(&outside_file, &output).unwrap();

    let error = validated_tts_output_path(&config, &runtime, Some(output.to_str().unwrap()))
        .await
        .unwrap_err();

    assert!(
        error.contains("symbolic links"),
        "unexpected error: {error}"
    );
    assert_eq!(std::fs::read(outside_file).unwrap(), b"outside target");
}

#[cfg(unix)]
#[tokio::test]
async fn tts_output_path_keeps_always_forbidden_paths_blocked() {
    let tmp = tempfile::tempdir().unwrap();
    let config = enabled_config(&tmp);
    let runtime = crate::inference::local_runtime_config(&config);

    let error = validated_tts_output_path(&config, &runtime, Some("/etc/openhuman-tts-test.wav"))
        .await
        .unwrap_err();

    assert!(error.contains("output_path"));
}
