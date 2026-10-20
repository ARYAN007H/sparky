//! Text embedding models, re-exported for hosts that index or search.
//!
//! The same models the core's memory layer embeds with, exposed directly so a
//! host that keeps its own vector index (a document-search service, a retrieval
//! step) depends on one OpenHuman pin rather than a second copy of
//! the inference crates.
//!
//! These are re-exports, not wrappers, on purpose: the embedding signature
//! ([`format_embedding_signature`], [`EmbeddingModel::signature`]) decides
//! which partition a stored vector belongs to, so a facade that reshaped it
//! would silently move every host's existing vectors into a fresh, empty
//! space. Re-exporting keeps the one source of truth.
//!
//! [`set_rate_limit`] is process-wide, like the provider quota it guards.
//! Unlike [`Completer`](crate::complete::Completer), these models need no
//! [`Runtime`](crate::Runtime).

pub use tinyinference_embeddings::{
    COHERE_API_BASE, COHERE_DEFAULT_DIMENSIONS, COHERE_DEFAULT_MODEL, CohereEmbeddingModel,
    DEFAULT_OLLAMA_DIMENSIONS, DEFAULT_OLLAMA_MODEL, DEFAULT_OLLAMA_URL,
    DEFAULT_REQUESTS_PER_MINUTE, EmbeddingModel, MockEmbeddingModel, NoopEmbeddingModel,
    OllamaEmbeddingModel, OpenAiEmbeddingModel, VOYAGE_API_BASE, VOYAGE_DEFAULT_DIMENSIONS,
    VOYAGE_DEFAULT_MODEL, VoyageEmbeddingModel, format_embedding_signature, set_rate_limit,
};
/// Errors the embedding models return.
pub use tinyinference_embeddings::{Error as EmbeddingError, Result as EmbeddingResult};

#[cfg(test)]
#[path = "embeddings_tests.rs"]
mod tests;
