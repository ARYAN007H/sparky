//! Host adapters for TinyMCP's generic configured-server bridge.
//!
//! TinyMCP owns the generic bridge declarations, transport calls, rendering,
//! secret scrubbing, and call classification. This module keeps OpenHuman's
//! act gate and adapts the returned bus vocabulary to `tinytools`.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use tinytools::{PermissionLevel, Tool, ToolCallOptions, ToolResult};

use crate::{
    config::Config,
    mcp::{host, module_client::McpConfiguredClient},
    security::{SecurityPolicy, ToolOperation},
};

pub use tinymcp::tools::{McpListServersTool, McpListToolsTool};

/// The host's `mcp_call_tool`, backed by TinyMCP's module-owned static server
/// call operation.
pub struct McpCallTool {
    config: Arc<Config>,
    security: Arc<SecurityPolicy>,
    spec: tinymcp_bus::agent_tools::AgentToolSpec,
}

impl McpCallTool {
    fn new(config: Arc<Config>, security: Arc<SecurityPolicy>) -> Self {
        let spec = tinymcp_bus::agent_tools::bridge_tool_specs()
            .into_iter()
            .find(|spec| spec.name == "mcp_call_tool")
            .expect("TinyMCP bridge contract includes mcp_call_tool");
        Self {
            config,
            security,
            spec,
        }
    }
}

#[async_trait]
impl Tool for McpCallTool {
    fn name(&self) -> &str {
        &self.spec.name
    }

    fn description(&self) -> &str {
        &self.spec.description
    }

    fn parameters_schema(&self) -> Value {
        self.spec.parameters.clone()
    }

    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::Execute
    }

    fn supports_markdown(&self) -> bool {
        true
    }

    fn indirect_target(&self, args: &Value) -> Option<tinytools::IndirectCall> {
        let server = required_string_arg(args, "server").ok()?;
        let tool = required_string_arg(args, "tool").ok()?;
        let mut target = tinytools::ToolSubject::named(
            tinymcp_bus::agent_tools::naming::disambiguated_tool_name(&server, &server, &tool),
        )
        .with_family(server.clone())
        .with_tag(format!("mcp.server:{server}"))
        .with_tag(format!("mcp.server_id:{server}"))
        .with_tag(format!("mcp.tool:{tool}"))
        .with_permission(PermissionLevel::Execute);
        target.category = Some(tinytools::ToolCategory::Workflow);
        let call = tinytools::IndirectCall::new(target);
        Some(
            match args
                .get("arguments")
                .cloned()
                .and_then(|arguments| crate::normalize_tool_arguments(arguments).ok())
            {
                Some(arguments) => call.with_arguments(Value::Object(arguments)),
                None => call,
            },
        )
    }

    async fn execute_with_options(
        &self,
        args: Value,
        options: ToolCallOptions,
    ) -> anyhow::Result<ToolResult> {
        self.security
            .enforce_tool_operation(ToolOperation::Act, self.name())
            .map_err(|error| anyhow::anyhow!(error))?;

        let server = required_string_arg(&args, "server")?;
        let tool = required_string_arg(&args, "tool")?;
        let arguments = args
            .get("arguments")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("missing required `arguments` object"))?;
        let arguments = match crate::normalize_tool_arguments(arguments) {
            Ok(arguments) => Value::Object(arguments),
            Err(error) => {
                let outcome = tinymcp_bus::McpCallOutcome::failed(
                    &server,
                    &tool,
                    tinymcp_bus::McpCallError::new(tinymcp_bus::errors::INVALID_ARGUMENTS),
                );
                return Ok(with_outcome(
                    ToolResult::error(format!("`arguments`: {error}")),
                    &outcome,
                ));
            }
        };

        let client = match McpConfiguredClient::for_config(&self.config) {
            Ok(client) => client,
            Err(_) => {
                let outcome = unavailable_outcome(&server, &tool);
                return Ok(with_outcome(
                    ToolResult::error("mcp_call_tool failed: TinyMCP module is unavailable"),
                    &outcome,
                ));
            }
        };
        let client_config = host::client_config(&self.config);
        let call: tinymcp_bus::ClassifiedStaticToolCall = match client
            .call(
                &client_config,
                tinymcp_bus::names::methods::STATIC_CALL_TOOL_CLASSIFIED,
                (server.as_str(), tool.as_str(), arguments),
            )
            .await
        {
            Ok(call) => call,
            Err(_) => {
                let outcome = unavailable_outcome(&server, &tool);
                return Ok(with_outcome(
                    ToolResult::error("mcp_call_tool failed: TinyMCP module call failed"),
                    &outcome,
                ));
            }
        };

        Ok(classified_result(call, &server, &tool, options.prefer_markdown))
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        self.execute_with_options(args, ToolCallOptions::default())
            .await
    }
}

