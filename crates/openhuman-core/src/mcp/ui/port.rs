//! The server operations widget RPCs need, behind a seam.
//!
//! [`HostUiPort`] answers them through this workspace's TinyMCP module:
//! installed servers by their connection, configured servers by the static
//! set. The module contract has no `resources/read`, so a widget document is
//! only available when the tool result embedded it. Tests substitute their
//! own port.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::config::Config;
use crate::mcp::host;
use crate::mcp::module_client::McpConfiguredClient;

use super::types::UiToolDescriptor;

/// What a widget RPC asks of an MCP server.
#[async_trait]
pub trait UiServerPort: Send + Sync {
    /// Whether `server_id` is connected (installed) or configured (static).
    async fn is_available(&self, server_id: &str) -> bool;

    /// The live, scan-clean descriptor of `tool`, or `None` when the server
    /// does not offer it now.
    async fn tool_descriptor(&self, server_id: &str, tool: &str) -> Option<UiToolDescriptor>;

    /// A `resources/read` reply's `contents`.
    async fn read_resource(&self, server_id: &str, uri: &str) -> Result<Vec<Value>, String>;

    /// A `tools/call` reply, as the server sent it.
    async fn call_tool(
        &self,
        server_id: &str,
        tool: &str,
        arguments: Value,
    ) -> Result<Value, String>;
}

/// [`UiServerPort`] over `config`'s TinyMCP module.
pub struct HostUiPort {
    config: Arc<Config>,
}

impl HostUiPort {
    #[must_use]
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }

    fn client(&self) -> Option<Arc<McpConfiguredClient>> {
        McpConfiguredClient::for_config(&self.config).ok()
    }

    async fn is_static(&self, server_id: &str) -> bool {
        let Some(client) = self.client() else {
            return false;
        };
        let listed: Result<Vec<String>, _> = client
            .call(
                &host::client_config(&self.config),
                tinymcp_bus::names::methods::STATIC_LIST,
                (),
            )
            .await;
        listed.is_ok_and(|servers| servers.iter().any(|name| name == server_id))
    }

    async fn is_installed_connected(&self, server_id: &str) -> bool {
        let Some(client) = self.client() else {
            return false;
        };
        let statuses: Result<Vec<tinymcp_bus::ConnStatus>, _> = client
            .call(
                &host::client_config(&self.config),
                tinymcp_bus::names::methods::STATUS,
                (),
            )
            .await;
        statuses.is_ok_and(|statuses| {
            statuses.iter().any(|status| {
                status.server_id == server_id
                    && status.status == tinymcp_bus::ServerStatus::Connected
            })
        })
    }
}

#[async_trait]
impl UiServerPort for HostUiPort {
    async fn is_available(&self, server_id: &str) -> bool {
        self.is_installed_connected(server_id).await || self.is_static(server_id).await
    }

    async fn tool_descriptor(&self, server_id: &str, tool: &str) -> Option<UiToolDescriptor> {
        let client = self.client()?;
        let client_config = host::client_config(&self.config);
        if self.is_installed_connected(server_id).await {
            let live: Vec<tinymcp_bus::McpTool> = client
                .call(
                    &client_config,
                    tinymcp_bus::names::methods::LIST_TOOLS,
                    (server_id,),
                )
                .await
                .ok()?;
            let safe = crate::mcp::registry::tools_safe_for_agent(server_id, live);
            safe.iter().find(|candidate| candidate.name == tool)?;
            return Some(UiToolDescriptor {
                meta: None,
                annotations: None,
            });
        }
        let listed: Vec<tinymcp_bus::McpRemoteTool> = client
            .call(
                &client_config,
                tinymcp_bus::names::methods::STATIC_LIST_TOOLS,
                (server_id,),
            )
            .await
            .map_err(|error| {
                tracing::debug!(server_id, ?error, "[mcp_ui] could not list tools");
            })
            .ok()?;
        let listed = client
            .scrub_static_tools(&client_config, server_id, listed)
            .await
            .ok()?;
        let meta = listed
            .iter()
            .find(|remote| remote.name == tool)
            .and_then(|remote| remote.meta.clone());
        let safe = crate::mcp::registry::tools_safe_for_agent(
            server_id,
            listed
                .into_iter()
                .map(|remote| tinymcp_bus::McpTool {
                    description: remote.description.or(remote.title),
                    name: remote.name,
                    input_schema: remote.input_schema,
                })
                .collect(),
        );
        safe.iter().find(|candidate| candidate.name == tool)?;
        Some(UiToolDescriptor {
            meta,
            annotations: None,
        })
    }

    async fn read_resource(&self, server_id: &str, uri: &str) -> Result<Vec<Value>, String> {
        tracing::debug!(
            server_id,
            uri,
            "[mcp_ui] resources/read is not part of the TinyMCP module contract"
        );
        Err("this server's widget document is not embedded in its tool result".to_string())
    }

    async fn call_tool(
        &self,
        server_id: &str,
        tool: &str,
        arguments: Value,
    ) -> Result<Value, String> {
        let client = self.client().ok_or("MCP module is unavailable")?;
        let client_config = host::client_config(&self.config);
        let member = if self.is_installed_connected(server_id).await {
            tinymcp_bus::names::methods::TOOL_CALL
        } else {
            tinymcp_bus::names::methods::STATIC_CALL_TOOL
        };
        let outcome: tinymcp_bus::ToolCallOutcome = client
            .call_confidential(&client_config, member, (server_id, tool, arguments))
            .await
            .map_err(|error| {
                tracing::debug!(server_id, tool, ?error, "[mcp_ui] widget tool call failed");
                "MCP tool call failed".to_string()
            })?;
        Ok(outcome.result)
    }
}

#[cfg(test)]
#[path = "port_tests.rs"]
mod tests;
