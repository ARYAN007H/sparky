//! Mixed Composio/HTTP connection picker adapter owned by flows.

use std::sync::Arc;
use async_trait::async_trait;
use serde_json::{json, Value};
use crate::config::Config;
use crate::flows::ops;
use tinytools::{PermissionLevel, Tool, ToolCallOptions, ToolResult, ToolRunContext};

// ─────────────────────────────────────────────────────────────────────────────
// list_flow_connections — read-only: connection refs (ids/names only)
// ─────────────────────────────────────────────────────────────────────────────

/// `list_flow_connections`: read-only enumeration of the connection sources a
/// node's `connection_ref` can attach to (Composio connected accounts +
/// named HTTP credentials) — non-secret metadata only (ids / display labels
/// / kind / toolkit / scheme / platform_user_id), never secrets.
pub struct ListFlowConnectionsTool {
    config: Arc<Config>,
}

impl ListFlowConnectionsTool {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Tool for ListFlowConnectionsTool {
    fn name(&self) -> &str {
        "list_flow_connections"
    }

    fn description(&self) -> &str {
        "List the connection sources a flow node's `connection_ref` can attach to: \
         Composio connected accounts and named HTTP credentials. Read-only; \
         returns only non-secret metadata — ids, display labels, kind, and \
         `toolkit`/`scheme` (never any secret). Each \
         Composio entry also carries `platform_user_id` — the connected \
         account's own member id (e.g. Slack `U123ABC`) — use it to wire a \
         self-targeted action like 'DM me' to that account instead of a \
         public channel. Use the `connection_ref` values verbatim on \
         tool_call / http_request nodes so the generated flow carries valid \
         connections."
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
        tracing::debug!(target: "flows", "[flows] list_flow_connections: enumerating connection refs (read-only)");
        match crate::flows::ops::flows_list_connections(&ctx, &self.config).await {
            Ok(outcome) => {
                let conns: Vec<Value> = outcome.value.iter().map(flow_connection_to_json).collect();
                Ok(ToolResult::success(serde_json::to_string_pretty(
                    &json!({ "connections": conns }),
                )?))
            }
            Err(e) => Ok(ToolResult::error(format!(
                "Failed to list flow connections: {e}"
            ))),
        }
    }
}

/// Render one [`tinyflows_catalog::types::FlowConnection`] as the
/// picker JSON shape the agent reads — ids/display/kind/toolkit/scheme plus
/// `platform_user_id` (the connected account's own member id, e.g. Slack
/// `U123ABC`, or `null` when no identity has synced yet). Never secret
/// material. A free function (rather than inline in `execute`) so the
/// mapping is unit-testable without a live Composio backend.
pub(crate) fn flow_connection_to_json(c: &tinyflows_catalog::types::FlowConnection) -> Value {
    json!({
        "connection_ref": c.connection_ref,
        "kind": c.kind,
        "display": c.display,
        "toolkit": c.toolkit,
        "scheme": c.scheme,
        "platform_user_id": c.platform_user_id,
    })
}
