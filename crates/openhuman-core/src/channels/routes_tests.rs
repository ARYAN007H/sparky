use super::*;
use crate::channels::context::{
    conversation_history_key, ChannelRuntimeContext, RouteSelectionMap, TurnModelSourceCacheMap,
};
use crate::channels::host::ChannelTurnStateSubscriber;
use crate::core::events::DomainEvent;
use async_trait::async_trait;
use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use tinybus::EventHandler;
use tinychannels::routes::resolve_provider_alias;
use tinychannels_bus::ChannelMessage;
use tinychannels_runtime::context::ChatMessage;
use tinytools::{Tool, ToolResult};

struct DummyTool;

#[async_trait]
impl Tool for DummyTool {
    fn name(&self) -> &str {
        "dummy"
    }

    fn description(&self) -> &str {
        "dummy"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({})
    }

    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult::success("ok"))
    }
}

#[derive(Default)]
struct RecordingChannel {
    sent: Mutex<Vec<SendMessage>>,
}

#[async_trait]
impl Channel for RecordingChannel {
    fn name(&self) -> &str {
        "recording"
    }

    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        self.sent.lock().unwrap().push(message.clone());
        Ok(())
    }

    async fn listen(&self, _tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        Ok(())
    }
}

fn runtime_context(workspace_dir: PathBuf) -> ChannelRuntimeContext {
    let model: Arc<dyn tinyinference_llm::model::ChatModel<()>> =
        Arc::new(tinyagents_harness::testkit::ScriptedModel::replies(vec![
            "ok",
        ]));
    ChannelRuntimeContext {
        relay_thread: None,
        channels_by_name: Arc::new(HashMap::new()),
        module_objects: Arc::new(HashMap::new()),
        turn_model_source: Some(crate::agent::tinyagents::TurnModelSource::from_model(model)),
        default_provider: Arc::new("openai".into()),
        tools_registry: Arc::new(vec![Box::new(DummyTool) as Box<dyn Tool>]),
        system_prompt: crate::channels::ChannelSystemPrompt::fixed("prompt"),
        model: Arc::new("reasoning-v1".into()),
        temperature: 0.0,
        max_tool_iterations: 1,
        conversation_histories: Arc::new(Mutex::new(HashMap::new())),
        turn_model_source_cache: TurnModelSourceCacheMap::default(),
        route_overrides: RouteSelectionMap::default(),
        api_url: None,
        inference_url: None,
        reliability: Arc::new(crate::config::ReliabilityConfig::default()),
        provider_runtime_options: crate::inference::provider::ProviderRuntimeOptions::default(),
        workspace_dir: Arc::new(workspace_dir),
        message_timeout_secs: 60,
        multimodal: crate::config::MultimodalConfig::default(),
        multimodal_files: crate::config::MultimodalFileConfig::default(),
        config: None,
        core: None,
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

#[cfg(feature = "modules")]
async fn module_route_help_response(
    workspace_dir: &Path,
    content: &str,
    providers: Vec<tinychannels_bus::module_control::ProviderDescriptor>,
) -> String {
    use tinychannels_bus::module_control::{ChannelControlRequest, ChannelControlResult};
    use tinychannels_bus::module_remote::RemoteRoute;

    let request = ChannelControlRequest::Route {
        workspace_dir: workspace_dir.to_string_lossy().into_owned(),
        message: Box::new(ChannelMessage {
            reply_to_id: None,
            topic_id: None,
            id: "help-fixture".into(),
            sender: "operator".into(),
            reply_target: "chat".into(),
            content: content.into(),
            channel: "telegram".into(),
            timestamp: 0,
            thread_ts: None,
            sender_name: None,
        }),
        route: RemoteRoute {
            provider: "openai".into(),
            model: "gpt-5".into(),
        },
        history_len: 0,
        session_scope: None,
        providers,
    };
    let response: ChannelControlResult = crate::channels::module_fixture::client()
        .await
        .call_confidential(
            "tinychannels",
            tinychannels_bus::methods::PREPARE_CHANNEL_CONTROL,
            (request,),
        )
        .await
        .expect("the owning module should prepare command help");
    match response {
        ChannelControlResult::Route {
            reply: Some(reply), ..
        } => reply,
        other => panic!("expected module-generated help text, got {other:?}"),
    }
}

#[test]
fn provider_alias_and_route_selection_round_trip() {
    let first_provider = provider::list_providers()
        .into_iter()
        .next()
        .expect("provider registry should not be empty");
    assert_eq!(
        resolve_provider_alias(first_provider.name, &provider_descriptors()).as_deref(),
        Some(first_provider.name)
    );
    assert!(resolve_provider_alias("   ", &provider_descriptors()).is_none());

    let ctx = runtime_context(PathBuf::from("/tmp"));
    let sender_key = "telegram_alice_reply";
    assert_eq!(
        get_route_selection(&ctx, sender_key),
        ChannelRouteSelection {
            provider: "openai".into(),
            model: "reasoning-v1".into()
        }
    );

    set_route_selection(
        &ctx,
        sender_key,
        ChannelRouteSelection {
            provider: "anthropic".into(),
            model: "claude".into(),
        },
    );
    assert_eq!(
        get_route_selection(&ctx, sender_key),
        ChannelRouteSelection {
            provider: "anthropic".into(),
            model: "claude".into()
        }
    );

    set_route_selection(&ctx, sender_key, default_route_selection(&ctx));
    assert!(ctx.route_overrides.lock().unwrap().is_empty());
}

#[cfg(feature = "modules")]
#[tokio::test]
async fn cached_models_and_help_responses_render_expected_text() {
    let tempdir = tempfile::tempdir().unwrap();
    let state_dir = tempdir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    std::fs::write(
        state_dir.join("models_cache.json"),
        serde_json::json!({
            "entries": [
                {
                    "provider": "openai",
                    "models": ["gpt-5", "gpt-5-mini", "gpt-4.1"]
                }
            ]
        })
        .to_string(),
    )
    .unwrap();

    let models = module_route_help_response(tempdir.path(), "/model", Vec::new()).await;
    assert!(models.contains("Current provider: `openai`"));
    assert!(models.contains("Cached model IDs"));
    assert!(models.contains("- `gpt-5-mini`"));

    let providers = module_route_help_response(
        tempdir.path(),
        "/models",
        vec![
            tinychannels_bus::module_control::ProviderDescriptor {
                name: "openai".into(),
                aliases: vec!["oa".into()],
            },
            tinychannels_bus::module_control::ProviderDescriptor {
                name: "anthropic".into(),
                aliases: Vec::new(),
            },
        ],
    )
    .await;
    assert!(providers.contains("Switch provider with `/models <provider>`"));
    assert!(providers.contains("Available providers:"));
    assert!(providers.contains("openai (aliases: oa)"));
}

#[test]
fn model_command_messages_use_thread_aware_history_keys() {
    let msg = ChannelMessage {
        reply_to_id: None,
        topic_id: None,
        id: "1".into(),
        sender: "alice".into(),
        reply_target: "room".into(),
        content: "/model gpt-5".into(),
        channel: "discord".into(),
        timestamp: 0,
        thread_ts: Some("thread-1".into()),
        sender_name: None,
    };
    assert_eq!(
        super::super::context::conversation_history_key(&msg),
        "discord_alice_room_thread:thread-1"
    );
}

#[cfg(feature = "modules")]
#[tokio::test]
async fn sign_out_cancels_remote_invalidation_while_it_waits_for_the_session_table() {
    let core_context =
        crate::core::runtime::CoreContext::for_test(crate::core::runtime::DomainSet::full(), None);
    let expected_core_context = Arc::clone(&core_context);
    crate::core::runtime::CoreContext::scope(core_context, async {
        let guard = crate::web_chat::lock_thread_sessions_for_test(
            &crate::core::runtime::DispatchCtx::ambient(),
        )
        .await;
        let cancellation = tokio_util::sync::CancellationToken::new();
        let callback = RouteCallbacks::new(
            runtime_context(PathBuf::from("/tmp/channel-callback")),
            cancellation.clone(),
            None,
        );
        assert!(Arc::ptr_eq(
            callback.core_context.as_ref().unwrap(),
            &expected_core_context
        ));
        assert_eq!(callback.tenant, crate::core::runtime::Tenant::default());

        let request = tinychannels_bus::ChannelBackendRequest::RemoteControl(
            tinychannels_bus::module_remote::RemoteHostRequest::InvalidateThread {
                thread_id: "sign-out-in-flight".into(),
            },
        );
        let mut operation = Box::pin(callback.dispatch_controller_backend(request));
        let waker = Waker::noop().clone();
        let mut task_context = Context::from_waker(&waker);

        assert!(
            matches!(operation.as_mut().poll(&mut task_context), Poll::Pending),
            "the callback should be waiting on the locked session table"
        );
        cancellation.cancel();
        assert!(
            matches!(
                operation.as_mut().poll(&mut task_context),
                Poll::Ready(Err(_))
            ),
            "sign-out must cancel the callback before it can mutate session state"
        );

        drop(guard);
    })
    .await;
}

#[cfg(feature = "modules")]
#[tokio::test]
async fn load_cached_model_preview_returns_empty_when_cache_json_is_invalid() {
    let tempdir = tempfile::tempdir().unwrap();
    let state_dir = tempdir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    std::fs::write(
        state_dir.join("models_cache.json"),
        "{ definitely invalid json",
    )
    .unwrap();

    let response = module_route_help_response(tempdir.path(), "/model", Vec::new()).await;
    assert!(response.contains("No cached model list found for `openai`"));
}

#[tokio::test]
async fn handle_runtime_command_unknown_provider_sends_helpful_error() {
    let ctx = runtime_context(PathBuf::from("/tmp"));
    let channel_impl = Arc::new(RecordingChannel::default());
    let channel: Arc<dyn Channel> = channel_impl.clone();
    let msg = ChannelMessage {
        reply_to_id: None,
        topic_id: None,
        id: "1".into(),
        sender: "alice".into(),
        reply_target: "room".into(),
        content: "/models definitely-not-a-provider".into(),
        channel: "telegram".into(),
        timestamp: 0,
        thread_ts: Some("thread-1".into()),
        sender_name: None,
    };

    let handled = handle_runtime_command_if_needed(
        &ctx,
        &msg,
        Some(&channel),
        &conversation_history_key(&msg),
    )
    .await;
    assert!(handled);

    let sent = channel_impl.sent.lock().unwrap();
    assert_eq!(sent.len(), 1);
    assert!(sent[0]
        .content
        .contains("Unknown provider `definitely-not-a-provider`"));
    assert_eq!(sent[0].thread_ts.as_deref(), Some("thread-1"));
}

#[tokio::test]
async fn clear_history_command_uses_reset_callback_and_persists_route_override() {
    let ctx = runtime_context(PathBuf::from("/tmp"));
    let key = "telegram_alice_room";
    ctx.conversation_histories
        .lock()
        .unwrap()
        .insert(key.to_string(), vec![ChatMessage::user("old history")]);
    let channel_impl = Arc::new(RecordingChannel::default());
    let channel: Arc<dyn Channel> = channel_impl.clone();
    let msg = ChannelMessage {
        reply_to_id: None,
        topic_id: None,
        id: "1".into(),
        sender: "alice".into(),
        reply_target: "room".into(),
        content: "/model gpt-5-mini".into(),
        channel: "telegram".into(),
        timestamp: 0,
        thread_ts: None,
        sender_name: None,
    };

    let reset_called = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let callback_flag = Arc::clone(&reset_called);
    let handled = handle_runtime_command_with_reset(
        &ctx,
        &msg,
        Some(&channel),
        &conversation_history_key(&msg),
        None,
        None,
        &msg,
        move || async move {
            callback_flag.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        },
    )
    .await;
    assert!(handled);
    assert!(reset_called.load(std::sync::atomic::Ordering::SeqCst));

    assert!(ctx.conversation_histories.lock().unwrap().contains_key(key));
    assert_eq!(
        get_route_selection(&ctx, key),
        ChannelRouteSelection {
            provider: "openai".into(),
            model: "gpt-5-mini".into()
        }
    );

    let sent = channel_impl.sent.lock().unwrap();
    assert_eq!(sent.len(), 1);
    assert!(sent[0].content.contains("Model switched to `gpt-5-mini`"));
}

#[cfg(feature = "modules")]
#[tokio::test]
async fn remote_clear_without_captured_scope_fails_closed() {
    let ctx = runtime_context(std::env::temp_dir().join("remote-clear-scope"));
    let dispatch = ctx.dispatch_ctx();
    let cancellation = tokio_util::sync::CancellationToken::new();

    let result = crate::channels::host::remote_control::dispatch_callback(
        &dispatch,
        &ctx,
        &crate::core::runtime::Tenant::default(),
        &cancellation,
        None,
        tinychannels_bus::module_remote::RemoteHostRequest::ClearHistory {
            session_scope: remote_session_scope(),
        },
    )
    .await;

    assert!(
        result.is_err(),
        "an unscoped reset must never report success"
    );
}

#[cfg(feature = "modules")]
#[tokio::test]
async fn remote_clear_rejects_an_unconfigured_captured_account() {
    let ctx = runtime_context(std::env::temp_dir().join("remote-clear-account"));
    let dispatch = ctx.dispatch_ctx();
    let cancellation = tokio_util::sync::CancellationToken::new();
    let scope = remote_session_scope();
    let reset_context = crate::channels::host::remote_control::RemoteResetContext {
        scope: scope.clone(),
        durable_session_key: "tenant:session:telegram:bot-a:group:room-1".into(),
        message: ChannelMessage {
            channel: "telegram:bot-a".into(),
            reply_target: "room-1".into(),
            content: "/new".into(),
            ..Default::default()
        },
        history_key: "legacy-history-key-is-not-used-as-identity".into(),
    };

    let result = crate::channels::host::remote_control::dispatch_callback(
        &dispatch,
        &ctx,
        &crate::core::runtime::Tenant::default(),
        &cancellation,
        Some(&reset_context),
        tinychannels_bus::module_remote::RemoteHostRequest::ClearHistory {
            session_scope: scope,
        },
    )
    .await;

    assert!(result.is_err(), "unknown provider account must fail closed");
}

#[tokio::test]
async fn remote_clear_resets_only_the_captured_tenant_session() {
    use crate::core::runtime::{ContextOverlay, CoreContext, DispatchCtx, DomainSet};
    use tinyagents_session::transcript::{
        SessionRef, TranscriptHistory, TranscriptLocator, TranscriptMessage, TranscriptMeta,
        TranscriptRead,
    };

    struct ConfiguredChannel;
    #[async_trait]
    impl Channel for ConfiguredChannel {
        fn name(&self) -> &str {
            "telegram:bot-a"
        }
        async fn send(&self, _message: &SendMessage) -> anyhow::Result<()> {
            Ok(())
        }
        async fn listen(
            &self,
            _tx: tokio::sync::mpsc::Sender<ChannelMessage>,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    fn transcript_meta(thread_id: &str) -> TranscriptMeta {
        TranscriptMeta {
            agent_name: "orchestrator".into(),
            agent_id: Some("orchestrator".into()),
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

    fn seed_session(core: &CoreContext, workspace: &Path, key: &str) -> SessionRef {
        let session = SessionRef::scoped(key, "orchestrator");
        let history =
            crate::agent::session_store::transcripts_or_files_in(core, "orchestrator", workspace)
                .open_session(&session, transcript_meta(key))
                .unwrap();
        history
            .append(TranscriptMessage::user("prior tenant turn"))
            .unwrap();
        history
            .append(TranscriptMessage::assistant("prior tenant reply"))
            .unwrap();
        session
    }

    fn messages(core: &CoreContext, workspace: &Path, session: &SessionRef) -> Vec<String> {
        let locator =
            crate::agent::session_store::transcripts_or_files_in(core, "orchestrator", workspace);
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

    let tmp = tempfile::TempDir::new().unwrap();
    let workspace = tmp.path().to_path_buf();
    let make_context = |profile: &str| {
        let config = crate::config::Config {
            workspace_dir: workspace.clone(),
            action_dir: workspace.clone(),
            config_path: workspace.join("config.toml"),
            ..Default::default()
        };
        let core = CoreContext::for_test(DomainSet::none(), Some(workspace.clone())).derive_with(
            ContextOverlay::new(config.clone(), DomainSet::full(), Default::default())
                .profile(profile),
        );
        let mut runtime = runtime_context(workspace.clone());
        runtime.config = Some(Arc::new(config));
        runtime.core = Some(Arc::downgrade(&core));
        runtime.channels_by_name = Arc::new(HashMap::from([(
            "telegram:bot-a".into(),
            Arc::new(ConfiguredChannel) as Arc<dyn Channel>,
        )]));
        (core, runtime)
    };

    let (alice_core, alice_runtime) = make_context("remote-reset-alice");
    let (bob_core, bob_runtime) = make_context("remote-reset-bob");
    let alice_dispatch = DispatchCtx::new(Arc::clone(&alice_core));
    let bob_dispatch = DispatchCtx::new(Arc::clone(&bob_core));
    let alice_tenant = alice_dispatch.tenant().unwrap();
    let bob_tenant = bob_dispatch.tenant().unwrap();
    let canonical = "channel-session:telegram:bot-a:group:room-1";
    let alice_key = crate::core::runtime::tenant::tenant_key(&alice_tenant, canonical);
    let bob_key = crate::core::runtime::tenant::tenant_key(&bob_tenant, canonical);
    assert_ne!(alice_key, bob_key);
    let alice_session = seed_session(&alice_core, &workspace, &alice_key);
    let bob_session = seed_session(&bob_core, &workspace, &bob_key);

    let scope = remote_session_scope();
    let reset_context = crate::channels::host::remote_control::RemoteResetContext {
        scope: scope.clone(),
        durable_session_key: alice_key,
        message: ChannelMessage {
            channel: "telegram".into(),
            reply_target: "room-1".into(),
            content: "/new".into(),
            ..Default::default()
        },
        history_key: "legacy-key-is-not-authority".into(),
    };
    let cancellation = tokio_util::sync::CancellationToken::new();

    let mut forged_scope = scope.clone();
    forged_scope.conversation.id = "other-room".into();
    let forged = crate::channels::host::remote_control::dispatch_callback(
        &alice_dispatch,
        &alice_runtime,
        &alice_tenant,
        &cancellation,
        Some(&reset_context),
        tinychannels_bus::module_remote::RemoteHostRequest::ClearHistory {
            session_scope: forged_scope,
        },
    )
    .await;
    assert!(forged.is_err(), "forged callback scope must be rejected");
    assert_eq!(messages(&alice_core, &workspace, &alice_session).len(), 2);
    assert_eq!(messages(&bob_core, &workspace, &bob_session).len(), 2);

    let cleared = crate::channels::host::remote_control::dispatch_callback(
        &alice_dispatch,
        &alice_runtime,
        &alice_tenant,
        &cancellation,
        Some(&reset_context),
        tinychannels_bus::module_remote::RemoteHostRequest::ClearHistory {
            session_scope: scope,
        },
    )
    .await
    .expect("captured tenant reset succeeds");
    assert_eq!(
        cleared,
        tinychannels_bus::module_remote::RemoteHostResult::Completed
    );
    assert!(messages(&alice_core, &workspace, &alice_session).is_empty());
    assert_eq!(
        messages(&bob_core, &workspace, &bob_session),
        ["prior tenant turn", "prior tenant reply"]
    );
}

#[tokio::test]
async fn handle_runtime_command_telegram_status_replies_without_agent() {
    let ctx = runtime_context(PathBuf::from("/tmp"));
    let channel_impl = Arc::new(RecordingChannel::default());
    let channel: Arc<dyn Channel> = channel_impl.clone();
    let msg = ChannelMessage {
        reply_to_id: None,
        topic_id: None,
        id: "1".into(),
        sender: "alice".into(),
        reply_target: "chat-remote".into(),
        content: "/status".into(),
        channel: "telegram".into(),
        timestamp: 0,
        thread_ts: Some("42".into()),
        sender_name: None,
    };

    let handled = handle_runtime_command_if_needed(
        &ctx,
        &msg,
        Some(&channel),
        &conversation_history_key(&msg),
    )
    .await;
    assert!(handled);

    let sent = channel_impl.sent.lock().unwrap();
    assert_eq!(sent.len(), 1);
    assert!(
        sent[0].content.contains("**Status**"),
        "status command returned unexpected reply: {:?}",
        sent[0].content
    );
    assert!(sent[0].content.contains("Provider:"));
}

#[tokio::test]
async fn handle_runtime_command_without_target_channel_still_consumes_command() {
    let ctx = runtime_context(PathBuf::from("/tmp"));
    let msg = ChannelMessage {
        reply_to_id: None,
        topic_id: None,
        id: "1".into(),
        sender: "alice".into(),
        reply_target: "chat-remote".into(),
        content: "/help".into(),
        channel: "telegram".into(),
        timestamp: 0,
        thread_ts: None,
        sender_name: None,
    };

    let handled =
        handle_runtime_command_if_needed(&ctx, &msg, None, &conversation_history_key(&msg)).await;
    assert!(handled);
}

#[tokio::test]
async fn handle_runtime_command_telegram_help_replies_with_remote_command_list() {
    let ctx = runtime_context(PathBuf::from("/tmp"));
    let channel_impl = Arc::new(RecordingChannel::default());
    let channel: Arc<dyn Channel> = channel_impl.clone();
    let msg = ChannelMessage {
        reply_to_id: None,
        topic_id: None,
        id: "1".into(),
        sender: "alice".into(),
        reply_target: "chat-remote".into(),
        content: "/help".into(),
        channel: "telegram".into(),
        timestamp: 0,
        thread_ts: Some("42".into()),
        sender_name: None,
    };

    let handled = handle_runtime_command_if_needed(
        &ctx,
        &msg,
        Some(&channel),
        &conversation_history_key(&msg),
    )
    .await;
    assert!(handled);

    let sent = channel_impl.sent.lock().unwrap();
    assert_eq!(sent.len(), 1);
    assert!(sent[0].content.contains("Remote control:"));
    assert!(sent[0].content.contains("`/status`"));
    assert!(sent[0].content.contains("`/sessions`"));
    assert!(sent[0].content.contains("`/new`"));
    assert!(sent[0]
        .content
        .contains("Model routing: `/model`, `/models`"));
}

#[tokio::test]
async fn handle_runtime_command_telegram_sessions_reports_empty_store() {
    let tempdir = tempfile::tempdir().unwrap();
    let ctx = runtime_context(tempdir.path().to_path_buf());
    let channel_impl = Arc::new(RecordingChannel::default());
    let channel: Arc<dyn Channel> = channel_impl.clone();
    let msg = ChannelMessage {
        reply_to_id: None,
        topic_id: None,
        id: "1".into(),
        sender: "alice".into(),
        reply_target: "chat-remote".into(),
        content: "/sessions".into(),
        channel: "telegram".into(),
        timestamp: 0,
        thread_ts: Some("42".into()),
        sender_name: None,
    };

    let handled = handle_runtime_command_if_needed(
        &ctx,
        &msg,
        Some(&channel),
        &conversation_history_key(&msg),
    )
    .await;
    assert!(handled);

    let sent = channel_impl.sent.lock().unwrap();
    assert_eq!(sent.len(), 1);
    assert!(sent[0]
        .content
        .contains("No conversation threads yet. Send `/new` to create one."));
    assert_eq!(sent[0].thread_ts.as_deref(), Some("42"));
}

#[tokio::test]
#[ignore = "the linked tinychannels-module is given no storage backend here, so /new fails to persist the chat binding (host storage is unavailable); needs a storage fixture"]
async fn handle_runtime_command_telegram_new_status_and_sessions_round_trip() {
    let tempdir = tempfile::tempdir().unwrap();
    let ctx = runtime_context(tempdir.path().to_path_buf());
    let channel_impl = Arc::new(RecordingChannel::default());
    let channel: Arc<dyn Channel> = channel_impl.clone();
    let sender_key = "telegram_alice_chat-remote";

    ctx.conversation_histories.lock().unwrap().insert(
        sender_key.to_string(),
        vec![ChatMessage::user("old history")],
    );

    let new_msg = ChannelMessage {
        reply_to_id: None,
        topic_id: None,
        id: "1".into(),
        sender: "alice".into(),
        reply_target: "chat-remote".into(),
        content: "/new".into(),
        channel: "telegram".into(),
        timestamp: 0,
        thread_ts: Some("42".into()),
        sender_name: None,
    };
    assert!(
        handle_runtime_command_if_needed(
            &ctx,
            &new_msg,
            Some(&channel),
            &conversation_history_key(&new_msg)
        )
        .await
    );
    assert!(ctx
        .conversation_histories
        .lock()
        .unwrap()
        .get(sender_key)
        .is_none());

    ctx.conversation_histories
        .lock()
        .unwrap()
        .insert(sender_key.to_string(), vec![ChatMessage::user("after new")]);
    set_route_selection(
        &ctx,
        sender_key,
        ChannelRouteSelection {
            provider: "anthropic".into(),
            model: "claude-3".into(),
        },
    );

    let subscriber = ChannelTurnStateSubscriber::new(tempdir.path().to_path_buf());
    subscriber
        .handle(&DomainEvent::ChannelMessageReceived {
            channel: "telegram".into(),
            message_id: "2".into(),
            sender: "alice".into(),
            reply_target: "chat-remote".into(),
            content: "work".into(),
            thread_ts: Some("42".into()),
            inbound_envelope: None,
            workspace_dir: tempdir.path().to_path_buf(),
        })
        .await;

    let status_msg = ChannelMessage {
        reply_to_id: None,
        topic_id: None,
        id: "3".into(),
        sender: "alice".into(),
        reply_target: "chat-remote".into(),
        content: "/status".into(),
        channel: "telegram".into(),
        timestamp: 0,
        thread_ts: Some("42".into()),
        sender_name: None,
    };
    assert!(
        handle_runtime_command_if_needed(
            &ctx,
            &status_msg,
            Some(&channel),
            &conversation_history_key(&status_msg)
        )
        .await
    );

    let sessions_msg = ChannelMessage {
        reply_to_id: None,
        topic_id: None,
        id: "4".into(),
        sender: "alice".into(),
        reply_target: "chat-remote".into(),
        content: "/sessions".into(),
        channel: "telegram".into(),
        timestamp: 0,
        thread_ts: Some("42".into()),
        sender_name: None,
    };
    assert!(
        handle_runtime_command_if_needed(
            &ctx,
            &sessions_msg,
            Some(&channel),
            &conversation_history_key(&sessions_msg)
        )
        .await
    );

    let sent = channel_impl.sent.lock().unwrap();
    assert_eq!(sent.len(), 3);
    assert!(sent[0].content.contains("Started new session"));
    assert!(sent[0]
        .content
        .contains("In-memory channel history cleared for this chat."));

    assert!(sent[1].content.contains("**Status**"));
    assert!(sent[1].content.contains("Thread: `Telegram"));
    assert!(sent[1].content.contains("Provider: `anthropic`"));
    assert!(sent[1].content.contains("Model: `claude-3`"));
    assert!(sent[1].content.contains("In-memory turns: 1"));
    assert!(sent[1].content.contains("Turn: in progress"));

    assert!(sent[2].content.contains("**Recent sessions**"));
    assert!(sent[2].content.contains("→ `Telegram"));
    assert!(sent
        .iter()
        .all(|message| message.thread_ts.as_deref() == Some("42")));
}

#[tokio::test]
async fn handle_runtime_command_telegram_new_reports_unavailable_storage() {
    let tempdir = tempfile::tempdir().unwrap();
    let ctx = runtime_context(tempdir.path().to_path_buf());
    let channel_impl = Arc::new(RecordingChannel::default());
    let channel: Arc<dyn Channel> = channel_impl.clone();
    let sender_key = "telegram_alice_chat-remote";

    ctx.conversation_histories.lock().unwrap().insert(
        sender_key.to_string(),
        vec![ChatMessage::user("old history")],
    );

    let msg = ChannelMessage {
        reply_to_id: None,
        topic_id: None,
        id: "1".into(),
        sender: "alice".into(),
        reply_target: "chat-remote".into(),
        content: "/new".into(),
        channel: "telegram".into(),
        timestamp: 0,
        thread_ts: Some("42".into()),
        sender_name: None,
    };
    assert!(
        handle_runtime_command_if_needed(
            &ctx,
            &msg,
            Some(&channel),
            &conversation_history_key(&msg)
        )
        .await
    );
    let sent = channel_impl.sent.lock().unwrap();
    assert_eq!(sent.len(), 1);
    assert!(
        sent[0].content.contains("Created thread"),
        "{}",
        sent[0].content
    );
    assert!(sent[0]
        .content
        .contains("failed to persist the chat binding"));
    assert!(sent[0].content.contains("host storage is unavailable"));
    assert_eq!(sent[0].thread_ts.as_deref(), Some("42"));
}
