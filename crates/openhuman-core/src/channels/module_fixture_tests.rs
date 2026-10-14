//! The pinned TinyChannels release fixture for host-side unit tests.
//!
//! These tests exercise the real owning module over TinyBus without contacting
//! provider services. The host test runner stages the checksum-pinned module
//! release in `OPENHUMAN_MODULE_PATH`, and the regular release loader attaches
//! it to the process-wide module host.

use crate::config::Config;
use crate::modules::client::ModuleClient;
use tinychannels_bus::methods;

/// Return a client for the checksum-pinned TinyChannels test module.
pub(crate) async fn client() -> ModuleClient {
    let mut config = Config::default();
    config.modules.enabled = true;
    config.modules.allow_download = false;
    ModuleClient::new(config)
}

#[tokio::test]
async fn released_module_prepares_runtime_limits_over_confidential_bus_call() {
    use tinychannels_bus::module_control::{ChannelControlRequest, ChannelControlResult};

    let response: ChannelControlResult = client()
        .await
        .call_confidential(
            "tinychannels",
            methods::PREPARE_CHANNEL_CONTROL,
            (ChannelControlRequest::RuntimeLimits {
                timeout_secs: 1,
                listener_count: 100,
            },),
        )
        .await
        .expect("the attached linked module should answer a confidential call");

    assert_eq!(
        response,
        ChannelControlResult::RuntimeLimits {
            timeout_secs: 30,
            max_in_flight: 64,
        }
    );
}

#[tokio::test]
async fn released_module_prepares_legacy_inbound_facts_over_confidential_bus_call() {
    use tinychannels_bus::module_inbound::{
        ChannelInboundSource, PrepareChannelInboundRequest, PreparedChannelInbound,
    };
    use tinychannels_bus::ChannelMessage;

    let message = ChannelMessage {
        id: "message-1".into(),
        channel: "discord".into(),
        sender: "sender/a".into(),
        reply_target: "room#b".into(),
        content: "fixture content".into(),
        timestamp: 42,
        thread_ts: Some("thread/c".into()),
        sender_name: Some("Display".into()),
    };
    let response: PreparedChannelInbound = client()
        .await
        .call_confidential(
            "tinychannels",
            methods::PREPARE_CHANNEL_INBOUND,
            (PrepareChannelInboundRequest {
                source: ChannelInboundSource::Legacy {
                    message,
                    envelope: None,
                },
                namespace: "openhuman".into(),
                session_policy: Default::default(),
            },),
        )
        .await
        .expect("the attached linked module should prepare inbound facts");

    assert_eq!(
        response.thread_id,
        "channel:discord/sender%2Fa/room%23b#thread:thread%2Fc"
    );
    assert_eq!(response.client_id, "inbound:discord:sender/a");
    assert_eq!(
        response.history_key,
        "discord_sender/a_room#b_thread:thread/c"
    );
    assert_eq!(
        response.session_key,
        "openhuman:discord:default:unknown:room#b:thread/c"
    );
    assert_eq!(response.message.timestamp, 42);
    assert_eq!(response.message.sender_name.as_deref(), Some("Display"));
    assert_eq!(response.envelope.text, "fixture content");
}
