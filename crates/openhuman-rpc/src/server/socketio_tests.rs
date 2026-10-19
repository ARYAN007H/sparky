use super::{
    channel_connection_update_payload, event_alias, origin_is_allowed_with_extra,
    emit_web_channel_event, publish_companion_state_changed, stamp_replay_timestamp,
    subscribe_companion_state_changed,
};

use crate::core_host::web_chat::{ScopedWebEvent, WebChannelEvent, WebEventContext};
use engineioxide::Packet;
use socketioxide::{extract::SocketRef, SocketIo};

#[test]
fn companion_state_transport_delivers_payload_to_bridge() {
    let mut rx = subscribe_companion_state_changed();
    let payload = serde_json::json!({
        "session_id": "session-1",
        "state": "thinking",
        "previous_state": "listening",
    });
    assert!(publish_companion_state_changed(payload.clone()) >= 1);
    assert_eq!(rx.try_recv().expect("payload delivered"), payload);
}

#[test]
fn replay_timestamp_is_stored_in_the_typed_event_context() {
    let mut event = crate::core_host::web_chat::approval_request_event(
        "request-1",
        "read_file",
        "inspect the document",
        &serde_json::json!({}),
        "thread-1",
        "client-1",
        None,
        None,
        false,
    );

    stamp_replay_timestamp(&mut event);

    assert!(event.context().ts.is_some());
}

#[test]
fn channel_connection_update_payload_connected_omits_error() {
    let payload = channel_connection_update_payload("discord", "connected", None);
    assert_eq!(payload["channel"], "discord");
    // Listener-backed channels always map to the bot_token auth mode.
    assert_eq!(payload["auth_mode"], "bot_token");
    assert_eq!(payload["status"], "connected");
    assert!(
        payload.get("last_error").is_none(),
        "connected payload must not carry a last_error: {payload}"
    );
}

#[test]
fn channel_connection_update_payload_error_carries_reason() {
    let payload =
        channel_connection_update_payload("discord", "error", Some("gateway closed (4004)"));
    assert_eq!(payload["channel"], "discord");
    assert_eq!(payload["auth_mode"], "bot_token");
    assert_eq!(payload["status"], "error");
    assert_eq!(payload["last_error"], "gateway closed (4004)");
}

#[test]
fn event_alias_translates_between_delimiters() {
    assert_eq!(event_alias("chat_done").as_deref(), Some("chat:done"));
    assert_eq!(event_alias("chat:error").as_deref(), Some("chat_error"));
    assert_eq!(event_alias("ready"), None);
}

#[test]
fn event_alias_suppressed_for_streaming_deltas() {
    // Streaming deltas must NOT be aliased — doubling every token frame is
    // the "double thinking-token streaming" bug. Discrete events still alias.
    assert_eq!(event_alias("thinking_delta"), None);
    assert_eq!(event_alias("text_delta"), None);
    assert_eq!(event_alias("tool_args_delta"), None);
    assert_eq!(event_alias("subagent_tool_args_delta"), None);
    // A *discrete* event that merely ends in `_delta` is NOT a streaming
    // token event and must keep its compat alias — this is what the explicit
    // STREAMING_DELTA_EVENTS set guarantees over the old `*_delta` suffix.
    assert_eq!(
        event_alias("inventory_delta").as_deref(),
        Some("inventory:delta")
    );
    // Sanity: a non-delta event in the same family still aliases.
    assert_eq!(event_alias("tool_call").as_deref(), Some("tool:call"));
}

