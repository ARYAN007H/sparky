use super::*;
use crate::config::test_env::EnvVarGuard;
use std::sync::Arc;
use std::time::Duration;
use tinychannels_bus::module_progressive::{
    ChannelOutputEvent, ProgressiveCommand, ProgressiveResult, ProgressiveTransportRequest,
    ProgressiveTransportResult,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn compiled_module_progressive_send_reenters_typed_callback_without_deadlock() {
    let env_lock = EnvVarGuard::locked_async().await;
    let server = MockServer::start().await;
    let _env = env_lock.with("OPENHUMAN_TELEGRAM_BOT_API_BASE", server.uri());
    let _storage = crate::channels::module_fixture::install_memory_storage();
    crate::channels::module_fixture::client().await;

    Mock::given(method("POST"))
        .and(path("/bottoken/getUpdates"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ok": true,
            "result": []
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/bottoken/deleteWebhook"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ok": true,
            "result": true
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/bottoken/editMessageText"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ok": true,
            "result": {
                "message_id": 991,
                "date": 1,
                "chat": { "id": 42, "type": "private" },
                "text": "updated"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/bottoken/deleteMessage"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ok": true,
            "result": true
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/bottoken/sendMessage"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ok": true,
            "result": {
                "message_id": 991,
                "date": 1,
                "chat": { "id": 42, "type": "private" },
                "text": "done"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let workspace = tempfile::tempdir().expect("temporary module workspace");
    let mut config = Config::default();
    config.workspace_dir = workspace.path().to_path_buf();
    config.modules.allow_download = false;
    config.modules.source = crate::config::schema::ModuleSourceMode::Linked;
    config.channels_config.telegram = Some(tinychannels_bus::config::TelegramConfig {
        bot_token: "bottoken".into(),
        chat_id: None,
        allowed_users: vec!["42".into()],
        stream_mode: Default::default(),
        draft_update_interval_ms: 1,
        silent_streaming: true,
        mention_only: false,
    });
    let context = Arc::new(OnceLock::new());
    let (dispatch, _receiver) = tokio::sync::mpsc::channel(4);
    let mut runtime = ModuleChannelRuntime::prepare(
        &crate::core::runtime::DispatchCtx::host_default(),
        Arc::new(config.clone()),
        context,
        dispatch,
    )
    .await
    .expect("compiled TinyBus module should prepare configured Telegram");
    runtime
        .start(&config)
        .await
        .expect("mock Telegram provider starts without a network dependency");

    let scope = tinychannels_bus::ConversationRef {
        kind: tinychannels_bus::ConversationKind::Dm,
        id: "42".into(),
        ..Default::default()
    };
    let opened: ProgressiveResult = tokio::time::timeout(
        Duration::from_secs(10),
        runtime.lease.object().call_confidential(
            tinychannels_bus::methods::EXECUTE_PROGRESSIVE,
            (ProgressiveCommand::OpenOutput {
                channel: tinychannels_bus::ChannelRef {
                    id: "telegram".into(),
                    account_id: Some("telegram".into()),
                },
                conversation: scope,
                turn_id: "compiled-module-turn".into(),
                lifetime_ms: 30_000,
            },),
        ),
    )
    .await
    .expect("opening progressive output is bounded")
    .expect("module should open a private progressive handle");
    let ProgressiveResult::Opened { handle } = opened else {
        panic!("expected compiled module to return an output handle");
    };

    let completed: ProgressiveResult = tokio::time::timeout(
        Duration::from_secs(10),
        runtime.lease.object().call_confidential(
            tinychannels_bus::methods::EXECUTE_PROGRESSIVE,
            (ProgressiveCommand::FeedOutput {
                handle,
                events: vec![
                    ChannelOutputEvent::TextDelta {
                        text: "working".into(),
                        iteration: Some(1),
                    },
                    ChannelOutputEvent::FinalMessage {
                        text: "nested callback delivery".into(),
                        parts: Vec::new(),
                        receipt: None,
                    },
                ],
            },),
        ),
    )
    .await
    .expect("module-to-host-to-module send must not deadlock")
    .expect("the configured module adapter should deliver the final message");
    let receipt = match completed {
        ProgressiveResult::DeliveryOutcome { outcome } => {
            assert!(outcome.error.is_none());
            assert_eq!(
                outcome.receipt.primary_platform_message_id.as_deref(),
                Some("991")
            );
            outcome.receipt
        }
        other => panic!("expected typed delivery outcome, got {other:?}"),
    };

    // Exercise the actual host adapter against the same compiled module object.
    // These Edit/Delete callback operations must retain the module receipt scope
    // and return the provider's real outcome through the bus contract.
    let adapter = ModuleChannel {
        name: "telegram".into(),
        object: runtime.lease.object().clone(),
        interaction: tinychannels_bus::ChannelInteractionInfo {
            capabilities: Default::default(),
            proactive_target: None,
            supports_reactions: false,
            supports_draft_updates: false,
        },
        relay: None,
        relay_fronted: false,
    };
    let channel_ref = tinychannels_bus::ChannelRef {
        id: "telegram".into(),
        account_id: Some("telegram".into()),
    };
    let conversation = tinychannels_bus::ConversationRef {
        kind: tinychannels_bus::ConversationKind::Dm,
        id: "42".into(),
        ..Default::default()
    };
    let edited = adapter
        .dispatch_progressive(ProgressiveTransportRequest::Edit {
            channel: "telegram".into(),
            turn_id: Some("compiled-module-turn".into()),
            channel_ref: Some(channel_ref.clone()),
            conversation: Some(conversation.clone()),
            message_id: "991".into(),
            receipt: Some(receipt.clone()),
            text: "updated".into(),
        })
        .await
        .expect("scoped edit should cross the compiled module bus");
    assert!(
        matches!(edited, ProgressiveTransportResult::Outcome(outcome) if outcome.error.is_none())
    );
    let deleted = adapter
        .dispatch_progressive(ProgressiveTransportRequest::Delete {
            channel: "telegram".into(),
            turn_id: Some("compiled-module-turn".into()),
            channel_ref: Some(channel_ref.clone()),
            conversation: Some(conversation.clone()),
            message_id: "991".into(),
            receipt: Some(receipt.clone()),
        })
        .await
        .expect("scoped delete should cross the compiled module bus");
    assert!(matches!(deleted, ProgressiveTransportResult::Acknowledged));
    let mut wrong_scope = conversation.clone();
    wrong_scope.kind = tinychannels_bus::ConversationKind::Group;
    let rejected = adapter
        .dispatch_progressive(ProgressiveTransportRequest::Delete {
            channel: "telegram".into(),
            turn_id: Some("compiled-module-turn".into()),
            channel_ref: Some(channel_ref),
            conversation: Some(wrong_scope),
            message_id: "991".into(),
            receipt: Some(receipt),
        })
        .await
        .expect("a mismatched scoped receipt should fail closed");
    assert!(matches!(rejected, ProgressiveTransportResult::Unavailable));

    let requests = server.received_requests().await.expect("mock requests");
    let send = requests
        .iter()
        .find(|request| request.url.path() == "/bottoken/sendMessage")
        .expect("one mock Telegram sendMessage request");
    let body: serde_json::Value = serde_json::from_slice(&send.body).expect("JSON request body");
    assert_eq!(body["chat_id"], "42");
    assert_eq!(body["text"], "nested callback delivery");
    server.verify().await;
    runtime
        .close()
        .await
        .expect("module runtime should shut down");
}
