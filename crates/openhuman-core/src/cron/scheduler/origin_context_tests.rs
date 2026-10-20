use super::*;

fn m(role: &str, text: &str) -> (String, String) {
    (role.to_string(), text.to_string())
}

#[test]
fn tail_keeps_the_newest_messages_in_order() {
    let msgs: Vec<_> = (0..15)
        .map(|i| {
            m(
                if i % 2 == 0 { "user" } else { "assistant" },
                &format!("msg{i}"),
            )
        })
        .collect();
    let tail = bounded_tail(&msgs, 10, 1400);
    let lines: Vec<_> = tail.lines().collect();
    assert_eq!(lines.len(), 10);
    assert_eq!(lines[0], "assistant: msg5");
    assert_eq!(lines[9], "user: msg14");
}

#[test]
fn tail_is_bounded_by_characters_and_keeps_the_newest() {
    let long = "x".repeat(1000);
    let msgs = vec![
        m("user", "old one"),
        m("user", &long),
        m("assistant", &long),
    ];
    let tail = bounded_tail(&msgs, 10, 1400);
    assert!(tail.chars().count() <= 1400, "len {}", tail.chars().count());
    assert!(tail.ends_with(&"x".repeat(100)));
    assert!(tail.lines().last().unwrap().starts_with("assistant: "));
    assert!(!tail.contains("old one"));
}

#[test]
fn tail_skips_empty_and_non_chat_roles() {
    let msgs = vec![m("system", "secret"), m("user", "  "), m("user", "hi")];
    assert_eq!(bounded_tail(&msgs, 10, 1400), "user: hi");
    assert_eq!(bounded_tail(&[], 10, 1400), "");
}

#[test]
fn current_prompt_has_preamble_context_and_task() {
    let p = compose_current_prompt("j1", "water", "drink water", "user: hi");
    assert!(p.starts_with("[cron:j1 water] "));
    assert!(p.contains(UNATTENDED_PREAMBLE));
    assert!(p.contains("NO_REPLY"));
    assert!(p.contains("user: hi"));
    assert!(p.ends_with("Scheduled task: drink water"));
    let bare = compose_current_prompt("j1", "water", "drink water", "");
    assert!(!bare.contains("Recent conversation"));
}

#[tokio::test]
async fn channel_origin_context_reads_the_tenant_scoped_durable_session() {
    use tinyagents_session::transcript::{SessionRef, TranscriptMessage, TranscriptMeta};
    use tinyflows_bus::schedule::types::{ChannelConversationIdentity, ChannelSessionIdentity};

    let workspace = tempfile::tempdir().unwrap();
    let config = crate::config::Config {
        workspace_dir: workspace.path().to_path_buf(),
        action_dir: workspace.path().to_path_buf(),
        config_path: workspace.path().join("config.toml"),
        ..Default::default()
    };
    let core = crate::core::runtime::CoreContext::for_test(
        crate::core::runtime::DomainSet::none(),
        Some(workspace.path().to_path_buf()),
    );
    let ctx = DispatchCtx::new(std::sync::Arc::clone(&core));
    let tenant = ctx.tenant().unwrap();
    let agent_id = crate::web_chat::pick_target_agent_id(&config);
    let canonical_session = "channel-session-v22:test";
    let session_key = crate::core::runtime::tenant_key(&tenant, canonical_session);
    let locator = crate::agent::session_store::transcripts_or_files_in(
        &core,
        &crate::agent::session_store::agent_key_for(&tenant, &agent_id),
        &config.workspace_dir,
    );
    let session = SessionRef::scoped(&session_key, &agent_id);
    let history = locator
        .open_session(
            &session,
            TranscriptMeta {
                session_id: None,
                parent_session_id: None,
                agent_name: agent_id.clone(),
                agent_id: Some(agent_id.clone()),
                agent_type: None,
                dispatcher: "native".into(),
                provider: None,
                model: None,
                created: "2026-10-20T00:00:00Z".into(),
                updated: "2026-10-20T00:00:00Z".into(),
                turn_count: 1,
                prefix_message_count: None,
                prefix_reset: false,
                input_tokens: 0,
                output_tokens: 0,
                cached_input_tokens: 0,
                charged_amount_usd: 0.0,
                thread_id: None,
                task_id: None,
            },
        )
        .unwrap();
    history
        .append(TranscriptMessage::user("durable channel question"))
        .unwrap();
    history
        .append(TranscriptMessage::assistant("durable channel answer"))
        .unwrap();

    let origin = JobOrigin::Channel {
        channel: "telegram:bot-a".into(),
        reply_target: "room-1".into(),
        history_key: "untrusted-legacy-history-key".into(),
        sender: Some("sender-1".into()),
        thread_id: None,
        channel_session: Some(Box::new(ChannelSessionIdentity {
            tenant_key: crate::core::runtime::tenant_key(&tenant, ""),
            agent_id,
            session_key: canonical_session.into(),
            provider: "telegram".into(),
            account_id: Some("telegram:bot-a".into()),
            conversation: ChannelConversationIdentity {
                kind: "group".into(),
                id: "room-1".into(),
                scope_id: None,
                parent_id: None,
                thread_id: None,
                topic_id: None,
            },
        })),
    };
    let messages = origin_messages(&ctx, &config, &origin).await;
    assert_eq!(
        messages,
        [
            m("user", "durable channel question"),
            m("assistant", "durable channel answer")
        ]
    );

    let mut forged = origin;
    if let JobOrigin::Channel {
        channel_session: Some(identity),
        ..
    } = &mut forged
    {
        identity.tenant_key = "another-tenant".into();
    }
    assert!(origin_messages(&ctx, &config, &forged).await.is_empty());
}
