//! OpenHuman policy adapters for TinyAgents' generic execution tools.

use std::{path::PathBuf, sync::Arc, time::Duration};

use async_trait::async_trait;
use serde_json::Value;
use tinyagents_harness_tools::exec::{
    ExecAuditEvent, ExecAuditOutcome, ExecAuthorizationRequest, ExecBackend, ExecBackendCapability,
    ExecBackendOutcome, ExecDenied, ExecGate, ExecOperation, ExecOutput, ExecPermit,
    ExecRefusalKind, ExecRequest, ExecTool, NativeExecCapability, ShellFlavor,
};
use tinytools::{PermissionLevel, Tool, ToolCallOptions, ToolResult, ToolRunContext, ToolTimeout};

use crate::{
    agent::tinyagents::host::run_context::{
        approval_for_tool, command_environment_for_tool, dispatch_ctx_for_tool,
    },
    config::Config,
    core::runtime::DispatchCtx,
    modules::tinybox::host_runtime::RuntimeAdapter,
    security::{AuditLogger, CommandExecutionLog, SecurityPolicy},
};

use super::shell::ShellHost;

/// Host-owned gate for package installation. The generic TinyAgents tool owns
/// argument validation and process-result formatting; OpenHuman owns whether
/// a host install is permitted.
pub(super) struct InstallGate {
    pub security: Arc<SecurityPolicy>,
    pub audit: Arc<AuditLogger>,
    pub environment: std::collections::BTreeMap<std::ffi::OsString, std::ffi::OsString>,
}

#[async_trait]
impl ExecGate for InstallGate {
    async fn prepare_authorization(&self, _args: &Value) -> anyhow::Result<Option<Value>> {
        Ok(None)
    }

    fn external_effect_with_args(&self, _args: &Value) -> bool {
        true
    }

    async fn authorize(
        &self,
        request: &ExecAuthorizationRequest,
        context: Option<&dyn ToolRunContext>,
    ) -> Result<ExecPermit, ExecDenied> {
        let ExecOperation::Install { .. } = &request.operation else {
            return Err(ExecDenied(
                "install gate received a non-install operation".into(),
            ));
        };
        if crate::core::runtime::is_saas() {
            return Err(ExecDenied(
                "[policy-denied] Package installation is unavailable in SaaS mode.".into(),
            ));
        }
        if !approval_for_tool(context).is_interactive() {
            return Err(ExecDenied(
                "[policy-denied] install_tool requires interactive approval and is not available in autonomous (background) turns.".into(),
            ));
        }
        if !self.security.allow_tool_install {
            return Err(ExecDenied(
                "[policy-denied] OS package installation is disabled. Enable it in the agent access settings (Full access mode / allow_tool_install) before installing tools.".into(),
            ));
        }
        if !self.security.can_act() {
            return Err(ExecDenied(
                "[policy-blocked] Action blocked: autonomy is read-only.".into(),
            ));
        }
        if self.security.is_rate_limited() {
            return Err(ExecDenied(
                "Rate limit exceeded: too many actions in the last hour".into(),
            ));
        }
        if !self.security.record_action() {
            return Err(ExecDenied(
                "Rate limit exceeded: action budget exhausted".into(),
            ));
        }
        Ok(ExecPermit {
            action_dir: PathBuf::from(&self.security.workspace_dir),
            environment: self.environment.clone(),
            backend_selection: "native".into(),
            backend_capability: ExecBackendCapability::new(NativeExecCapability),
        })
    }

