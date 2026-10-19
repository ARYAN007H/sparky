use super::*;
use tinyinference_embeddings::format_embedding_signature;

async fn endpoint() -> (String, tokio::task::JoinHandle<()>) {
    use axum::{routing::post, Json, Router};

    let app = Router::new().route(
        "/v1/embeddings",
        post(|| async {
            Json(serde_json::json!({
                "object": "list",
                "data": [{
                    "object": "embedding",
                    "index": 0,
                    "embedding": [0.1_f32, 0.2, 0.3, 0.4]
                }],
                "model": "bge-m3"
            }))
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!(
        "http://127.0.0.1:{}/v1",
        listener.local_addr().unwrap().port()
    );
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (base, server)
}

async fn config_context(provider: String) -> (tempfile::TempDir, DispatchCtx) {
    let tmp = tempfile::TempDir::new().unwrap();
    let mut config = Config::default();
    config.workspace_dir = tmp.path().to_path_buf();
    config.config_path = tmp.path().join("config.toml");
    config.memory.embedding_provider = provider;
    config.memory.embedding_model = "bge-m3".to_string();
    config.memory.embedding_dimensions = 4;
    config.save().await.unwrap();
    let core = crate::core::runtime::CoreContext::for_test_with_config(
        crate::core::runtime::DomainSet::full(),
        config,
    );
    (tmp, DispatchCtx::new(core))
}

async fn reloaded_config(tmp: &tempfile::TempDir) -> Config {
    Config::load_from_config_path(&tmp.path().join("config.toml"), tmp.path())
        .await
        .unwrap()
}

#[tokio::test]
async fn endpoint_only_custom_change_reports_persisted_provider_signature() {
    let (endpoint, server) = endpoint().await;
    let (tmp, ctx) = config_context("custom:http://127.0.0.1:1/v1".to_string()).await;

    let outcome = update_settings(
        &ctx,
        None,
        None,
        None,
        Some(endpoint.clone()),
        None,
    )
    .await
    .unwrap();
    let persisted = reloaded_config(&tmp).await;
    let expected = format_embedding_signature(
        &persisted.memory.embedding_provider,
        &persisted.memory.embedding_model,
        persisted.memory.embedding_dimensions,
    );

    assert_eq!(persisted.memory.embedding_provider, format!("custom:{endpoint}"));
    assert_eq!(outcome.value["signature_changed"], true);
    assert_eq!(outcome.value["new_signature"], expected);
    server.abort();
}

#[tokio::test]
async fn explicit_custom_provider_with_same_endpoint_keeps_signature_unchanged() {
    let (endpoint, server) = endpoint().await;
    let (_tmp, ctx) = config_context(format!("custom:{endpoint}")).await;

    let outcome = update_settings(
        &ctx,
        Some("custom".to_string()),
        None,
        None,
        Some(endpoint),
        None,
    )
    .await
    .unwrap();

    assert_eq!(outcome.value["signature_changed"], false);
    server.abort();
}

#[tokio::test]
async fn auto_detected_dimensions_are_used_in_persisted_provider_signature() {
    let (endpoint, server) = endpoint().await;
    let (tmp, ctx) = config_context("custom:http://127.0.0.1:1/v1".to_string()).await;

    let outcome = update_settings(
        &ctx,
        None,
        None,
        Some(8),
        Some(endpoint.clone()),
        None,
    )
    .await
    .unwrap();
    let persisted = reloaded_config(&tmp).await;
    let expected = format_embedding_signature(
        &persisted.memory.embedding_provider,
        &persisted.memory.embedding_model,
        persisted.memory.embedding_dimensions,
    );

    assert_eq!(persisted.memory.embedding_provider, format!("custom:{endpoint}"));
    assert_eq!(persisted.memory.embedding_dimensions, 4);
    assert_eq!(outcome.value["signature_changed"], true);
    assert_eq!(outcome.value["new_signature"], expected);
    server.abort();
}
