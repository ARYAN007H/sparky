//! Live embedding through the configured provider, and the connectivity test.

use crate::config::Config;
use crate::core::Outcome;

use super::{resolve_api_key, LOG_PREFIX};
use crate::inference::embedding_host::factory::{
    credential_resolver, managed_factory, settings, create_embedding_provider_with_config,
};
use tinyinference_embeddings::verify::test_embedding_connection;
use tinyinference_providers::route::split_custom_provider;

/// Generates embeddings for the given input texts using the currently
/// configured provider.
pub async fn embed(
    config: &Config,
    inputs: &[String],
) -> Result<Outcome<serde_json::Value>, String> {
    let provider_name = &config.memory.embedding_provider;
    let model = &config.memory.embedding_model;
    let dims = config.memory.embedding_dimensions;

    let api_key = resolve_api_key(config, provider_name);

    let (provider_slug, custom_endpoint) = split_custom_provider(provider_name);

    let provider_slug = if custom_endpoint.is_some() {
        "custom"
    } else {
        provider_slug.as_str()
    };

    let embedder = create_embedding_provider_with_config(
        config,
        provider_slug,
        model,
        dims,
        &api_key,
        custom_endpoint.as_deref(),
    )
    .map_err(|e| e.to_string())?;

    let vectors = embedder.embed(inputs).await.map_err(|e| e.to_string())?;

    let actual_dims = vectors.first().map(|v| v.len()).unwrap_or(0);

    tracing::debug!(
        provider = provider_slug,
        model,
        input_count = inputs.len(),
        vector_count = vectors.len(),
        dims = actual_dims,
        "{LOG_PREFIX} embed completed"
    );

    let payload = serde_json::json!({
        "vectors": vectors,
        "dimensions": actual_dims,
        "count": vectors.len(),
        "provider": provider_slug,
        "model": model,
    });

    Ok(Outcome::new(payload, vec!["embedding completed".into()]))
}

/// Tests connectivity to the configured (or specified) embedding provider.
pub async fn test_connection(
    config: &Config,
    provider_slug: Option<&str>,
    model: Option<&str>,
    dims: Option<usize>,
) -> Result<Outcome<serde_json::Value>, String> {
    let slug = provider_slug.unwrap_or(&config.memory.embedding_provider);
    let model = model.unwrap_or(&config.memory.embedding_model);
    let dims = dims.unwrap_or(config.memory.embedding_dimensions);

    let (provider_slug, custom_endpoint) = split_custom_provider(slug);

    let provider_tag = if custom_endpoint.is_some() {
        "custom"
    } else {
        provider_slug.as_str()
    };

    tracing::debug!(
        provider = provider_tag,
        model,
        dims,
        "{LOG_PREFIX} test_connection starting"
    );

    let selected = settings(
        if custom_endpoint.is_some() {
            &format!("custom:{}", custom_endpoint.as_deref().unwrap_or_default())
        } else {
            provider_tag
        },
        model,
        dims,
        tinyinference_local::ollama::ollama_base_url_from_override(
            config.local_ai.base_url.as_deref(),
        ),
    );
    let credentials = credential_resolver(config);
    let managed = managed_factory(Some(config.clone()), None);
    let result = test_embedding_connection(
        &selected,
        &credentials,
        &managed,
        custom_endpoint.as_deref(),
    )
    .await
    .map(|result| result.actual_dims)
    .map_err(|error| error.to_string());

    match result {
        Ok(actual_dims) => {
            let payload = serde_json::json!({
                "success": true,
                "provider": provider_tag,
                "model": model,
                "requested_dimensions": dims,
                "actual_dimensions": actual_dims,
            });
            Ok(Outcome::new(payload, vec!["connection test passed".into()]))
        }
        Err(e) => {
            let payload = serde_json::json!({
                "success": false,
                "provider": provider_tag,
                "model": model,
                "error": e.to_string(),
            });
            Ok(Outcome::new(
                payload,
                vec![format!("connection test failed: {e}")],
            ))
        }
    }
}
