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
