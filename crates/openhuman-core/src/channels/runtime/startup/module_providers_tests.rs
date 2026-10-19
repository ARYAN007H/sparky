use super::*;
use crate::config::test_env::EnvVarGuard;
use std::sync::Arc;
use std::time::Duration;
use tinychannels_bus::module_progressive::{
    ChannelOutputEvent, ProgressiveCommand, ProgressiveResult,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn compiled_module_progressive_send_reenters_typed_callback_without_deadlock() {
    let server = MockServer::start().await;
    let _env = EnvVarGuard::locked_set_async("OPENHUMAN_TELEGRAM_BOT_API_BASE", server.uri()).await;
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
        Duration::from_secs(3),
        runtime.lease.object().call_confidential(
            tinychannels_bus::methods::EXECUTE_PROGRESSIVE,
            (ProgressiveCommand::OpenOutput {
                channel: tinychannels_bus::ChannelRef {
                    id: "telegram".into(),
                    account_id: Some("telegram".into()),
                },
                conversation: scope,
                turn_id: "compiled-module-turn".into(),
                lifetime_ms: 5_000,
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
        Duration::from_secs(3),
        runtime.lease.object().call_confidential(
            tinychannels_bus::methods::EXECUTE_PROGRESSIVE,
            (ProgressiveCommand::FeedOutput {
                handle,
                events: vec![ChannelOutputEvent::FinalMessage {
                    text: "nested callback delivery".into(),
                    parts: Vec::new(),
                    receipt: None,
                }],
            },),
        ),
    )
    .await
    .expect("module-to-host-to-module send must not deadlock")
    .expect("the configured module adapter should deliver the final message");
    assert!(matches!(
        completed,
        ProgressiveResult::DeliveryOutcome { outcome }
            if outcome.error.is_none()
                && outcome.receipt.primary_platform_message_id.as_deref() == Some("991")
    ));
    server.verify().await;
    runtime
        .close()
        .await
        .expect("module runtime should shut down");
}