    fn record(&self, event: ExecAuditEvent) {
        let (command, risk_level) = match &event.operation {
            ExecOperation::Install { package, manager } => {
                (format!("{manager} install {package}"), "install")
            }
            ExecOperation::Shell { command, category } => {
                (command.clone(), category.as_deref().unwrap_or("unknown"))
            }
        };
        let success = matches!(
            event.outcome,
            ExecAuditOutcome::Completed { success: true, .. }
        );
        if let Err(error) = self.audit.log_command_event(CommandExecutionLog {
            channel: "tool:install_tool",
            command: &command,
            risk_level,
            approved: event.approval_required,
            allowed: event.allowed,
            success,
            duration_ms: u64::try_from(event.duration.as_millis()).unwrap_or(u64::MAX),
        }) {
            tracing::warn!(error = %error, "[install_tool] failed to persist execution audit event");
        }
        if let ExecAuditOutcome::BackendRefused {
            kind: ExecRefusalKind::BackendUnavailable,
        } = event.outcome
        {
            tracing::warn!("[install_tool] selected execution backend is unavailable");
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct HostShellCapability;

/// Host-owned implementation of TinyAgents' execution ports. OpenHuman keeps
/// its policy and runtime selection here; the upstream tool owns generic shell
/// metadata and invocation behavior.
struct ShellPorts {
    host: Arc<ShellHost>,
}

#[async_trait]
impl ExecGate for ShellPorts {
    async fn prepare_authorization(&self, args: &Value) -> anyhow::Result<Option<Value>> {
        self.host.prepare_authorization(args).await
    }

    fn external_effect_with_args(&self, args: &Value) -> bool {
        self.host.external_effect_with_args(args)
    }

    async fn authorize(
        &self,
        request: &ExecAuthorizationRequest,
        context: Option<&dyn ToolRunContext>,
    ) -> Result<ExecPermit, ExecDenied> {
        let ExecOperation::Shell { command, .. } = &request.operation else {
            return Err(ExecDenied(
                "shell gate received a non-shell operation".into(),
            ));
        };

        // Recheck facts before issuing a permit. The backend repeats this
        // module analysis immediately before launch to close the approval race.
        #[cfg(feature = "modules")]
        {
            let Some(config) = self.host.analysis_config.as_deref() else {
                #[cfg(test)]
                self.host
                    .security
                    .check_gated_command(command)
                    .map_err(ExecDenied)?;
                #[cfg(not(test))]
                return Err(ExecDenied("Command analysis is unavailable.".into()));
                #[cfg(test)]
                return self.permit(context);
            };
            let analysis = crate::modules::tinybox::analyze_command(config, command.clone())
                .await
                .map_err(|_| ExecDenied("Command analysis is unavailable.".into()))?;
            self.host
                .security
                .check_gated_command_analysis(&analysis)
                .map_err(ExecDenied)?;
        }
        #[cfg(not(feature = "modules"))]
        {
            #[cfg(test)]
            self.host
                .security
                .check_gated_command(command)
                .map_err(ExecDenied)?;
            #[cfg(not(test))]
            return Err(ExecDenied("Command analysis is unavailable.".into()));
        }
        self.permit(context)
    }

    fn record(&self, event: ExecAuditEvent) {
        let (command, risk_level) = match &event.operation {
            ExecOperation::Shell { command, .. } => (command.as_str(), "unknown"),
            ExecOperation::Install { .. } => ("<invalid shell operation>", "unknown"),
        };
        let success = matches!(
            event.outcome,
            ExecAuditOutcome::Completed { success: true, .. }
        );
        if let Err(error) = self.host.audit.log_command_event(CommandExecutionLog {
            channel: "tool:shell",
            command,
            risk_level,
            approved: event.approval_required,
            allowed: event.allowed,
            success,
            duration_ms: u64::try_from(event.duration.as_millis()).unwrap_or(u64::MAX),
        }) {
            tracing::warn!(error = %error, channel = "tool:shell", "failed to persist shell audit event");
        }
    }
}

impl ShellPorts {
    fn permit(&self, context: Option<&dyn ToolRunContext>) -> Result<ExecPermit, ExecDenied> {
        let action_dir = self.host.effective_action_dir_for_context(context);
        let environment = command_environment_for_tool(context)
            .as_deref()
            .into_iter()
            .flat_map(|environment| environment.iter())
            .map(|(name, value)| (name.into(), value.into()))
            .collect();
        Ok(ExecPermit {
            action_dir,
            environment,
            backend_selection: "openhuman-runtime".into(),
            backend_capability: ExecBackendCapability::new(HostShellCapability),
        })
    }
}

#[async_trait]
impl ExecBackend for ShellPorts {
    fn shell_flavor(&self) -> ShellFlavor {
        match self.host.runtime.shell_flavor() {
            crate::modules::tinybox::host_runtime::ShellFlavor::Cmd => ShellFlavor::Cmd,
            _ => ShellFlavor::Posix,
        }
    }

    async fn execute(
        &self,
        request: ExecRequest,
        context: Option<&dyn ToolRunContext>,
        _progress: Option<tinyagents_harness_tools::exec::ExecProgress>,
    ) -> ExecBackendOutcome {
        if request.backend_selection != "openhuman-runtime"
            || request
                .backend_capability
                .downcast_ref::<HostShellCapability>()
                .is_none()
        {
            return ExecBackendOutcome::Refused {
                kind: ExecRefusalKind::Preflight,
                reason: "OpenHuman runtime was not selected or authorized".into(),
            };
        }
        let Some(command) = request.shell_command else {
            return ExecBackendOutcome::Refused {
                kind: ExecRefusalKind::Preflight,
                reason: "shell command is missing".into(),
            };
        };
        let timeout_secs = request.timeout.map(|timeout| timeout.as_secs());
        let (allowed, result) = crate::sandbox::shell_progress::with_shell_progress(
            context,
            crate::sandbox::shell_progress::HEARTBEAT_INTERVAL,
            |meter| async {
                self.host
                    .run_with_security_in_context(
                        &dispatch_ctx_for_tool(context),
                        &command,
                        timeout_secs,
                        context,
                        meter,
                    )
                    .await
            },
        )
        .await;
        if !allowed {
            return ExecBackendOutcome::Refused {
                kind: ExecRefusalKind::Preflight,
                reason: result.output(),
            };
        }
        ExecBackendOutcome::Completed(ExecOutput {
            stdout: result.output().into_bytes(),
            stderr: Vec::new(),
            exit_code: Some(if result.is_error { 1 } else { 0 }),
            timed_out: false,
        })
    }
}

/// Public OpenHuman adapter around the upstream generic `ExecTool`.
pub struct ShellTool {
    inner: ExecTool,
    host: Arc<ShellHost>,
}

impl ShellTool {
    pub fn new(
        security: Arc<SecurityPolicy>,
        runtime: Arc<dyn RuntimeAdapter>,
        audit: Arc<AuditLogger>,
    ) -> Self {
        Self::from_host(Arc::new(ShellHost::new(security, runtime, audit)))
    }

    pub fn with_config(
        security: Arc<SecurityPolicy>,
        runtime: Arc<dyn RuntimeAdapter>,
        audit: Arc<AuditLogger>,
        config: Arc<Config>,
    ) -> Self {
        Self::from_host(Arc::new(ShellHost::with_config(
            security, runtime, audit, config,
        )))
    }

    fn from_host(host: Arc<ShellHost>) -> Self {
        let ports = Arc::new(ShellPorts {
            host: Arc::clone(&host),
        });
        let gate: Arc<dyn ExecGate> = ports.clone();
        let backend: Arc<dyn ExecBackend> = ports;
        Self {
            inner: ExecTool::new(gate, backend),
            host,
        }
    }

    pub(super) fn explicit_timeout(&self, requested: Option<u64>) -> Option<Duration> {
        self.host.explicit_timeout(requested)
    }

    /// Workflow commands share the same host execution policy.
    pub(crate) async fn run_with_security(
        &self,
        ctx: &DispatchCtx,
        command: &str,
        requested_timeout: Option<u64>,
    ) -> (bool, ToolResult) {
        self.host
            .run_with_security(ctx, command, requested_timeout)
            .await
    }
}

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }
    fn description(&self) -> &str {
        self.inner.description()
    }
    fn parameters_schema(&self) -> Value {
        self.inner.parameters_schema()
    }
    fn max_result_size_chars(&self) -> Option<usize> {
        self.inner.max_result_size_chars()
    }
    fn timeout_policy(&self, args: &Value) -> ToolTimeout {
        self.inner.timeout_policy(args)
    }
    fn permission_level(&self) -> PermissionLevel {
        self.inner.permission_level()
    }
    fn external_effect_with_args(&self, args: &Value) -> bool {
        self.inner.external_effect_with_args(args)
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
