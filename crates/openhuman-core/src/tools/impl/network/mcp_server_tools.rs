//! Every tool on a configured (`[[mcp_client.servers]]` / `AgentSpec::mcp`)
//! server as its own agent tool.
//!
//! The generic bridge (`mcp_list_tools` + `mcp_call_tool`) makes a model
//! discover a server's tools and route each call through one proxy. These
//! tools are the direct form: `mcp_<server>_<tool>`, with the remote schema,
//! deferred by default so `tool_search` finds them, or sent every turn when
//! the server says `expose = "direct"` / lists them in `direct_tools`.
//!
//! They are built from tinymcp's persistent tool cache, never from the
//! network, so assembling a tool list costs no round trip. A server with
//! nothing cached yet is listed in the background and its tools appear on the
//! next build; until then the generic bridge still reaches it.
//!
//! Policy stays here: the act gate, the prompt-injection scan on remote
//! definitions, and the secret scrubber on output, exactly as `mcp_call_tool`
//! applies them.

use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tinymcp::tools::{McpExposure, McpServerTool, McpToolInvoker, McpToolSource, SecretScrubber};
use tinymcp::McpRemoteToolExt;
use tinytools::{PermissionLevel, Tool, ToolCategory, ToolExposure, ToolResult};

use crate::config::{Config, McpToolExposure};
use crate::mcp::config_servers::McpServerRegistry;
use crate::security::{SecurityPolicy, ToolOperation};

/// Builds host adapters from TinyMCP-prepared declarations. Catalog retrieval,
/// naming, schema normalization, and execution remain module-owned.
pub fn configured_server_tools_from_presentations(
    config: &Config,
    presentations: Vec<tinymcp_bus::McpToolPresentation>,
    security: &Arc<SecurityPolicy>,
) -> Vec<Box<dyn Tool>> {
    presentations
        .into_iter()
        .filter_map(|presentation| {
            let server = config
                .mcp_client
                .servers
                .iter()
                .find(|server| server.server.name == presentation.server_id)?;
            Some(Box::new(ModuleConfiguredMcpServerTool {
                presentation,
                config: Arc::new(config.clone()),
                server: server.server.clone(),
                security: Arc::clone(security),
            }) as Box<dyn Tool>)
        })
        .collect()
}

struct ModuleConfiguredMcpServerTool {
    presentation: tinymcp_bus::McpToolPresentation,
    config: Arc<Config>,
    server: tinymcp_bus::McpServerConfig,
    security: Arc<SecurityPolicy>,
}

#[async_trait]
impl Tool for ModuleConfiguredMcpServerTool {
    fn name(&self) -> &str {
        &self.presentation.name
    }

    fn description(&self) -> &str {
        &self.presentation.description
    }

