use super::*;
use tempfile::TempDir;

#[tokio::test]
async fn cached_probe_serves_definitive_negative_results() {
    use crate::core::runtime::{CoreContext, DispatchCtx, DomainSet};
    use crate::core::slots::Extensions;
    use crate::inference::registry::{register_chat_model_in, ChatModelEntry, ProviderModelHooks};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tinyinference_llm::model::ChatModel;

    let tmp = TempDir::new().unwrap();
    let mut config = Config {
        workspace_dir: tmp.path().join("workspace"),
        action_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("config.toml"),
        ..Config::default()
    };
    let hit_count = std::sync::Arc::new(AtomicUsize::new(0));
    let counter = hit_count.clone();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = axum::Router::new().route(
        "/openai/v1/chat/completions",
        axum::routing::post(move || {
            let counter = counter.clone();
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                use axum::response::IntoResponse;
                (
                    axum::http::StatusCode::BAD_REQUEST,
                    axum::Json(serde_json::json!({
                        "success": false,
                        "error": "API key not configured for provider",
                        "errorCode": "BAD_REQUEST"
                    })),
                )
                    .into_response()
            }
        }),
    );
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    config.api_url = Some(format!("http://{addr}"));

    let extensions = Extensions::new();
    register_chat_model_in(
        &extensions,
        crate::inference::provider::factory::PROVIDER_OPENHUMAN,
        ChatModelEntry {
            build: Arc::new(|_| {
                Ok((
                    Arc::new(tinyagents_harness::testkit::ScriptedModel::replies([
                        "ready",
                    ])),
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
}
