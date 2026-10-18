use super::*;
use tinychannels_bus::channel::{ContentPart, OutboundContentPart};

fn context() -> bus_web::WebEventContext {
    bus_web::WebEventContext {
        client_id: "c1".into(),
        thread_id: "t1".into(),
        request_id: "r1".into(),
        seq: Some(4),
        ts: Some(99),
        ..Default::default()
    }
}

#[test]
fn a_typed_event_becomes_the_flat_form_with_the_same_wire_fields() {
    let flat = WebChannelEvent::from(bus_web::WebChannelEvent::ChatDone {
        context: context(),
        full_response: Some("hello".into()),
        citations: Some(serde_json::json!([{ "id": 1 }])),
        usage: Some(TurnUsagePayload {
            input_tokens: 3,
            ..Default::default()
        }),
        timing: Some(TurnTimingPayload {
            total_ms: Some(10),
            ..Default::default()
        }),
        parts: None,
    });
    assert_eq!(flat.event, "chat_done");
    assert_eq!(
        (
            flat.client_id.as_str(),
            flat.thread_id.as_str(),
            flat.request_id.as_str()
        ),
        ("c1", "t1", "r1")
    );
    assert_eq!((flat.seq, flat.ts), (Some(4), Some(99)));
    assert_eq!(flat.full_response.as_deref(), Some("hello"));
    assert_eq!(flat.usage.expect("usage").input_tokens, 3);
    assert_eq!(flat.timing.expect("timing").total_ms, Some(10));
    assert!(flat.citations.is_some());
    // Tenant stamps are the host's, never the wire's.
    assert!(flat.agent.is_none() && flat.profile.is_none());
}

#[test]
fn a_null_argument_payload_and_an_unnamed_tool_are_absent_not_empty() {
    let call = WebChannelEvent::from(bus_web::WebChannelEvent::ToolCall {
        context: context(),
        tool_name: Some("shell".into()),
        skill_id: None,
        args: Some(serde_json::Value::Null),
        round: Some(1),
        tool_call_id: Some("call".into()),
        tool_display_label: None,
        tool_display_detail: None,
    });
    assert_eq!(call.event, "tool_call");
    assert!(call.args.is_none());
    assert_eq!(call.tool_name.as_deref(), Some("shell"));

    let delta = WebChannelEvent::from(bus_web::WebChannelEvent::ToolArgsDelta {
        context: context(),
        delta: Some("{".into()),
        delta_kind: Some("tool_args".into()),
        tool_call_id: Some("call".into()),
        tool_name: Some(String::new()),
        skill_id: None,
        round: Some(1),
    });
    assert_eq!(delta.event, "tool_args_delta");
    assert!(delta.tool_name.is_none());
    assert_eq!(delta.tool_call_id.as_deref(), Some("call"));
}

#[test]
fn typed_content_parts_ride_the_flat_form_to_the_wire() {
    let flat = WebChannelEvent::from(bus_web::WebChannelEvent::ChatMessage {
        context: context(),
        message: None,
        full_response: Some("look".into()),
        parts: Some(vec![OutboundContentPart::Content {
            part: ContentPart::Text {
                text: "look".into(),
                markdown_dialect: None,
            },
        }]),
    });
    assert_eq!(flat.event, "chat_message");
    let wire = serde_json::to_value(&flat).expect("serializes");
    assert_eq!(wire["parts"][0]["part"]["text"], "look");
}

#[test]
fn tenant_stamps_route_but_never_serialize() {
    let mut event = WebChannelEvent {
        event: "chat_done".into(),
        ..Default::default()
    };
    event.stamp_tenant(Some(crate::core::runtime::Tenant {
        profile: Some("p".into()),
        agent: Some("a".into()),
    }));
    assert!(event.belongs_to("a") && event.belongs_to_profile("p"));
    assert!(!event.belongs_to("other"));
    let wire = serde_json::to_value(&event).expect("serializes");
    assert!(wire.get("agent").is_none() && wire.get("profile").is_none());
}