#[tokio::test]
async fn web_events_reach_client_and_thread_rooms_once_with_compat_aliases() {
    let (_service, io) = SocketIo::new_svc();
    let (connected_tx, mut connected_rx) = tokio::sync::mpsc::unbounded_channel();
    io.ns("/", move |socket: SocketRef| {
        connected_tx.send(socket).expect("test receiver stays open");
    });

    let (_initiator_tx, mut initiator_rx) = io.new_dummy_sock("/", ()).await;
    let _ = initiator_rx.recv().await.expect("initiator connects");
    let initiator = connected_rx.recv().await.expect("initiator socket");
    initiator.join("client-a").expect("join client room");
    initiator.join("thread:thread-1").expect("join thread room");

    let (_reconnected_tx, mut reconnected_rx) = io.new_dummy_sock("/", ()).await;
    let _ = reconnected_rx.recv().await.expect("reconnected peer connects");
    let reconnected = connected_rx.recv().await.expect("reconnected peer socket");
    reconnected
        .join("thread:thread-1")
        .expect("reconnected peer joins thread room");

    let (_unrelated_tx, mut unrelated_rx) = io.new_dummy_sock("/", ()).await;
    let _ = unrelated_rx.recv().await.expect("unrelated peer connects");
    let unrelated = connected_rx.recv().await.expect("unrelated peer socket");
    unrelated
        .join("thread:thread-2")
        .expect("unrelated peer joins another thread");

    let done = ScopedWebEvent::new(WebChannelEvent::ChatDone {
        context: WebEventContext {
            client_id: "client-a".into(),
            thread_id: "thread-1".into(),
            request_id: "request-1".into(),
            ..Default::default()
        },
        full_response: Some("finished".into()),
        citations: None,
        usage: None,
        timing: None,
        parts: None,
    });
    emit_web_channel_event(&io, done);

    let initiator_packets = receive_messages(&mut initiator_rx, 2).await;
    let reconnected_packets = receive_messages(&mut reconnected_rx, 2).await;
    assert_eq!(event_names(&initiator_packets), ["chat_done", "chat:done"]);
    assert_eq!(event_names(&reconnected_packets), ["chat_done", "chat:done"]);
    assert!(initiator_packets.iter().all(|payload| payload["full_response"] == "finished"));
    assert!(reconnected_packets
        .iter()
        .all(|payload| payload["thread_id"] == "thread-1"));
    assert_no_message(&mut unrelated_rx).await;

    let delta = ScopedWebEvent::new(WebChannelEvent::TextDelta {
        context: WebEventContext {
            client_id: "client-a".into(),
            thread_id: "thread-1".into(),
            request_id: "request-1".into(),
            ..Default::default()
        },
        delta: "token".into(),
        iteration: 1,
    });
    emit_web_channel_event(&io, delta);

    let initiator_delta = receive_messages(&mut initiator_rx, 1).await;
    let reconnected_delta = receive_messages(&mut reconnected_rx, 1).await;
    assert_eq!(event_names(&initiator_delta), ["text_delta"]);
    assert_eq!(event_names(&reconnected_delta), ["text_delta"]);
    assert_eq!(initiator_delta[0]["delta"], "token");
    assert_eq!(reconnected_delta[0]["delta"], "token");
    assert_no_message(&mut unrelated_rx).await;
}

async fn receive_messages(
    rx: &mut tokio::sync::mpsc::Receiver<Packet>,
    count: usize,
) -> Vec<serde_json::Value> {
    let mut messages = Vec::with_capacity(count);
    while messages.len() < count {
        let packet = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .expect("expected event packet before timeout")
            .expect("socket stays connected");
        if let Packet::Message(message) = packet {
            let packet: Vec<serde_json::Value> = serde_json::from_str(&message)
                .unwrap_or_else(|error| panic!("invalid Socket.IO event packet {message:?}: {error}"));
            assert_eq!(packet.len(), 2, "event packet has name and payload");
            messages.push(packet[1].clone());
        }
    }
    messages
}

fn event_names(payloads: &[serde_json::Value]) -> Vec<&str> {
    payloads
        .iter()
        .map(|payload| payload["event"].as_str().expect("typed event discriminator"))
        .collect()
}

async fn assert_no_message(rx: &mut tokio::sync::mpsc::Receiver<Packet>) {
    while let Ok(Some(packet)) = tokio::time::timeout(std::time::Duration::from_millis(30), rx.recv()).await {
        assert!(!matches!(packet, Packet::Message(_)), "unrelated room received event: {packet:?}");
    }
}

#[test]
fn origin_allowlist_accepts_native_clients() {
    assert!(origin_is_allowed_with_extra(None, None));
}

#[test]
fn origin_allowlist_accepts_tauri_localhost_across_schemes() {
    // The CEF-served app shell stamps a platform-dependent Origin:
    //   - macOS / iOS use the native `tauri://localhost` scheme
    //   - Windows uses the CEF custom HTTP protocol → `http://tauri.localhost`
    //   - Linux / older Windows builds use `https://tauri.localhost`
    // All three flavours are the same trust tier (the bundled webview),
    // so each must pass the handshake gate.
    assert!(origin_is_allowed_with_extra(
        Some("tauri://localhost"),
        None
    ));
    assert!(origin_is_allowed_with_extra(
        Some("https://tauri.localhost"),
        None
    ));
    assert!(origin_is_allowed_with_extra(
        Some("http://tauri.localhost"),
        None
    ));
}

