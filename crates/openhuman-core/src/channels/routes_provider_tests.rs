use super::tests::{runtime_context, RecordingChannel};
use super::*;
use crate::channels::context::conversation_history_key;
use crate::channels::{Channel, SendMessage};
use std::path::PathBuf;
use std::sync::Arc;
use tinychannels_bus::ChannelMessage;

#[tokio::test]
async fn handle_runtime_command_routes_configured_provider_slug_and_alias() {
    use crate::config::schema::{AuthStyle, CloudProviderCreds};

    let mut ctx = runtime_context(PathBuf::from("/tmp"));
    let config = crate::config::Config {
        cloud_providers: vec![
            CloudProviderCreds {
                id: "p_custom_route_123".into(),
                slug: "custom-route".into(),
                label: "Human Friendly Name".into(),
                endpoint: "https://provider.example.test/v1".into(),
                auth_style: AuthStyle::Bearer,
                default_model: None,
            },
            CloudProviderCreds {
                id: "p_managed_123".into(),
                slug: "openhuman".into(),
                label: "Managed Backend".into(),
                endpoint: "https://api.openhuman.ai/v1".into(),
                auth_style: AuthStyle::OpenhumanJwt,
                default_model: None,
            },
        ],
        ..crate::config::Config::default()
    };
    ctx.config = Some(Arc::new(config));
    let channel_impl = Arc::new(RecordingChannel::default());
    let channel: Arc<dyn Channel> = channel_impl.clone();
    let key = "telegram_alice_room";

    for requested in ["custom-route", "backend"] {
        let msg = ChannelMessage {
            id: requested.into(),
            sender: "alice".into(),
            reply_target: "room".into(),
            content: format!("/models {requested}"),
            channel: "telegram".into(),
            timestamp: 0,
            thread_ts: None,
            sender_name: None,
        };
        assert!(
            handle_runtime_command_if_needed(
                &ctx,
                &msg,
                Some(&channel),
                &conversation_history_key(&msg),
            )
            .await
        );
    }

    assert_eq!(get_route_selection(&ctx, key).provider, "openhuman");
    let sent = channel_impl.sent.lock().unwrap();
    assert_eq!(sent.len(), 1, "the configured slug should be accepted");
    assert!(sent[0]
        .content
        .contains("Provider switched to `custom-route`"));
}
