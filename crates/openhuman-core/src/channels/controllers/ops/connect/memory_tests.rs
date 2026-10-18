use super::*;
use std::sync::Arc;
use tinymemory_api::conformance::ReferenceEngine;
use tinymemory_api::{MemoryEngine, MemoryMeta, Role, SourceKind, StoreItem, Turn};
use crate::core::runtime::{ContextOverlay, CoreContext, DispatchCtx, DomainSet};
use crate::threads::store::{ConversationMessage, CreateConversationThread};
use serde_json::json;

fn conversation(thread: &str, channel: Option<&str>) -> StoreItem {
    let mut meta = MemoryMeta::from_source(SourceKind::Conversation, Some(thread.to_string()));
    meta.thread_id = Some(thread.to_string());
    meta.channel = channel.map(str::to_string);
    StoreItem::Conversation {
        turns: vec![Turn::new(Role::User, format!("hello from {thread}"))],
        meta,
    }
}

fn account_context(workspace: &std::path::Path, profile: &str) -> DispatchCtx {
    let config = Config {
        workspace_dir: workspace.to_path_buf(),
        action_dir: workspace.to_path_buf(),
        config_path: workspace.join("config.toml"),
        ..Config::default()
    };
    DispatchCtx::new(
        CoreContext::for_test(DomainSet::none(), Some(workspace.to_path_buf())).derive_with(
            ContextOverlay::new(config, DomainSet::full(), Default::default()).profile(profile),
        ),
    )
}

async fn store_channel_thread(
    ctx: DispatchCtx,
    workspace: &std::path::Path,
    channel: &str,
    message_id: &str,
) -> String {
    let params = crate::channels::providers::relay::RelayInboundParams::new(
        channel,
        "chat-1",
        "sender-1",
        message_id,
        "hello",
    );
    let thread_id = params.thread_id();
    let now = chrono::Utc::now().to_rfc3339();
    crate::threads::store::blocking::ensure_thread(
        ctx.clone(),
        workspace.to_path_buf(),
        CreateConversationThread {
            id: thread_id.clone(),
            title: params.thread_title(),
            created_at: now.clone(),
            parent_thread_id: None,
            labels: Some(vec!["general".into()]),
            personality_id: None,
            working_dir: None,
        },
    )
    .await
    .unwrap();
    crate::threads::store::blocking::append_message(
        ctx,
        workspace.to_path_buf(),
        thread_id.clone(),
        ConversationMessage {
            id: format!("user:{message_id}"),
            content: "hello".into(),
            message_type: "text".into(),
            extra_metadata: json!({"scope": "channel", "channel": channel}),
            sender: "user".into(),
            created_at: now,
        },
    )
    .await
    .unwrap();
    thread_id
}

#[tokio::test]
async fn clears_stamped_and_legacy_channel_conversations_without_mutating_legacy_map() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mut config = Config::default();
    config.workspace_dir = tmp.path().to_path_buf();
    let engine = Arc::new(ReferenceEngine::new());
    crate::memory::engine::install_test_engine(&config.workspace_dir, engine.clone());
    let discord_thread = store_channel_thread(
        DispatchCtx::ambient(),
        &config.workspace_dir,
        "discord",
        "discord-current",
    )
    .await;
    store_channel_thread(
        DispatchCtx::ambient(),
        &config.workspace_dir,
        "telegram",
        "telegram-current",
    )
    .await;
    engine
        .store(conversation(&discord_thread, Some("discord")))
        .await
        .unwrap();
    engine
        .store(conversation("discord-legacy", None))
        .await
        .unwrap();
    engine
        .store(conversation("telegram-current", Some("telegram")))
        .await
        .unwrap();
    let legacy_file = config.workspace_dir.join("memory/channel_threads.json");
    std::fs::create_dir_all(legacy_file.parent().unwrap()).unwrap();
    let legacy_bytes = br#"{"discord":["discord-legacy"]}"#;
    std::fs::write(&legacy_file, legacy_bytes).unwrap();

    assert_eq!(clear_channel_memory(&config, "discord").await.unwrap(), 2);
    assert_eq!(engine.len(), 1, "the other channel remains");
    assert_eq!(std::fs::read(&legacy_file).unwrap(), legacy_bytes);
}

