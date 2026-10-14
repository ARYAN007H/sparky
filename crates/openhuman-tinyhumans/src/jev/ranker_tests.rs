use tinytools::{RankCandidate, RankContext, RankError, ToolRanker};

use super::*;

#[test]
fn kind_is_jev_and_debug_hides_the_client() {
    let ranker = TinyHumansJevRanker::new();
    assert_eq!(ranker.kind(), "jev");
    let debug = format!("{ranker:?}");
    assert!(debug.contains("TinyHumansJevRanker"));
    assert!(!debug.contains("api_key"));
}

#[test]
fn fingerprint_changes_with_secret_or_base_url() {
    let a = fingerprint("secret", "https://api.tinyhumans.ai");
    assert_eq!(a, fingerprint("secret", "https://api.tinyhumans.ai"));
    assert_ne!(a, fingerprint("other", "https://api.tinyhumans.ai"));
    assert_ne!(a, fingerprint("secret", "https://staging.tinyhumans.ai"));
    assert!(!fingerprint_changed(None, a));
}

/// Without a credential the ranker must answer with a `Backend` error the
/// harness turns into a BM25 fallback — never a panic, never a network call.
#[tokio::test]
async fn no_credential_is_a_backend_error_naming_the_gap() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let config = Config {
        workspace_dir: tmp.path().join("workspace"),
        action_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("config.toml"),
        ..Config::default()
    };
    // No env keys: a developer's shell may export OPENROUTER_API_KEY, and `auto`
    // would then (correctly) route through it.
    let ranker = TinyHumansJevRanker::new()
        .with_env_loader(Arc::new(|_| None))
        .with_config_loader(Arc::new(move || {
            let config = config.clone();
            Box::pin(async move { Ok(config) })
        }));
    let err = ranker
        .rank(
            "send a message",
            &RankContext::empty(),
            &[RankCandidate::new("a", "a send a message")],
            3,
        )
        .await
        .expect_err("no credential must fail");
    match err {
        RankError::Backend { reason } => {
            assert!(reason.contains("no TinyHumans credential"), "{reason}");
        }
        other => panic!("expected a backend error, got {other}"),
    }
}

/// A config whose embedding provider is `none` disables the Jev search
/// outright — the harness's BM25 answers — rather than quietly retrieving
/// lexically.
#[test]
fn no_embedding_provider_disables_the_search() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut config = Config {
        workspace_dir: tmp.path().join("workspace"),
        action_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("config.toml"),
        ..Config::default()
    };
    config.memory.embedding_provider = "none".into();
    let err = retriever_for(&config, None, &default_embedder()).err().map(|e| e.to_string());
    assert!(
        err.as_deref()
            .is_some_and(|e| e.contains("no usable embedding provider")),
        "{err:?}"
    );
}

fn quiet_config(tmp: &tempfile::TempDir) -> Config {
    Config {
        workspace_dir: tmp.path().join("workspace"),
        action_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("config.toml"),
        ..Config::default()
    }
}

/// An embedder that records the chat each provider was built for and embeds
/// nothing (the `none` provider), so a search stops at "no usable embedding
/// provider" after the retriever was requested.
fn recording_embedder(seen: Arc<std::sync::Mutex<Vec<Option<ApprovalChatContext>>>>) -> EmbedderFactory {
    Arc::new(move |config, chat| {
        seen.lock().unwrap().push(chat.clone());
        let mut quiet = config.clone();
        quiet.memory.embedding_provider = "none".into();
        openhuman_embed::__host::inference::embedding_host::default_embedding_provider_for_chat(
            &quiet, chat,
        )
    })
}

/// The Jev retriever's cloud-embedding disclosure goes to the chat the search
/// runs for: the chat in the `RankContext` reaches the embedding provider
/// factory, and a search with no chat builds an unrouted provider.
#[tokio::test]
async fn the_chat_of_the_search_reaches_the_embedding_provider() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let config = quiet_config(&tmp);
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let ranker = TinyHumansJevRanker::new()
        .with_embedder_factory(recording_embedder(seen.clone()))
        .with_env_loader(Arc::new(|key| {
            (key == "OPENROUTER_API_KEY").then(|| "sk-test".to_owned())
        }))
        .with_config_loader(Arc::new(move || {
            let config = config.clone();
            Box::pin(async move { Ok(config) })
        }));
    let candidates = [RankCandidate::new("a", "a send a message")];

    let chat = tinytools::RankChat::new("thread-9", "client-4").with_request_id("req-2");
    let routed = RankContext::empty().with_chat(chat);
    let _ = ranker.rank("send a message", &routed, &candidates, 3).await;
    let _ = ranker
        .rank("send a message", &RankContext::empty(), &candidates, 3)
        .await;

    let seen = seen.lock().unwrap().clone();
    assert_eq!(
        seen.first().cloned().flatten(),
        Some(ApprovalChatContext {
            thread_id: "thread-9".to_owned(),
            client_id: "client-4".to_owned(),
            request_id: Some("req-2".to_owned()),
        }),
        "the first search ran in a chat: {seen:?}"
    );
    assert!(
        seen.iter().skip(1).all(Option::is_none),
        "a search with no chat builds an unrouted provider: {seen:?}"
    );
}
