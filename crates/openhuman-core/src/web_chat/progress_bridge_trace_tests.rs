use super::*;

#[test]
fn disabled_export_does_not_create_a_trace_context() {
    let config = crate::config::Config::default();
    let context = trace_context(
        &config,
        &ChatRequestMetadata::default(),
        "socket-client",
        "thread-123",
        "request-456",
    );
    assert!(context.is_none());
}

#[test]
fn trace_context_keeps_request_session_group_and_transport_client_distinct() {
    let mut config = crate::config::Config::default();
    config.observability.share_usage_data = false;
    config.observability.agent_tracing.enabled = true;
    config.observability.agent_tracing.capture_content = false;
    let metadata = ChatRequestMetadata {
        source: Some("autonomous".into()),
        session_id: Some(93),
        agent_id: Some("writer".into()),
        ..Default::default()
    };

    let context = trace_context(
        &config,
        &metadata,
        "socket-client-7",
        "conversation-4",
        "request-2",
    )
    .expect("local tracing enabled");

    assert_eq!(context.session_id, "93:request-2");
    assert_eq!(context.session_group.as_deref(), Some("conversation-4"));
    assert_eq!(context.client_id.as_deref(), Some("socket-client-7"));
    assert_eq!(context.agent_id.as_deref(), Some("writer"));
    assert_eq!(context.channel_source.as_deref(), Some("autonomous"));
    assert_eq!(context.run_type.as_str(), "autonomous_task");
    assert!(!context.capture_content);
}
