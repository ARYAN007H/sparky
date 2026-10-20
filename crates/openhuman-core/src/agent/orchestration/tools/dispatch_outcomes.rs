//! Result helpers shared by the asynchronous spawn completion path.

pub(crate) fn is_unexecuted_tool_call_stub(output: &str) -> bool {
    use tinytools_agent::dialect::{contains_tool_call_payload, strip_tool_calls_from_response};
    if !contains_tool_call_payload(output) {
        return false;
    }
    if let Ok(serde_json::Value::Object(map)) = serde_json::from_str(output.trim()) {
        if map.contains_key("tool_calls") {
            return true;
        }
    }
    !strip_tool_calls_from_response(output)
        .chars()
        .any(char::is_alphanumeric)
}

pub(crate) fn incomplete_stop(
    reason: &str,
    failure_cause: Option<tinyagents_harness::no_progress::ClassifiedStopCause>,
) -> Option<tinyagents_harness::turn_stop::TurnStop> {
    Some(
        crate::agent::session_host::stop_summary::stop_from_incomplete_with_cause(
            reason,
            failure_cause,
        ),
    )
}
