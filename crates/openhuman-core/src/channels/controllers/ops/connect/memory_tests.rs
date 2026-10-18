use super::*;
use std::sync::Arc;
use tinymemory_api::conformance::ReferenceEngine;
use tinymemory_api::{MemoryEngine, MemoryMeta, Role, SourceKind, StoreItem, Turn};

fn conversation(thread: &str, channel: Option<&str>) -> StoreItem {
    let mut meta = MemoryMeta::from_source(SourceKind::Conversation, Some(thread.to_string()));
    meta.thread_id = Some(thread.to_string());
    meta.channel = channel.map(str::to_string);
    StoreItem::Conversation {
        turns: vec![Turn::new(Role::User, format!("hello from {thread}"))],
        meta,
    }
}

#[tokio::test]
async fn clears_stamped_and_legacy_channel_conversations_without_mutating_legacy_map() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mut config = Config::default();
    config.workspace_dir = tmp.path().to_path_buf();
    let engine = Arc::new(ReferenceEngine::new());
    crate::memory::engine::install_test_engine(&config.workspace_dir, engine.clone());
    engine
        .store(conversation("discord-current", Some("discord")))
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
    let current_engine = Arc::new(ReferenceEngine::new());
    let other_engine = Arc::new(ReferenceEngine::new());
    crate::memory::engine::install_test_engine(&current.workspace_dir, current_engine.clone());
    crate::memory::engine::install_test_engine(&other.workspace_dir, other_engine.clone());
    current_engine
        .store(conversation("current-account", Some("discord")))
        .await
        .unwrap();
    other_engine
        .store(conversation("other-account", Some("discord")))
        .await
        .unwrap();

    assert_eq!(clear_channel_memory(&current, "discord").await.unwrap(), 1);
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
        engine
            .store(conversation("discord-current", Some("discord")))
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
async fn memory_off_clears_nothing() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mut config = Config::default();
    config.workspace_dir = tmp.path().to_path_buf();
    config.memory.engine = String::new();
    assert_eq!(clear_channel_memory(&config, "discord").await.unwrap(), 0);
}