#[test]
fn origin_allowlist_accepts_local_dev_server() {
    assert!(origin_is_allowed_with_extra(
        Some("http://localhost:1420"),
        None
    ));
    assert!(origin_is_allowed_with_extra(
        Some("http://127.0.0.1:1420"),
        None
    ));
    assert!(origin_is_allowed_with_extra(
        Some("http://[::1]:1420"),
        None
    ));
    // Loopback without an explicit port (some CEF builds stamp this
    // shape when the shell runs on the default port).
    assert!(origin_is_allowed_with_extra(Some("http://localhost"), None));
}

#[test]
fn origin_allowlist_rejects_cross_origin_browser_pages() {
    assert!(!origin_is_allowed_with_extra(
        Some("https://attacker.example"),
        None
    ));
    assert!(!origin_is_allowed_with_extra(
        Some("http://evil.local"),
        None
    ));
    assert!(!origin_is_allowed_with_extra(Some("null"), None));
    assert!(!origin_is_allowed_with_extra(Some(""), None));
}

#[test]
fn origin_allowlist_rejects_host_prefix_decoys() {
    // Regression: `starts_with("localhost")` accepted these; the exact
    // host match must not.
    assert!(!origin_is_allowed_with_extra(
        Some("http://localhost.attacker.example"),
        None
    ));
    assert!(!origin_is_allowed_with_extra(
        Some("http://127.0.0.1.attacker.example"),
        None
    ));
    assert!(!origin_is_allowed_with_extra(
        Some("https://localhost-evil"),
        None
    ));
    // Same rule applies to the tauri.localhost host — must be exact.
    assert!(!origin_is_allowed_with_extra(
        Some("http://tauri.localhost.attacker.example"),
        None
    ));
    assert!(!origin_is_allowed_with_extra(
        Some("https://tauri.localhost.evil"),
        None
    ));
}

#[test]
fn origin_allowlist_accepts_env_allowlisted_origin() {
    // Regression: a non-loopback browser origin that the JSON-RPC CORS
    // layer accepts must also pass the socket handshake, or chat (a
    // socket-only transport) silently never connects.
    let extra = Some("https://box.example.ts.net:9000");
    assert!(origin_is_allowed_with_extra(
        Some("https://box.example.ts.net:9000"),
        extra
    ));
}

#[test]
fn origin_allowlist_env_handles_comma_list_and_whitespace() {
    let extra = Some("https://a.example:9000 , https://b.example:8443");
    assert!(origin_is_allowed_with_extra(
        Some("https://a.example:9000"),
        extra
    ));
    assert!(origin_is_allowed_with_extra(
        Some("https://b.example:8443"),
        extra
    ));
    assert!(!origin_is_allowed_with_extra(
        Some("https://c.example"),
        extra
    ));
}

#[test]
fn origin_allowlist_env_requires_exact_match() {
    // Populating the allowlist must not loosen the decoy rules: no port
    // wildcarding, no scheme swapping, no suffix matching.
    let extra = Some("https://box.example.ts.net:9000");
    assert!(!origin_is_allowed_with_extra(
        Some("http://box.example.ts.net:9000"),
        extra
    ));
    assert!(!origin_is_allowed_with_extra(
        Some("https://box.example.ts.net:9001"),
        extra
    ));
    assert!(!origin_is_allowed_with_extra(
        Some("https://box.example.ts.net.attacker.example:9000"),
        extra
    ));
}

#[test]
fn origin_allowlist_env_does_not_rescue_null_or_empty() {
    let extra = Some("null,");
    assert!(!origin_is_allowed_with_extra(Some("null"), extra));
    assert!(!origin_is_allowed_with_extra(Some(""), extra));
}

#[test]
fn origin_allowlist_rejects_unparseable_origin() {
    assert!(!origin_is_allowed_with_extra(Some("not a url"), None));
    assert!(!origin_is_allowed_with_extra(
        Some("javascript:alert(1)"),
        None
    ));
}