    fn parameters_schema(&self) -> Value {
        self.presentation.parameters.clone()
    }

    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::Execute
    }

    fn external_effect(&self) -> bool {
        true
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Workflow
    }

    fn exposure(&self) -> ToolExposure {
        if self.presentation.deferred {
            ToolExposure::Deferred
        } else {
            ToolExposure::Direct
        }
    }

    fn family(&self) -> Option<&str> {
        Some(&self.presentation.family)
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        self.security
            .enforce_tool_operation(ToolOperation::Act, self.name())
            .map_err(|error| anyhow::anyhow!(error))?;
        let current = crate::config::ops::reload_config_from_paths(
            &self.config.config_path,
            &self.config.workspace_dir,
        )
        .await
        .map_err(|_| anyhow::anyhow!("could not reload MCP configuration"))?;
        let current_server = current
            .mcp_client
            .servers
            .iter()
            .find(|server| server.server.name == self.presentation.server_id)
            .map(|server| &server.server)
            .ok_or_else(|| anyhow::anyhow!("MCP server is no longer configured"))?;
        if current_server != &self.server {
            anyhow::bail!("MCP server configuration changed; rebuild tools before calling");
        }

        let client = crate::mcp::module_client::McpConfiguredClient::for_config(&self.config)
            .map_err(|_| anyhow::anyhow!("TinyMCP module is unavailable"))?;
        let client_config = crate::mcp::host::client_config(&self.config);
        let live: Vec<tinymcp_bus::McpRemoteTool> = client
            .call(
                &client_config,
                tinymcp_bus::names::methods::STATIC_LIST_TOOLS,
                (self.presentation.server_id.as_str(),),
            )
            .await
            .map_err(|_| anyhow::anyhow!("MCP tool is unavailable"))?;
        let live = client
            .scrub_static_tools(&client_config, &self.presentation.server_id, live)
            .await
            .map_err(|_| anyhow::anyhow!("MCP tool metadata is unavailable"))?;
        let safe = crate::mcp::registry::tools_safe_for_agent(
            &self.presentation.server_id,
            live.into_iter()
                .map(|tool| tinymcp_bus::McpTool {
                    name: tool.name,
                    description: tool.description.or(tool.title),
                    input_schema: tool.input_schema,
                })
                .collect(),
        );
        if !safe
            .iter()
            .any(|tool| tool.name == self.presentation.remote_name)
        {
            anyhow::bail!("MCP tool is no longer available or safe");
        }
        let arguments = client
            .normalize_tool_arguments(&client_config, args)
            .await
            .map_err(|_| anyhow::anyhow!("invalid MCP tool arguments"))?;
        let call: tinymcp_bus::ClassifiedStaticToolCall = client
            .call_confidential(
                &client_config,
                tinymcp_bus::names::methods::STATIC_CALL_TOOL_CLASSIFIED,
                (
                    self.presentation.server_id.as_str(),
                    self.presentation.remote_name.as_str(),
                    Value::Object(arguments),
                ),
            )
            .await
            .map_err(|_| anyhow::anyhow!("MCP tool call failed"))?;
        Ok(super::mcp::classified_result(
            call,
            &self.presentation.server_id,
            &self.presentation.remote_name,
            false,
        ))
    }
}

/// One tool per cached tool of every server in `registry`, for `config`.
///
/// `reserved` names are never produced — the generic bridge tools and
/// anything else already registered keep their names.
pub fn configured_server_tools(
    config: &Config,
    registry: &Arc<McpServerRegistry>,
    security: &Arc<SecurityPolicy>,
    reserved: &HashSet<String>,
) -> Vec<Box<dyn Tool>> {
    let host = match crate::mcp::host::for_config(config) {
        Ok(host) => host,
        Err(error) => {
            tracing::debug!(
                ?error,
                "[mcp_client] no host for workspace; no server tools"
            );
            return Vec::new();
        }
    };

    let mut sources = Vec::new();
    let mut uncached = Vec::new();
    for definition in registry.list() {
        match registry.cached_tools(&definition.name, host.dynamic().store()) {
            Some(tools) => {
                let tools = crate::mcp::registry::tools_safe_for_agent(&definition.name, tools);
                sources.push(
                    McpToolSource::from_definition(definition, tools)
                        .with_exposure(exposure_for(config, &definition.name)),
                );
            }
            None => uncached.push(definition.name.clone()),
        }
    }
    if !uncached.is_empty() {
        warm_in_background(Arc::clone(&host), Arc::clone(registry), uncached);
    }

    let invoker: Arc<dyn McpToolInvoker> = Arc::clone(registry) as Arc<dyn McpToolInvoker>;
    let tools: Vec<Box<dyn Tool>> = tinymcp::tools::tools_for(&sources, &invoker)
        .into_iter()
        .filter(|tool| !reserved.contains(tool.name()))
        .map(|tool| {
            Box::new(ConfiguredMcpServerTool::new(
                tool,
                Arc::clone(registry),
                Arc::clone(security),
                config.config_path.clone(),
                config.workspace_dir.clone(),
            )) as Box<dyn Tool>
        })
        .collect();
    tracing::debug!(
        servers = sources.len(),
        tools = tools.len(),
        "[mcp_client] registered per-tool MCP server tools"
    );
    tools
}

/// The exposure `config` asks for on the server named `server`.
fn exposure_for(config: &Config, server: &str) -> McpExposure {
    let Some(declared) = config
        .mcp_client
        .servers
        .iter()
        .find(|candidate| candidate.name.trim() == server)
    else {
        return McpExposure::deferred();
    };
    McpExposure {
        default: match declared.expose {
            McpToolExposure::Direct => ToolExposure::Direct,
            McpToolExposure::Deferred => ToolExposure::Deferred,
        },
        direct_tools: declared.direct_tools.clone(),
    }
}

