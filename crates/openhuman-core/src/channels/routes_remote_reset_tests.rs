use crate::channels::routes::tests::runtime_context;
use crate::core::runtime::{ContextOverlay, CoreContext, DispatchCtx, DomainSet};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tinyagents_session::transcript::{SessionRef, TranscriptMessage, TranscriptMeta};

struct ConfiguredChannel;

#[async_trait::async_trait]
impl crate::channels::Channel for ConfiguredChannel {
    fn name(&self) -> &str {
        "telegram:bot-a"
    }

    async fn send(&self, _message: &crate::channels::SendMessage) -> anyhow::Result<()> {
        Ok(())
    }

    async fn listen(
        &self,
        _tx: tokio::sync::mpsc::Sender<tinychannels_bus::ChannelMessage>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

fn transcript_meta(thread_id: &str, agent_id: &str) -> TranscriptMeta {
    TranscriptMeta {
        agent_name: agent_id.into(),
        agent_id: Some(agent_id.into()),
        agent_type: Some("root".into()),
        dispatcher: "native".into(),
        provider: Some("mock".into()),
        model: Some("session-test".into()),
        created: "2026-10-20T00:00:00Z".into(),
        updated: "2026-10-20T00:00:00Z".into(),
        turn_count: 1,
        prefix_message_count: Some(0),
        prefix_reset: false,
        input_tokens: 0,
        output_tokens: 0,
        cached_input_tokens: 0,
        charged_amount_usd: 0.0,
        thread_id: Some(thread_id.into()),
        task_id: None,
        session_id: None,
        parent_session_id: None,
    }
}

fn remote_session_scope() -> tinychannels_bus::module_remote::RemoteSessionScope {
    tinychannels_bus::module_remote::RemoteSessionScope {
        channel: tinychannels_bus::ChannelRef {
            id: "telegram".into(),
            account_id: Some("telegram:bot-a".into()),
        },
        conversation: tinychannels_bus::ConversationRef {
            kind: tinychannels_bus::ConversationKind::Group,
            id: "room-1".into(),
            topic_id: Some("topic-3".into()),
            ..Default::default()
        },
        session_key: "prepared-session".into(),
    }
}

fn seed_session(core: &CoreContext, workspace: &Path, key: &str, agent_id: &str) -> SessionRef {
    let session = SessionRef::scoped(key, agent_id);
    let history =
        crate::agent::session_store::transcripts_or_files_in(core, agent_id, workspace)
            .open_session(&session, transcript_meta(key, agent_id))
            .unwrap();
    history
        .append(TranscriptMessage::user("prior tenant turn"))
        .unwrap();
    history
        .append(TranscriptMessage::assistant("prior tenant reply"))
        .unwrap();
    session
}

fn session_messages(core: &CoreContext, workspace: &Path, session: &SessionRef) -> Vec<String> {
    let locator = crate::agent::session_store::transcripts_or_files_in(
        core,
        &session.agent_id,
        workspace,
    );
    let head = locator.head_generation(session);
    locator
        .read_session_transcript(&head)
        .expect("durable transcript exists")
        .read_session()
        .unwrap()
        .expect("transcript is readable")
        .messages
        .into_iter()
        .map(|message| message.content)
        .collect()
}

fn tenant_runtime(
    profile: &str,
    workspace: &Path,
) -> (
    Arc<CoreContext>,
    crate::channels::context::ChannelRuntimeContext,
) {
    let workspace = workspace.to_path_buf();
    let mut config = crate::config::Config {
        workspace_dir: workspace.clone(),
        action_dir: workspace.clone(),
        config_path: workspace.join("config.toml"),
        ..Default::default()
    };
    config.cloud_providers = vec![crate::config::schema::CloudProviderCreds {
        id: "p_openai_reset_test".into(),
        slug: "openai".into(),
        label: "Reset test provider".into(),
        endpoint: "http://127.0.0.1:9/v1".into(),
        default_model: Some("reset-test-model".into()),
        ..Default::default()
    }];
    let core = CoreContext::for_test(DomainSet::none(), Some(workspace.clone())).derive_with(
        ContextOverlay::new(config.clone(), DomainSet::full(), Default::default()).profile(profile),
    );
    let mut runtime = runtime_context(workspace);
    runtime.config = Some(Arc::new(config));
    runtime.core = Some(Arc::downgrade(&core));
    runtime.channels_by_name = Arc::new(HashMap::from([(
        "telegram:bot-a".into(),
        Arc::new(ConfiguredChannel) as Arc<dyn crate::channels::Channel>,
    )]));
    (core, runtime)
}

#[cfg(feature = "modules")]
#[tokio::test]
async fn remote_clear_resets_only_the_captured_tenant_session() {
    use tinychannels_bus::module_remote::{RemoteHostRequest, RemoteHostResult};

    let tmp = tempfile::TempDir::new().unwrap();
    let workspace = tmp.path();
    let (alice_core, alice_runtime) = tenant_runtime("remote-reset-alice", workspace);
    let (bob_core, _bob_runtime) = tenant_runtime("remote-reset-bob", workspace);
    let alice_dispatch = DispatchCtx::new(Arc::clone(&alice_core));
    let bob_dispatch = DispatchCtx::new(Arc::clone(&bob_core));
    let alice_tenant = alice_dispatch.tenant().unwrap();
    let bob_tenant = bob_dispatch.tenant().unwrap();
    let alice_route = crate::channels::routes::get_route_selection(&alice_runtime, "remote-reset");
    let bob_route = crate::channels::routes::get_route_selection(&_bob_runtime, "remote-reset");
    let alice_agent = crate::channels::runtime::dispatch::host_agent::configured_agent(
        &alice_runtime,
        &alice_route,
    )
    .await
    .expect("Alice's configured channel agent resolves");
    let bob_agent = crate::channels::runtime::dispatch::host_agent::configured_agent(
        &_bob_runtime,
        &bob_route,
    )
    .await
    .expect("Bob's configured channel agent resolves");
    let canonical = "channel-session:telegram:bot-a:group:room-1";
    let alice_key = crate::core::runtime::tenant::tenant_key(&alice_tenant, canonical);
    let bob_key = crate::core::runtime::tenant::tenant_key(&bob_tenant, canonical);
    assert_ne!(alice_key, bob_key);
    let alice_session = seed_session(&alice_core, workspace, &alice_key, &alice_agent.definition.id);
    let bob_session = seed_session(&bob_core, workspace, &bob_key, &bob_agent.definition.id);
    assert_eq!(
        super::durable_channel_history_len(
            &alice_runtime,
            &alice_route,
            "telegram",
            Some(&alice_key),
        )
        .await,
        2,
        "remote status history comes from the durable session",
    );

    let scope = remote_session_scope();
    let reset = crate::channels::host::remote_control::RemoteResetContext {
        scope: scope.clone(),
        durable_session_key: alice_key.clone(),
        message: tinychannels_bus::ChannelMessage {
            channel: "telegram".into(),
            reply_target: "room-1".into(),
            content: "/new".into(),
            ..Default::default()
        },
        history_key: "legacy-key-is-not-authority".into(),
    };
    let cancellation = tokio_util::sync::CancellationToken::new();

    let mut forged = scope.clone();
    forged.conversation.id = "other-room".into();
    assert!(crate::channels::host::remote_control::dispatch_callback(
        &alice_dispatch,
        &alice_runtime,
        &alice_tenant,
        &cancellation,
        Some(&reset),
        RemoteHostRequest::ClearHistory {
            session_scope: forged,
        },
    )
    .await
    .is_err());
    assert_eq!(
        session_messages(&alice_core, workspace, &alice_session).len(),
        2
    );
    assert_eq!(
        session_messages(&bob_core, workspace, &bob_session).len(),
        2
    );

    let cleared = crate::channels::host::remote_control::dispatch_callback(
        &alice_dispatch,
        &alice_runtime,
        &alice_tenant,
        &cancellation,
        Some(&reset),
        RemoteHostRequest::ClearHistory {
            session_scope: scope,
        },
    )
    .await
    .expect("captured tenant reset succeeds");
    assert!(matches!(cleared, RemoteHostResult::Completed));
    assert!(session_messages(&alice_core, workspace, &alice_session).is_empty());
    assert_eq!(
        super::durable_channel_history_len(
            &alice_runtime,
            &alice_route,
            "telegram",
            Some(&alice_key),
        )
        .await,
        0,
        "a reset clears the durable status count",
    );
    assert_eq!(
        session_messages(&bob_core, workspace, &bob_session),
        ["prior tenant turn", "prior tenant reply"]
    );
}
