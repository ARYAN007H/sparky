use super::*;

use tinymemory_api::{Hit, ItemId, ItemKind, MemoryMeta};
use tinymemory_tools::recall::{ContextPack, SectionHits};

#[test]
fn host_pack_citations_follow_references_and_exclude_unshown_hits() {
    let hit = |id: &str| Hit {
        id: ItemId(id.to_string()),
        kind: ItemKind::Learning,
        text: id.to_string(),
        meta: MemoryMeta::default(),
        score: 1.0,
        confidence: None,
    };
    let pack = ContextPack {
        markdown: "shown memory".into(),
        tokens: 2,
        refs: vec![ItemId::from("first"), ItemId::from("second")],
        sections: vec![SectionHits {
            heading: "Learnings".into(),
            answer: None,
            hits: vec![hit("first"), hit("unshown"), hit("second")],
        }],
        skipped: Vec::new(),
        engine: "test".into(),
    };

    let turn_pack = TurnPack::from_pack(pack).expect("nonempty pack");
    assert_eq!(turn_pack.refs, ["first", "second"]);
    assert_eq!(
        turn_pack
            .citations
            .iter()
            .map(|citation| citation.id.as_str())
            .collect::<Vec<_>>(),
        ["first", "second"]
    );
}

#[tokio::test]
async fn background_prefetch_records_citations_only_when_returning_the_pack() {
    let tmp = tempfile::tempdir().unwrap();
    let config = Arc::new(config_in(&tmp));
    let engine = bind_reference(&config);
    engine
        .store(StoreItem::learning(
            "The preferred colour is teal",
            LearningKind::Preference,
            0.9,
            MemoryMeta::default(),
        ))
        .await
        .unwrap();
    let identity = tinymemory_tools::layout::identity::Identity::agent("citation-agent")
        .resolve(&crate::memory::scope::identity_pins(&config));
    let ctx = crate::core::runtime::DispatchCtx::ambient();

    assert!(super::super::prefetch::pre_turn(
        &ctx,
        config.clone(),
        identity.clone(),
        "citation-thread".into(),
        0,
        "what colour is preferred?".into(),
        0,
        Utc::now(),
        false,
        None,
        None,
    )
    .is_none());
    assert!(crate::web_chat::citations::take_turn_citations(&ctx, "citation-thread").is_empty());

    super::super::prefetch::settle_for_test(
        &ctx,
        &config,
        &identity,
        "citation-thread",
        false,
    )
    .await;
    let pack = super::super::prefetch::pre_turn(
        &ctx,
        config,
        identity,
        "citation-thread".into(),
        2,
        "what colour is preferred?".into(),
        0,
        Utc::now(),
        false,
        None,
        None,
    )
    .expect("completed prefetch pack");
    let recorded = crate::web_chat::citations::take_turn_citations(&ctx, "citation-thread");
    assert_eq!(
        recorded.iter().map(|citation| citation.id.as_str()).collect::<Vec<_>>(),
        pack.citations
            .iter()
            .map(|citation| citation.id.as_str())
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn replaying_a_committed_reply_does_not_queue_its_build_twice() {
    let tmp = tempfile::tempdir().unwrap();
    let mut config = config_in(&tmp);
    config.memory.recall.build_beliefs_every = 2;
    bind_reference(&config);
    let identity = tinymemory_tools::layout::identity::Identity::agent("writer")
        .resolve(&crate::memory::scope::identity_pins(&config));
    let committed = reply("replayed-thread", 1, "The reply is committed.");

    run_post_turn(&config, &identity, committed.clone()).await;
    let first = jobs::snapshot(&config).await.pending;
    assert_eq!(first.len(), 1);

    run_post_turn(&config, &identity, committed).await;
    assert_eq!(jobs::snapshot(&config).await.pending, first);
}

#[test]
fn memory_turn_keeps_its_core_context_without_owning_the_core() {
    let tmp = tempfile::tempdir().unwrap();
    let config = Arc::new(config_in(&tmp));
    let identity = tinymemory_tools::layout::identity::Identity::agent("context-agent")
        .resolve(&crate::memory::scope::identity_pins(&config));
    let core = crate::core::runtime::CoreContext::for_test(
        crate::core::runtime::DomainSet::full(),
        Some(config.workspace_dir.clone()),
    );
    let weak = std::sync::Arc::downgrade(&core);
    let turn = MemoryTurn {
        config,
        identity,
        thread_id: "context-thread".into(),
        pack: None,
        in_prompt_from: std::sync::Arc::new(super::super::window::InPromptFrom::new(2, 4)),
        core: Some(weak.clone()),
    };

    assert_eq!(turn.in_prompt().from_turn, 2);
    assert!(std::sync::Arc::ptr_eq(turn.dispatch_ctx().core().unwrap(), &core));
    assert_eq!(std::sync::Arc::strong_count(&core), 1, "MemoryTurn stores only a weak core");
    drop(core);
    assert!(weak.upgrade().is_none(), "the turn does not retain the core lease");
}
