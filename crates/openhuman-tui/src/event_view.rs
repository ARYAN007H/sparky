//! Small typed read model for the fields the terminal transcript displays.
//!
//! The wire contract remains TinyChannels' tagged `WebChannelEvent`; this is a
//! consumer projection rather than another wire DTO.

use openhuman_rpc::embed::chat_surface::WebChannelEvent;
use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub(crate) struct SubagentView {
    pub task_id: Option<String>,
    pub display_name: Option<String>,
    pub elapsed_ms: Option<u64>,
    pub output: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct WebEventView {
    pub event: String,
    pub client_id: String,
    pub thread_id: String,
    pub request_id: String,
    pub seq: Option<u64>,
    pub delta: Option<String>,
    pub full_response: Option<String>,
    pub message: Option<String>,
    pub error_retryable: Option<bool>,
    pub error_retry_after_ms: Option<u64>,
    pub error_type: Option<String>,
    pub output: Option<String>,
    pub skill_id: Option<String>,
    pub tool_call_id: Option<String>,
    pub tool_name: Option<String>,
    pub success: Option<bool>,
    pub args: Option<Value>,
    pub elapsed_ms: Option<u64>,
    pub tool_display_label: Option<String>,
    pub subagent: Option<SubagentView>,
    pub failure: Option<Value>,
}

impl WebEventView {
    pub(crate) fn from_event(event: &WebChannelEvent) -> Self {
        let wire = serde_json::to_value(event).expect("typed web events serialize");
        let context = event.context();
        let string = |name: &str| wire.get(name).and_then(Value::as_str).map(str::to_owned);
        let number = |name: &str| wire.get(name).and_then(Value::as_u64);
        let boolean = |name: &str| wire.get(name).and_then(Value::as_bool);
        let subagent = wire.get("subagent").map(|detail| SubagentView {
            task_id: detail.get("task_id").and_then(Value::as_str).map(str::to_owned),
            display_name: detail
                .get("display_name")
                .and_then(Value::as_str)
                .map(str::to_owned),
            elapsed_ms: detail.get("elapsed_ms").and_then(Value::as_u64),
            output: detail.get("output").and_then(Value::as_str).map(str::to_owned),
        });
        Self {
            event: event.event_name().to_owned(),
            client_id: context.client_id.clone(),
            thread_id: context.thread_id.clone(),
            request_id: context.request_id.clone(),
            seq: context.seq,
            delta: string("delta"),
            full_response: string("full_response"),
            message: string("message"),
            error_retryable: boolean("error_retryable"),
            error_retry_after_ms: number("error_retry_after_ms"),
            error_type: string("error_type"),
            output: string("output"),
            skill_id: string("skill_id"),
            tool_call_id: string("tool_call_id"),
            tool_name: string("tool_name"),
            success: boolean("success"),
            args: wire.get("args").cloned(),
            elapsed_ms: number("elapsed_ms"),
            tool_display_label: string("tool_display_label"),
            subagent,
            failure: wire.get("failure").cloned(),
        }
    }
}
