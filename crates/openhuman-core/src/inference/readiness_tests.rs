use super::*;
use tempfile::TempDir;

#[tokio::test]
async fn cached_probe_serves_definitive_negative_results() {
    use crate::core::runtime::{CoreContext, DispatchCtx, DomainSet};
    use crate::core::slots::Extensions;
    use crate::inference::registry::{register_chat_model_in, ChatModelEntry, ProviderModelHooks};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let _model_override_guard =
        crate::inference::provider::factory::test_provider_override::exclusive();
    let tmp = TempDir::new().unwrap();
    let config = Config {
        workspace_dir: tmp.path().join("workspace"),
        action_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("config.toml"),
        ..Config::default()
    };
    let hit_count = std::sync::Arc::new(AtomicUsize::new(0));
    // The core owns no hosted provider; the managed backend's readiness probe
    // is whatever the host registered. Register one that rejects the way the
    // backend does, and count how often the core actually asks it.
    let _guard = crate::inference::inference_test_guard_async().await;
    let _exclusive = crate::inference::provider::factory::test_provider_override::exclusive();
    let counter = hit_count.clone();
    let managed = crate::inference::registry::chat_model(
        crate::inference::provider::factory::PROVIDER_OPENHUMAN,
    )
    .expect("the unit-test managed provider is available");
    crate::inference::registry::register_chat_model(
        crate::inference::provider::factory::PROVIDER_OPENHUMAN,
        crate::inference::registry::ChatModelEntry {
            build: managed.build,
            probe: Some(std::sync::Arc::new(move |_role, _model, _config| {
                counter.fetch_add(1, Ordering::SeqCst);
                Box::pin(async { Err("API key not configured for provider".to_string()) })
            })),
            models: None,
        },
    );

    let extensions = Extensions::new();
    register_chat_model_in(
        &extensions,
        crate::inference::provider::factory::PROVIDER_OPENHUMAN,
        ChatModelEntry {
            build: Arc::new(|_| {
                Ok((
                    Arc::new(tinyagents_harness::testkit::ScriptedModel::replies(
                        ["ready"].to_vec(),
                    )),
                    "fixture-model".to_string(),
                ))
            }),
            probe: Some(Arc::new(|_, _, config| {
                Box::pin(async move {
                    let base = config.api_url.as_deref().unwrap().trim_end_matches('/');
                    let response = reqwest::Client::new()
                        .post(format!("{base}/openai/v1/chat/completions"))
                        .json(&serde_json::json!({"model":"fixture-model","messages":[]}))
                        .send()
                        .await
                        .map_err(|error| error.to_string())?;
                    let status = response.status();
                    let body = response.text().await.map_err(|error| error.to_string())?;
                    if status.is_success() {
                        Ok(())
                    } else {
                        Err(body)
                    }
                })
            })),
            models: Some(ProviderModelHooks {
                default_model: Arc::new(|| Some("fixture-model".to_string())),
                role_for_model: Arc::new(|_| None),
                is_alias: Arc::new(|_| false),
                is_raw_model: Arc::new(|model| !model.trim().is_empty()),
                supports_vision: Arc::new(|_| false),
                resolve_model: Arc::new(|_, _, _| Some("fixture-model".to_string())),
                context_window: Arc::new(|_| None),
                default_pricing: Arc::new(|| None),
                without_reasoning: Arc::new(tinyinference_llm::model::without_reasoning),
            }),
        },
    );
    let core = CoreContext::for_test_with_extensions(
        DomainSet::full(),
        Some(config.workspace_dir.clone()),
        extensions,
    );
    let ctx = DispatchCtx::new(core);

    let first = cached_probe(&ctx, "summarization", &config).await;
    assert!(first
        .unwrap_err()
        .to_ascii_lowercase()
        .contains("api key not configured for provider"));
    assert_eq!(hit_count.load(Ordering::SeqCst), 1);
    assert!(cached_probe(&ctx, "summarization", &config).await.is_err());
    assert_eq!(hit_count.load(Ordering::SeqCst), 1);
    crate::inference::registry::unregister_chat_model(
        crate::inference::provider::factory::PROVIDER_OPENHUMAN,
    );
}
