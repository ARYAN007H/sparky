//! Adapts native tool results to the value and failure contract expected by
//! TinyFlows. Tool execution itself remains in [`super`], which owns the
//! native registry and its policy snapshot.

use serde_json::{json, Value};
use tinytools::ToolResult;

/// Turns a failed native tool result into the flow-facing error body.
pub(crate) fn reject_failed_native_tool_result(
    slug: &str,
    result: &ToolResult,
) -> Result<(), String> {
    if !result.is_error {
        return Ok(());
    }
    let rendered = result.output();
    let detail = match rendered.trim() {
        "" => "no error detail returned by the tool",
        detail => detail,
    };
    tracing::warn!(
        target: "flows",
        %slug,
        %detail,
        "[flows] tool_call: native tool reported is_error — failing the step"
    );
    Err(format!("tool_call `{slug}` failed: {detail}"))
}

/// Projects a tool result to the value downstream flow expressions bind.
/// A single JSON block is returned directly; text and mixed blocks expose the
/// rendered output under `text`.
pub(crate) fn native_tool_payload(result: &ToolResult) -> Value {
    use tinytools::ToolContent;
    match result.content.as_slice() {
        [ToolContent::Json { data }] => data.clone(),
        _ => json!({ "text": result.output() }),
    }
}

#[cfg(test)]
#[path = "flow_backend_tests.rs"]
mod tests;
