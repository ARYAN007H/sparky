use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tinyagents_harness_tools::exec::{ExecBackend, InstallTool};
use tinytools::{PermissionLevel, Tool, ToolCallOptions, ToolResult, ToolRunContext, ToolTimeout};

use crate::security::{AuditLogger, SecurityPolicy};

use super::exec_adapters::InstallGate;

/// OpenHuman host adapter for TinyAgents' generic package installer.
pub struct InstallToolTool {
    inner: InstallTool,
}

impl InstallToolTool {
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self::with_audit(security, AuditLogger::disabled())
    }

    pub fn with_audit(security: Arc<SecurityPolicy>, audit: Arc<AuditLogger>) -> Self {
        let gate = Arc::new(InstallGate { security, audit });
        let backend: Arc<dyn ExecBackend> =
            Arc::new(tinyagents_harness_tools::exec::NativeExecBackend::new(
                tinyagents_harness_tools::exec::ShellFlavor::Posix,
            ));
        Self {
            inner: InstallTool::new(gate, backend),
        }
    }
}

#[async_trait]
impl Tool for InstallToolTool {
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

    fn external_effect_with_args(&self, args: &Value) -> bool {
        self.inner.external_effect_with_args(args)
    }

    fn timeout_policy(&self, args: &Value) -> ToolTimeout {
        self.inner.timeout_policy(args)
    }

    async fn prepare_authorization(&self, args: &Value) -> anyhow::Result<Option<Value>> {
        self.inner.prepare_authorization(args).await
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        self.inner.execute(args).await
    }

    async fn execute_with_context(
        &self,
        args: Value,
        options: ToolCallOptions,
        context: Option<&dyn ToolRunContext>,
    ) -> anyhow::Result<ToolResult> {
        self.inner
            .execute_with_context(args, options, context)
            .await
    }
}

#[cfg(test)]
#[path = "install_tool_tests.rs"]
mod tests;
