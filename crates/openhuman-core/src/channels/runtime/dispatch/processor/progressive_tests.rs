use super::*;

#[test]
fn native_text_progress_keeps_iteration_and_content() {
    let event = output_event(AgentProgress::TextDelta {
        delta: "partial answer".into(),
        iteration: 3,
    })
    .expect("text progress is projected");
    assert!(matches!(
        event,
        ChannelOutputEvent::TextDelta { text, iteration: Some(3) }
            if text == "partial answer"
    ));
}

#[test]
fn native_tool_progress_keeps_correlation_and_display_metadata() {
    let event = output_event(AgentProgress::ToolCallStarted {
        call_id: "call-1".into(),
        tool_name: "file_read".into(),
        arguments: serde_json::json!({"path":"README.md"}),
        iteration: 2,
        display_label: Some("Reading file".into()),
        display_detail: Some("README.md".into()),
    })
    .expect("tool progress is projected");
    assert!(matches!(
        event,
        ChannelOutputEvent::ToolCallStarted {
            call_id,
            tool_name,
            arguments,
            iteration: 2,
            display_label: Some(label),
            display_detail: Some(detail),
        } if call_id == "call-1"
            && tool_name == "file_read"
            && arguments["path"] == "README.md"
            && label == "Reading file"
            && detail == "README.md"
    ));
}

#[test]
fn progressive_delivery_status_rejects_partial_or_unsupported_terminal_results() {
    use tinychannels_bus::module_progressive::{ProgressiveOutputFailure, ProgressiveResult};

    let receipt = tinychannels_bus::channel::MessageReceipt {
        primary_platform_message_id: Some("provider-message-9".into()),
        ..Default::default()
    };
    let delivered = ProgressiveResult::DeliveryOutcome {
        outcome: Box::new(tinychannels_bus::channel::ChannelSendOutcome {
            receipt: receipt.clone(),
            error: None,
        }),
    };
    assert!(delivery_succeeded(&delivered));
    assert!(matches!(
        delivered,
        ProgressiveResult::DeliveryOutcome { outcome }
            if outcome.receipt.primary_platform_message_id.as_deref() == Some("provider-message-9")
    ));

    let partial = ProgressiveResult::DeliveryBatch {
        outcomes: vec![tinychannels_bus::channel::ChannelSendOutcome {
            receipt,
            error: Some(tinychannels_bus::channel::ChannelSendError {
                kind: tinychannels_bus::channel::SendErrorKind::Transient,
                sent_before_error: true,
                ..Default::default()
            }),
        }],
        unsupported: Vec::new(),
        failure: Some(ProgressiveOutputFailure::ProviderTransport),
    };
    assert!(!delivery_succeeded(&partial));

    let unsupported = ProgressiveResult::DeliveryUnsupported {
        parts: vec![tinychannels_bus::channel::OutboundContentPart::Content {
            part: tinychannels_bus::channel::ContentPart::Text {
                text: "retained content".into(),
                markdown_dialect: None,
            },
        }],
    };
    assert!(!delivery_succeeded(&unsupported));
}
