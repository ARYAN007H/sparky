//! The memory tool's reads: recall returns the engine's evidence, and recall
//! and fetch leave the calling thread out.

use super::*;
use crate::memory::scope::MemoryIdentity;
use crate::memory::test_fixtures::{bind_reference, config_in};
use tinymemory_api::MemoryEngine;

fn facts() -> CallFacts {
    CallFacts {
        thread_id: Some("thread-tool-1".into()),
        agent_id: Some("orchestrator".into()),
        ..CallFacts::of(&Config::default(), &MemoryIdentity::agent("orchestrator"))
    }
}

fn text(result: &ToolResult) -> String {
    result.text()
}

/// A one-turn conversation in `thread`, where `facts` reads and writes.
fn turn_in(facts: &CallFacts, thread: &str, text: &str) -> tinymemory_api::StoreItem {
    tinymemory_api::StoreItem::Conversation {
        turns: vec![tinymemory_api::Turn::new(tinymemory_api::Role::User, text)],
        meta: tinymemory_api::MemoryMeta {
            namespace: facts.namespace.clone(),
            thread_id: Some(thread.to_string()),
            turns: Some(tinymemory_api::TurnRange { first: 0, last: 0 }),
            ..tinymemory_api::MemoryMeta::default()
        },
    }
}

#[tokio::test]
async fn recall_returns_evidence_and_leaves_the_calling_thread_out() {
    let tmp = tempfile::tempdir().unwrap();
    let config = config_in(&tmp);
    let engine = bind_reference(&config);
    let facts = facts();
    let live = facts.thread_id.clone().unwrap();
    engine
        .store(turn_in(&facts, &live, "which oolong tea did I like"))
        .await
        .unwrap();
    engine
        .store(turn_in(
            &facts,
            "thread-past",
            "I liked the oolong tea from Taiwan",
        ))
        .await
        .unwrap();

    let recalled = run_action(
        &DispatchCtx::ambient(),
        &config,
        &json!({"action": "recall", "question": "oolong tea"}),
        &facts,
    )
    .await;
    assert!(!recalled.is_error, "{}", text(&recalled));
    let out: Value = serde_json::from_str(&text(&recalled)).unwrap();
    let evidence = out["evidence"].as_str().unwrap();
    assert!(evidence.contains("oolong tea from Taiwan"), "{out}");
    assert!(
        !evidence.contains("which oolong tea did I like"),
        "the turn being answered must not come back as a memory: {out}"
    );
    assert_eq!(out["items"], 1);
    assert!(
        out.get("answer").is_none(),
        "recall writes no answer of its own"
    );
    let cites = take_turn_citations(&DispatchCtx::ambient(), &live);
    assert_eq!(cites.len(), 1, "{cites:?}");
}

#[tokio::test]
async fn fetch_leaves_the_calling_thread_out() {
    let tmp = tempfile::tempdir().unwrap();
    let config = config_in(&tmp);
    let engine = bind_reference(&config);
    let facts = facts();
    let live = facts.thread_id.clone().unwrap();
    engine
        .store(turn_in(&facts, &live, "oolong question in this thread"))
        .await
        .unwrap();
    engine
        .store(turn_in(&facts, "thread-past", "oolong answer from before"))
        .await
        .unwrap();

    let fetched = run_action(
        &DispatchCtx::ambient(),
        &config,
        &json!({"action": "fetch", "query": "oolong", "mode": "keyword", "limit": 5}),
        &facts,
    )
    .await;
    assert!(!fetched.is_error, "{}", text(&fetched));
    let hits = serde_json::from_str::<Value>(&text(&fetched)).unwrap()["hits"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0]["meta"]["thread_id"], "thread-past");
}

#[tokio::test]
async fn recall_with_nothing_relevant_says_so() {
    let tmp = tempfile::tempdir().unwrap();
    let config = config_in(&tmp);
    bind_reference(&config);
    let recalled = run_action(
        &DispatchCtx::ambient(),
        &config,
        &json!({"action": "recall", "question": "anything at all"}),
        &facts(),
    )
    .await;
    assert!(!recalled.is_error, "{}", text(&recalled));
    let out: Value = serde_json::from_str(&text(&recalled)).unwrap();
    assert_eq!(out["evidence"], "");
    assert_eq!(out["note"], "Nothing relevant is stored in memory.");
}

#[tokio::test]
async fn the_rpc_recall_still_answers_and_excludes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let config = config_in(&tmp);
    let engine = bind_reference(&config);
    let facts = facts();
    engine
        .store(turn_in(
            &facts,
            "thread-tool-1",
            "oolong in the live thread",
        ))
        .await
        .unwrap();
    let view = ops::recall(
        &config,
        RecallParams {
            question: "oolong".into(),
            filter: None,
            limit: None,
            refers_to: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        view.citations.len(),
        1,
        "no thread is excluded outside the tool"
    );
    assert!(
        view.answer.starts_with("From 1 stored items"),
        "{}",
        view.answer
    );
    assert!(
        view.model.is_some(),
        "answer mode reports the answering engine"
    );
}
