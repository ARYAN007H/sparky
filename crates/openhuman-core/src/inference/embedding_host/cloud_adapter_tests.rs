use super::*;

/// With no embedding provider registered, the managed embedder keeps its
/// identity and answers with the `BACKEND_UNAVAILABLE:` sentinel on use.
#[tokio::test]
async fn unregistered_managed_embedder_answers_backend_unavailable() {
    let provider = UnavailableEmbeddingModel::new(
        DEFAULT_CLOUD_EMBEDDING_MODEL,
        DEFAULT_CLOUD_EMBEDDING_DIMENSIONS,
    );
    assert_eq!(provider.name(), "cloud");
    assert_eq!(provider.dimensions(), DEFAULT_CLOUD_EMBEDDING_DIMENSIONS);
    let err = provider.embed(&["hello".to_string()]).await.unwrap_err();
    assert!(
        err.to_string()
            .contains(crate::core::observability::BACKEND_UNAVAILABLE_PREFIX),
        "unexpected error: {err}"
    );
}
