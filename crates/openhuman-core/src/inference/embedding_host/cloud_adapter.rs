//! Credential scope and the unavailable placeholder for the managed cloud
//! embedder. The embedder itself is a registered provider
//! ([`crate::inference::registry::register_embedding`]); with none registered
//! the managed embedder is a placeholder that answers `BACKEND_UNAVAILABLE:`.

use tinyinference_embeddings::{EmbeddingModel, DEFAULT_CLOUD_DIMENSIONS, DEFAULT_CLOUD_MODEL};

pub const DEFAULT_CLOUD_EMBEDDING_MODEL: &str = DEFAULT_CLOUD_MODEL;
pub const DEFAULT_CLOUD_EMBEDDING_DIMENSIONS: usize = DEFAULT_CLOUD_DIMENSIONS;

/// The managed embedder when no provider is registered for it: keeps the
/// configured identity (name, model, dimensions) so callers that only describe
/// the embedder still work, and refuses to embed with the `BACKEND_UNAVAILABLE:`
/// sentinel.
pub struct UnavailableEmbeddingModel {
    model: String,
    dimensions: usize,
}

impl UnavailableEmbeddingModel {
    pub fn new(model: &str, dimensions: usize) -> Self {
        Self {
            model: model.to_string(),
            dimensions,
        }
    }
}

#[async_trait::async_trait]
impl EmbeddingModel for UnavailableEmbeddingModel {
    fn name(&self) -> &str {
        "cloud"
    }

    fn model_id(&self) -> &str {
        &self.model
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn signature(&self) -> String {
        format!("unavailable:{}:{}", self.model, self.dimensions)
    }

    async fn embed(&self, _texts: &[String]) -> tinyinference_embeddings::Result<Vec<Vec<f32>>> {
        Err(tinyinference_embeddings::Error::Validation(format!(
            "{} managed embeddings need a hosted provider; none is registered",
            crate::core::observability::BACKEND_UNAVAILABLE_PREFIX
        )))
    }
}

#[cfg(test)]
#[path = "cloud_adapter_tests.rs"]
mod tests;