#[tokio::test]
async fn stamped_channel_forget_isolated_to_the_configured_account() {
    let current_tmp = tempfile::TempDir::new().unwrap();
    let other_tmp = tempfile::TempDir::new().unwrap();
    let mut current = Config::default();
    current.workspace_dir = current_tmp.path().to_path_buf();
    let mut other = Config::default();
    other.workspace_dir = other_tmp.path().to_path_buf();
    let current_ctx = account_context(&current.workspace_dir, "account-a");
    let other_ctx = account_context(&other.workspace_dir, "account-b");
    let current_engine = Arc::new(ReferenceEngine::new());
    let other_engine = Arc::new(ReferenceEngine::new());
    crate::memory::engine::install_test_engine(&current.workspace_dir, current_engine.clone());
    crate::memory::engine::install_test_engine(&other.workspace_dir, other_engine.clone());
    let current_thread = store_channel_thread(
        current_ctx.clone(),
        &current.workspace_dir,
        "discord",
        "same-message-id",
    )
    .await;
    let other_thread = store_channel_thread(
        other_ctx.clone(),
        &other.workspace_dir,
        "discord",
        "same-message-id",
    )
    .await;
    current_engine
        .store(conversation(&current_thread, Some("discord")))
        .await
        .unwrap();
    other_engine
        .store(conversation(&other_thread, Some("discord")))
        .await
        .unwrap();

    assert_eq!(
        CoreContext::scope(current_ctx.into_core().unwrap(), async {
            clear_channel_memory(&current, "discord").await.unwrap()
        })
        .await,
        1
    );
    assert!(current_engine.is_empty());
    assert_eq!(other_engine.len(), 1);
}

#[tokio::test]
async fn missing_or_corrupt_legacy_map_does_not_block_stamped_forget() {
    for corrupt in [false, true] {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut config = Config::default();
        config.workspace_dir = tmp.path().to_path_buf();
        let engine = Arc::new(ReferenceEngine::new());
        crate::memory::engine::install_test_engine(&config.workspace_dir, engine.clone());
        let thread_id = store_channel_thread(
            DispatchCtx::ambient(),
            &config.workspace_dir,
            "discord",
            "discord-current",
        )
        .await;
        engine
            .store(conversation(&thread_id, Some("discord")))
            .await
            .unwrap();
        let legacy_file = config.workspace_dir.join("memory/channel_threads.json");
        let corrupt_bytes = b"{not-json";
        if corrupt {
            std::fs::create_dir_all(legacy_file.parent().unwrap()).unwrap();
            std::fs::write(&legacy_file, corrupt_bytes).unwrap();
        }

        assert_eq!(clear_channel_memory(&config, "discord").await.unwrap(), 1);
        assert!(engine.is_empty());
        if corrupt {
            assert_eq!(std::fs::read(&legacy_file).unwrap(), corrupt_bytes);
            assert!(!legacy_file.with_extension("json.corrupt").exists());
        } else {
            assert!(!legacy_file.exists());
        }
    }
}

#[tokio::test]
async fn memory_off_queues_channel_threads_from_the_canonical_thread_store() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mut config = Config::default();
    config.workspace_dir = tmp.path().to_path_buf();
    config.memory.engine.clear();
    let thread_id = store_channel_thread(
        DispatchCtx::ambient(),
        &config.workspace_dir,
        "discord",
        "discord-offline",
    )
    .await;

    assert_eq!(clear_channel_memory(&config, "discord").await.unwrap(), 0);
    assert_eq!(
        crate::memory::deletion::pending(&config.workspace_dir),
        vec![crate::memory::deletion::PendingDeletion::Thread { thread_id }]
    );
}

#[tokio::test]
async fn memory_off_clears_nothing() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mut config = Config::default();
    config.workspace_dir = tmp.path().to_path_buf();
    config.memory.engine = String::new();
    assert_eq!(clear_channel_memory(&config, "discord").await.unwrap(), 0);
}
