use super::*;

#[test]
fn inference_agent_chat_controllers_keep_their_wire_names_and_namespace() {
    let controllers = all_registered_controllers();
    assert_eq!(controllers.len(), 2);
    assert_eq!(
        controllers[0].rpc_method_name(),
        crate::agent_runs::INFERENCE_AGENT_CHAT
    );
    assert_eq!(controllers[0].schema.namespace, "inference");
    assert_eq!(controllers[0].schema.function, "agent_chat");
    assert_eq!(controllers[1].schema.function, "agent_chat_simple");
}

#[test]
fn ordinary_agent_chat_still_ignores_inference_only_fields() {
    let params: AgentChatParams = serde_json::from_value(serde_json::json!({
        "message": "hello",
        "inference_url": 7,
        "api_key": {"unexpected": true},
        "inference_headers": "malformed",
        "agent_id": false
    }))
    .expect("unknown inference-only fields remain ignored on agent.chat");
    assert_eq!(params.message, "hello");
    assert!(params.cwd.is_none());
}

#[test]
fn inference_agent_chat_parses_its_route_fields() {
    let params: InferenceAgentChatParams = serde_json::from_value(serde_json::json!({
        "message": "hello",
        "agent_id": "alpha",
        "inference_url": "https://example.test/v1",
        "api_key": "secret",
        "inference_headers": [["x-test", "value"]]
    }))
    .expect("inference route params decode");
    assert_eq!(params.agent_id.as_deref(), Some("alpha"));
    assert_eq!(
        params.inference_url.as_deref(),
        Some("https://example.test/v1")
    );
    assert_eq!(
        params.inference_headers,
        vec![("x-test".into(), "value".into())]
    );
}
