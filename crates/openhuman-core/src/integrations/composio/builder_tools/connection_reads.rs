//! Read-only connection tools: connectable toolkits and connection refs (ids/names only).

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::config::Config;
use tinytools::{PermissionLevel, Tool, ToolCallOptions, ToolResult, ToolRunContext};

/// `list_connectable_toolkits`: read-only list of the Composio toolkits the
/// builder can wire, each tagged connected/unconnected — so the agent can steer
/// toolkit choice toward what's already connected (audit Phase 5, item 19).
pub struct ListConnectableToolkitsTool {
    config: Arc<Config>,
}

impl ListConnectableToolkitsTool {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Tool for ListConnectableToolkitsTool {
    fn name(&self) -> &str {
        "list_connectable_toolkits"
    }

    fn description(&self) -> &str {
        "List the Composio toolkits available to wire into a tool_call/app_event, each flagged \
         `connected: true/false`. Read-only. Use it to prefer an ALREADY-connected toolkit when \
         several would work, and to tell the user which toolkits a proposed flow still needs \
         connecting. Returns a JSON array of { toolkit, connected }."
    }

    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "properties": {}, "additionalProperties": false })
    }

    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::None
    }

    fn external_effect(&self) -> bool {
        false
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        self.execute_with_context(args, ToolCallOptions::default(), None)
            .await
    }

    async fn execute_with_context(
        &self,
        _args: Value,
        _options: ToolCallOptions,
        tool_context: Option<&dyn ToolRunContext>,
    ) -> anyhow::Result<ToolResult> {
        let ctx = crate::agent::tinyagents::host::run_context::dispatch_ctx_for_tool(tool_context);
        // The agent-ready list is the connector module's (`GetVocabulary`),
        // read through the host's curated snapshot.
        use crate::integrations::composio::contract::agent_ready_toolkits;
        tracing::debug!(target: "flows", "[flows] list_connectable_toolkits: listing toolkits + connected state (read-only) via the memory contract");
        let connected = crate::integrations::composio::flow_gate::connected_toolkit_slugs(&self.config)
            .await
            .unwrap_or_default()
            .into_iter()
            .collect::<std::collections::HashSet<_>>();
        let toolkits: Vec<Value> = agent_ready_toolkits()
            .into_iter()
            .map(|tk| {
                let tk_lc = tk.to_ascii_lowercase();
                json!({ "toolkit": tk_lc, "connected": connected.contains(&tk_lc) })
            })
            .collect();
        Ok(ToolResult::success(serde_json::to_string_pretty(
            &json!({ "toolkits": toolkits }),
        )?))
    }
}