/// Builds the host-policy adapter for the generic MCP call tool.
pub fn mcp_call_tool(config: Arc<Config>, security: Arc<SecurityPolicy>) -> McpCallTool {
    McpCallTool::new(config, security)
}

fn classified_result(
    call: tinymcp_bus::ClassifiedStaticToolCall,
    server: &str,
    tool: &str,
    prefer_markdown: bool,
) -> ToolResult {
    if !call.outcome.ok {
        let mut result = ToolResult::error(
            call.error_message
                .unwrap_or_else(|| "mcp_call_tool failed".to_owned()),
        );
        result.metadata = serde_json::to_value(call.outcome).ok();
        return result;
    }

    let Some(mut rendered) = call.rendered else {
        let outcome = tinymcp_bus::McpCallOutcome::failed(
            server,
            tool,
            tinymcp_bus::McpCallError::new(tinymcp_bus::errors::INVALID_RESPONSE),
        );
        return with_outcome(ToolResult::error("mcp_call_tool failed"), &outcome);
    };

    let mut result = map_rendered_result(rendered.clone());
    if prefer_markdown && rendered.markdown_formatted.is_none() {
        rendered.markdown_formatted = Some(result.output());
        result.markdown_formatted = rendered.markdown_formatted;
    }

    result.metadata = call
        .envelope
        .and_then(|envelope| serde_json::to_value(envelope).ok())
        .or_else(|| serde_json::to_value(call.outcome).ok());
    result
}

fn map_rendered_result(rendered: tinymcp_bus::McpToolResult) -> ToolResult {
    ToolResult {
        content: rendered
            .content
            .into_iter()
            .map(|block| match block {
                tinymcp_bus::McpToolContent::Text { text } => tinytools::ToolContent::Text { text },
                tinymcp_bus::McpToolContent::Json { data } => tinytools::ToolContent::Json { data },
                other => tinytools::ToolContent::Json {
                    data: serde_json::to_value(other).unwrap_or(Value::Null),
                },
            })
            .collect(),
        is_error: rendered.is_error,
        markdown_formatted: rendered.markdown_formatted,
        ..ToolResult::default()
    }
}

fn unavailable_outcome(server: &str, tool: &str) -> tinymcp_bus::McpCallOutcome {
    tinymcp_bus::McpCallOutcome::failed(
        server,
        tool,
        tinymcp_bus::McpCallError::new(tinymcp_bus::errors::MODULE_UNAVAILABLE),
    )
}

fn with_outcome(
    mut result: ToolResult,
    outcome: &tinymcp_bus::McpCallOutcome,
) -> ToolResult {
    result.metadata = serde_json::to_value(outcome).ok();
    result
}

/// A required identifier argument, trimmed and with surrounding markdown
/// delimiters and trailing sentence punctuation removed.
fn required_string_arg(args: &Value, key: &str) -> anyhow::Result<String> {
    let missing = || anyhow::anyhow!("missing required `{key}`");
    let value = args
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(missing)?;
    let unpunctuated = value.trim_end_matches(['.', ',', ';', ':', '!']);
    let fenced = if unpunctuated.ends_with(['`', '*']) {
        unpunctuated
    } else {
        value
    };
    let cleaned = fenced.trim_matches(['`', '*']);
    if cleaned.is_empty() {
        return Err(missing());
    }
    Ok(cleaned.to_owned())
}

#[cfg(test)]
#[path = "mcp_tests.rs"]
mod tests;
