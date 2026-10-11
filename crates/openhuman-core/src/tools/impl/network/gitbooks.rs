//! `gitbooks` — answer questions about OpenHuman through the TinyMCP module's
//! confidential one-shot tool-call contract.

use crate::{
    config::Config,
    mcp::http_client::redact_endpoint,
    modules::client::{ModuleCallError, ModuleClient},
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::time::Duration;
use tinytools::{PermissionLevel, Tool, ToolResult};

pub struct GitbooksSearchTool {
    endpoint: String,
    timeout_secs: u64,
    client: ModuleClient,
}

impl GitbooksSearchTool {
    /// Builds the tool.
    ///
    /// The endpoint and timeout are retained as one-shot server configuration;
    /// TinyMCP owns the HTTP client and transport lifecycle.
    ///
    /// # Errors
    ///
    /// Returns the reason the tool configuration could not be built.
    pub fn new(endpoint: String, timeout_secs: u64, config: Config) -> Result<Self, String> {
        Ok(Self {
            endpoint,
            timeout_secs,
            client: ModuleClient::new(config).with_timeout(Duration::from_secs(timeout_secs)),
        })
    }

    #[cfg(test)]
    fn with_client(endpoint: String, timeout_secs: u64, client: ModuleClient) -> Self {
        Self {
            endpoint,
            timeout_secs,
            client,
        }
    }
}

#[async_trait]
impl Tool for GitbooksSearchTool {
    fn exposure(&self) -> tinytools::ToolExposure {
        tinytools::ToolExposure::Deferred
    }

    fn name(&self) -> &str {
        "gitbooks_search"
    }

    fn description(&self) -> &str {
        "Search the OpenHuman product documentation. Use this to answer questions about how \
        OpenHuman works, find features, look up configuration, or locate guides. Returns \
        excerpts with page titles and links — follow up with `gitbooks_get_page` for the \
        full markdown of a page."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Natural-language question or keyword query about OpenHuman."
                }
            },
            "required": ["query"]
        })
    }

    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::ReadOnly
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let query = args
            .get("query")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("Missing 'query' parameter"))?;
        if query.trim().is_empty() {
            return Ok(ToolResult::error("query cannot be empty"));
        }

        tracing::debug!(
            target: "[gitbooks]",
            endpoint = %redact_endpoint(&self.endpoint),
            tool = "searchDocumentation",
            "dispatching via shared MCP client"
        );

        Ok(self
            .call("searchDocumentation", json!({ "query": query }), "gitbooks_search")
            .await)
    }
}

pub struct GitbooksGetPageTool {
    endpoint: String,
    timeout_secs: u64,
    client: ModuleClient,
}

impl GitbooksGetPageTool {
    /// Builds the tool.
    ///
    /// The endpoint and timeout are retained as one-shot server configuration;
    /// TinyMCP owns the HTTP client and transport lifecycle.
    ///
    /// # Errors
    ///
    /// Returns the reason the tool configuration could not be built.
    pub fn new(endpoint: String, timeout_secs: u64, config: Config) -> Result<Self, String> {
        Ok(Self {
            endpoint,
            timeout_secs,
            client: ModuleClient::new(config).with_timeout(Duration::from_secs(timeout_secs)),
        })
    }

    #[cfg(test)]
    fn with_client(endpoint: String, timeout_secs: u64, client: ModuleClient) -> Self {
        Self {
            endpoint,
            timeout_secs,
            client,
        }
    }
}

#[async_trait]
impl Tool for GitbooksGetPageTool {
    fn exposure(&self) -> tinytools::ToolExposure {
        tinytools::ToolExposure::Deferred
    }

    fn name(&self) -> &str {
        "gitbooks_get_page"
    }

    fn description(&self) -> &str {
        "Fetch the full markdown of a specific OpenHuman documentation page by URL. Pair this \
        with `gitbooks_search` — search returns partial excerpts; use this to get the \
        complete page when more detail is needed."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The full URL of the OpenHuman documentation page (e.g. https://tinyhumans.gitbook.io/openhuman/getting-started)."
                }
            },
            "required": ["url"]
        })
    }

    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::ReadOnly
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let url = args
            .get("url")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("Missing 'url' parameter"))?;
        if url.trim().is_empty() {
            return Ok(ToolResult::error("url cannot be empty"));
        }

        tracing::debug!(
            target: "[gitbooks]",
            endpoint = %redact_endpoint(&self.endpoint),
            tool = "getPage",
            "dispatching via shared MCP client"
        );

        Ok(self
            .call("getPage", json!({ "url": url }), "gitbooks_get_page")
            .await)
    }
}

impl GitbooksSearchTool {
    async fn call(&self, tool: &str, arguments: Value, label: &str) -> ToolResult {
        configured_call(&self.client, &self.endpoint, self.timeout_secs, tool, arguments, label)
            .await
    }
}

impl GitbooksGetPageTool {
    async fn call(&self, tool: &str, arguments: Value, label: &str) -> ToolResult {
        configured_call(&self.client, &self.endpoint, self.timeout_secs, tool, arguments, label)
            .await
    }
}

async fn configured_call(
    client: &ModuleClient,
    endpoint: &str,
    timeout_secs: u64,
    tool: &str,
    arguments: Value,
    label: &str,
) -> ToolResult {
    let request = tinymcp_bus::ConfiguredToolCallRequest {
        server: tinymcp_bus::McpServerConfig {
            name: "openhuman-gitbooks".to_owned(),
            endpoint: endpoint.to_owned(),
            timeout_secs,
            ..Default::default()
        },
        tool: tool.to_owned(),
        arguments,
    };
    match client
        .call_confidential(
            "tinymcp",
            tinymcp_bus::names::methods::CONFIGURED_TOOL_CALL,
            (request,),
        )
        .await
    {
        Ok(call) if call.outcome.ok => call
            .rendered
            .map(tinymcp::tools::tool_result)
            .unwrap_or_else(|| ToolResult::error(format!("{label} failed: empty module result"))),
        Ok(call) => ToolResult::error(format!(
            "{label} failed: {}",
            call.error_message
                .unwrap_or_else(|| "MCP server call failed".to_owned())
        )),
        Err(error) => ToolResult::error(format!("{label} failed: {error}")),
    }
}

#[cfg(test)]
#[path = "gitbooks_tests.rs"]
mod tests;