/// Lists and caches `servers` off the tool-assembly path.
fn warm_in_background(
    host: Arc<crate::mcp::host::McpHost>,
    registry: Arc<McpServerRegistry>,
    servers: Vec<String>,
) {
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        tracing::debug!("[mcp_client] no runtime; leaving the tool cache cold");
        return;
    };
    handle.spawn(async move {
        for server in servers {
            match registry
                .list_tools_caching(&server, host.dynamic().store())
                .await
            {
                Ok(tools) => tracing::debug!(
                    server = %server,
                    tools = tools.len(),
                    "[mcp_client] warmed the tool cache"
                ),
                Err(error) => tracing::debug!(
                    server = %server,
                    "[mcp_client] could not warm the tool cache: {error}"
                ),
            }
        }
    });
}

/// A configured server's tool under this application's call policy.
pub struct ConfiguredMcpServerTool {
    inner: McpServerTool,
    registry: Arc<McpServerRegistry>,
    security: Arc<SecurityPolicy>,
    scrubber: SecretScrubber,
    config_path: std::path::PathBuf,
    workspace_dir: std::path::PathBuf,
}

impl ConfiguredMcpServerTool {
    fn new(
        inner: McpServerTool,
        registry: Arc<McpServerRegistry>,
        security: Arc<SecurityPolicy>,
        config_path: std::path::PathBuf,
        workspace_dir: std::path::PathBuf,
    ) -> Self {
        let scrubber = SecretScrubber::for_server(&registry, inner.server_id());
        Self {
            inner,
            registry,
            security,
            scrubber,
            config_path,
            workspace_dir,
        }
    }
}

#[async_trait]
impl Tool for ConfiguredMcpServerTool {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    fn parameters_schema(&self) -> Value {
        self.inner.parameters_schema()
    }

    fn permission_level(&self) -> PermissionLevel {
        self.inner.permission_level()
    }

    fn external_effect(&self) -> bool {
        self.inner.external_effect()
    }

    fn category(&self) -> ToolCategory {
        self.inner.category()
    }

    fn exposure(&self) -> ToolExposure {
        self.inner.exposure()
    }

    fn family(&self) -> Option<&str> {
        self.inner.family()
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        self.security
            .enforce_tool_operation(ToolOperation::Act, self.name())
            .map_err(|err| anyhow::anyhow!(err))?;
        let server = self.inner.server_id();
        let tool = self.inner.remote_name();
        let config =
            crate::config::ops::reload_config_from_paths(&self.config_path, &self.workspace_dir)
                .await
                .map_err(|error| anyhow::anyhow!("could not reload MCP configuration: {error}"))?;
        let current_registry = crate::mcp::host::static_registry(&config);
        let Some(current_definition) = current_registry.get(server) else {
            anyhow::bail!("MCP server is no longer configured: {server}");
        };
        let Some(captured_definition) = self.registry.get(server) else {
            anyhow::bail!("MCP server is no longer configured: {server}");
        };
        if current_definition.fingerprint() != captured_definition.fingerprint() {
            anyhow::bail!(
                "MCP server configuration changed; rebuild tools before calling {server}"
            );
        }
        let live = current_registry.list_tools(server).await?;
        let safe = crate::mcp::registry::tools_safe_for_agent(
            server,
            live.into_iter()
                .map(|remote| {
                    let description = remote.display_description();
                    tinymcp_bus::McpTool {
                        name: remote.name,
                        description,
                        input_schema: remote.input_schema,
                    }
                })
                .collect(),
        );
        if !safe.iter().any(|candidate| candidate.name == tool) {
            anyhow::bail!("MCP tool is no longer available or safe: {server}/{tool}");
        }
        let result = self
            .inner
            .execute(args)
            .await
            .map_err(|error| anyhow::anyhow!(self.scrubber.scrub_error(&error)))?;
        Ok(self.scrubber.scrub_result(result))
    }
}

#[cfg(test)]
#[path = "mcp_server_tools_tests.rs"]
mod tests;
