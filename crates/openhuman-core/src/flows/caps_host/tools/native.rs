//! Thin TinyFlows registration and error adapter for native OpenHuman tools.

use async_trait::async_trait;
use serde_json::Value;
use tinybox_bus::CommandClass;
use super::super::{EngineError, Result};

use super::{ToolBackend, ToolCallCtx, NATIVE_TOOL_PREFIX};

pub(crate) struct NativeToolBackend;

#[async_trait]
impl ToolBackend for NativeToolBackend {
    fn name(&self) -> &'static str {
        "native"
    }

    fn prefix(&self) -> &'static str {
        NATIVE_TOOL_PREFIX
    }

    async fn invoke(
        &self,
        ctx: &ToolCallCtx<'_>,
        slug: &str,
        args: Value,
        _conn: Option<&str>,
    ) -> Result<Value> {
        crate::tools::native_ops::flow_backend::invoke(
            ctx.config,
            slug,
            args,
            &NativeFlowPolicyHooks { ctx },
        )
        .await
        .map_err(EngineError::Capability)
    }
}

struct NativeFlowPolicyHooks<'a, 'ctx> {
    ctx: &'a ToolCallCtx<'ctx>,
}

#[async_trait]
impl crate::tools::native_ops::flow_backend::FlowPolicyHooks for NativeFlowPolicyHooks<'_, '_> {
    async fn resolve_policy(
        &self,
        config: &crate::config::Config,
    ) -> std::result::Result<std::sync::Arc<crate::security::SecurityPolicy>, String> {
        super::super::current_tier_policy(self.ctx.dispatch, config, self.ctx.security)
            .await
            .map(std::sync::Arc::new)
            .map_err(|error| match error {
                EngineError::Capability(body) => body,
                other => other.to_string(),
            })
    }

    async fn authorize(
        &self,
        policy: &crate::security::SecurityPolicy,
        class: CommandClass,
        tool_name: &str,
        args: &Value,
    ) -> std::result::Result<Option<String>, String> {
        let tier_decision = super::super::enforce_node_tier_gate(policy, class, "tool_call")
            .map_err(|error| match error {
                EngineError::Capability(body) => body,
                other => other.to_string(),
            })?;
        let summary = crate::security::approval::summarize_action(tool_name, args);
        let redacted = crate::security::approval::redact_args(args);
        let (outcome, audit_id) = super::super::gate_call_for_tier(
            self.ctx.dispatch,
            self.ctx.approval,
            tier_decision,
            tool_name,
            &summary,
            redacted,
        )
        .await;
        match outcome {
            crate::security::approval::GateOutcome::Deny { reason } => Err(reason),
            crate::security::approval::GateOutcome::Allow => Ok(audit_id),
        }
    }

    fn record_execution(
        &self,
        audit_id: Option<&str>,
        result: &std::result::Result<Value, String>,
    ) {
        let Some(id) = audit_id else {
            return;
        };
        let Some(gate) = crate::security::approval::ApprovalGate::try_global() else {
            return;
        };
        let execution = if result.is_ok() {
            crate::security::approval::ExecutionOutcome::Success
        } else {
            crate::security::approval::ExecutionOutcome::Failure
        };
        let error = result
            .as_ref()
            .err()
            .map(|body| EngineError::Capability(body.clone()).to_string());
        tracing::debug!(
            target: "flows",
            audit_id = %id,
            success = result.is_ok(),
            "[flows] recording native tool execution outcome on the approval audit trail"
        );
        gate.record_execution(id, execution, error.as_deref());
    }
}
