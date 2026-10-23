#[cfg(feature = "modules")]
use super::shell_support::container_environment;
use super::shell_support::{command_failure, exec_result, tool_timeout_message};
use crate::agent::tinyagents::host::run_context::command_environment_for_tool;
use crate::config::Config;
use crate::core::runtime::DispatchCtx;
use crate::modules::tinybox::host_runtime::RuntimeAdapter;
use crate::security::{AuditLogger, SecurityPolicy};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
#[cfg(test)]
use tinytools::Tool;
use tinytools::ToolResult;
use tinytools::ToolRunContext;

/// Maximum output size in bytes (1MB).
const MAX_OUTPUT_BYTES: usize = 1_048_576;
#[cfg(feature = "modules")]
fn docker_module_result(
    output: crate::modules::tinybox::contract::DockerRunOutput,
    explicit_timeout: Option<Duration>,
) -> ToolResult {
    let mut stdout = output.stdout;
    let mut stderr = output.stderr;
    if stdout.len() > MAX_OUTPUT_BYTES {
        stdout.truncate(crate::util::floor_char_boundary(&stdout, MAX_OUTPUT_BYTES));
        stdout.push_str("\n... [output truncated at 1MB]");
    }
    if stderr.len() > MAX_OUTPUT_BYTES {
        stderr.truncate(crate::util::floor_char_boundary(&stderr, MAX_OUTPUT_BYTES));
        stderr.push_str("\n... [stderr truncated at 1MB]");
    }

    if output.timed_out {
        let secs = explicit_timeout
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        let mut message = tool_timeout_message(secs);
        if !stdout.is_empty() {
            message.push_str("\n[stdout]\n");
            message.push_str(&stdout);
        }
        if !stderr.is_empty() {
            message.push_str("\n[stderr]\n");
            message.push_str(&stderr);
        }
        ToolResult::error(message)
    } else if output.exit_code == 0 {
        if stderr.is_empty() {
            ToolResult::success(stdout)
        } else {
            ToolResult::success(format!("{stdout}\n[stderr]\n{stderr}"))
        }
    } else {
        command_failure(Some(output.exit_code), &stdout, &stderr)
    }
}

/// Shell command execution tool with sandboxing
pub(super) struct ShellHost {
    pub(super) security: Arc<SecurityPolicy>,
    pub(super) runtime: Arc<dyn RuntimeAdapter>,
    pub(super) audit: Arc<AuditLogger>,
    pub(super) analysis_config: Option<Arc<Config>>,
}

impl ShellHost {
    pub(super) fn new(
        security: Arc<SecurityPolicy>,
        runtime: Arc<dyn RuntimeAdapter>,
        audit: Arc<AuditLogger>,
    ) -> Self {
        Self {
            security,
            runtime,
            audit,
            analysis_config: None,
        }
    }

    /// Builds a shell tool whose command facts come from the configured
    /// TinyBox module before host authorization and execution.
    pub(super) fn with_config(
        security: Arc<SecurityPolicy>,
        runtime: Arc<dyn RuntimeAdapter>,
        audit: Arc<AuditLogger>,
        config: Arc<Config>,
    ) -> Self {
        Self {
            security,
            runtime,
            audit,
            analysis_config: Some(config),
        }
    }

    /// Resolve the working directory for this shell invocation.
    ///
    /// Returns the root from a [`WorkspaceDescriptor`] threaded into this
    /// call's [`ToolExecutionContext`](tinyagents_harness::tool::ToolExecutionContext).
    /// Otherwise falls back to the shared `self.security.action_dir`.
    /// See #3376, #4249 (08.5).
    pub(super) fn effective_action_dir_for_context(
        &self,
        context: Option<&dyn ToolRunContext>,
    ) -> PathBuf {
        if let Some(workspace) = context.and_then(|ctx| ctx.workspace()) {
            tracing::debug!(
                workspace_root = %workspace.root.display(),
                policy_id = %workspace.policy_id,
                "[shell] using TinyAgents workspace descriptor as action dir"
            );
            return workspace.root.clone();
        }
        self.security.action_dir.clone()
    }

    /// The explicit wall-clock budget for this invocation, or `None` to run
    /// unbounded.
    ///
    /// Shell commands run scripts — builds, test suites, solvers — that
    /// legitimately take minutes, so there is **no** default timeout: a
    /// deadline applies only when the caller passes `timeout_secs` (issue
    /// #4023). A `0` explicitly disables it. Any positive value is clamped to
    /// `1..=3600`. See
    /// [`tinyagents_harness_tools::exec::timeout::explicit_call_timeout_duration`].
    pub(super) fn explicit_timeout(&self, requested: Option<u64>) -> Option<Duration> {
        tinyagents_harness_tools::exec::timeout::explicit_call_timeout_duration(
            requested,
            tinyagents_harness_tools::exec::timeout::MAX_TIMEOUT_SECS,
        )
    }
}

impl ShellHost {
    pub(super) fn external_effect_with_args(&self, args: &Value) -> bool {
        args.get("command")
            .and_then(Value::as_str)
            .is_some_and(|command| !command.is_empty())
    }

    pub(super) async fn prepare_authorization(
        &self,
        args: &Value,
    ) -> anyhow::Result<Option<Value>> {
        #[cfg(feature = "modules")]
        {
            let config = self
                .analysis_config
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("command analysis is unavailable"))?;
            let command = args
                .get("command")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| anyhow::anyhow!("command analysis is unavailable"))?;
            let analysis = crate::modules::tinybox::analyze_command(config, command.to_owned())
                .await
                .map_err(|_| anyhow::anyhow!("command analysis is unavailable"))?;
            let analysis = serde_json::to_value(analysis)
                .map_err(|_| anyhow::anyhow!("command analysis is unavailable"))?;
            Ok(Some(json!({ "analysis": analysis })))
        }
        #[cfg(not(feature = "modules"))]
        {
            let _ = args;
            Err(anyhow::anyhow!("command analysis is unavailable"))
        }
    }
}

impl ShellHost {
    /// Run the command through the security policy and runtime. Returns
    /// `(allowed, result)` where `allowed=false` means the policy or rate
    /// limiter blocked execution before the command was launched.
    ///
    /// Exposed as `pub(crate)` so workflow phase scripts can reuse the
    /// same gated execution path as the `shell` tool — all security
    /// checks (rate limits, path guards, approval gate routing) apply
    /// identically to workflow-triggered commands.
    pub(super) async fn run_with_security(
        &self,
        ctx: &DispatchCtx,
        command: &str,
        requested_timeout: Option<u64>,
    ) -> (bool, ToolResult) {
        self.run_with_security_in_context(ctx, command, requested_timeout, None, None)
            .await
    }

    pub(super) async fn run_with_security_in_context(
        &self,
        ctx: &DispatchCtx,
        command: &str,
        requested_timeout: Option<u64>,
        context: Option<&dyn ToolRunContext>,
        meter: Option<crate::sandbox::shell_progress::ShellOutputMeter>,
    ) -> (bool, ToolResult) {
        let command_environment = command_environment_for_tool(context);
        // Revalidate facts immediately before execution. The authorization
        // hook supplies the same module-owned parser to the harness gate; this
        // second call keeps direct/workflow execution on the same contract and
        // fails closed if the module disappeared after approval.
        #[cfg(feature = "modules")]
        if let Some(config) = self.analysis_config.as_deref() {
            let analysis =
                match crate::modules::tinybox::analyze_command(config, command.to_owned()).await {
                    Ok(analysis) => analysis,
                    Err(_) => {
                        return (false, ToolResult::error("Command analysis is unavailable."));
                    }
                };
            if let Err(reason) = self.security.check_gated_command_analysis(&analysis) {
                return (false, ToolResult::error(reason));
            }
        } else {
            #[cfg(test)]
            if let Err(reason) = self.security.check_gated_command(command) {
                // Unit tests exercise the standalone policy helper without
                // booting TinyBus. Production constructors always carry the
                // module config and never use this compatibility path.
                return (false, ToolResult::error(reason));
            }
            #[cfg(not(test))]
            return (false, ToolResult::error("Command analysis is unavailable."));
        }
        #[cfg(not(feature = "modules"))]
        {
            let _ = &self.analysis_config;
            let _ = command;
            #[cfg(test)]
            if let Err(reason) = self.security.check_gated_command(command) {
                return (false, ToolResult::error(reason));
            }
            #[cfg(not(test))]
            return (false, ToolResult::error("Command analysis is unavailable."));
        }

        let _cwd = self.effective_action_dir_for_context(context);

        if self.security.is_rate_limited() {
            return (
                false,
                ToolResult::error("Rate limit exceeded: too many actions in the last hour"),
            );
        }

        if !self.security.record_action() {
            return (
                false,
                ToolResult::error("Rate limit exceeded: action budget exhausted"),
            );
        }

        // SaaS: every command runs in the user's container, whatever the
        // agent's sandbox mode says, and never on the host.
        let saas_action_dir = self.effective_action_dir_for_context(context);
        if let Some(resolved) =
            super::shell_saas::saas_sandbox_with(crate::core::runtime::is_saas(), || {
                crate::profiles::tools::sandbox_policy(
                    &saas_action_dir,
                    &self.security.workspace_dir,
                )
            })
        {
            let action_dir = saas_action_dir;
            return match resolved {
                Ok(policy) => {
                    self.run_in_policy(
                        ctx,
                        policy,
                        command,
                        requested_timeout,
                        &action_dir,
                        context,
                        meter,
                    )
                    .await
                }
                Err(why) => {
                    tracing::warn!(reason = %why, "[shell] SaaS sandbox refused the command");
                    // Nothing ran: report it as not allowed.
                    super::shell_saas::saas_sandbox_refusal(&why)
                }
            };
        }

        // A sandboxed agent or an explicit operator backend uses the sandbox
        // execution path. Security checks above still apply.
        let sandbox_required = match crate::sandbox::command_requires_sandbox(
            ctx,
            crate::agent::tinyagents::host::run_context::sandbox_mode_for_tool(context),
        )
        .await
        {
            Ok(required) => required,
            Err(err) => {
                return (
                    false,
                    ToolResult::error(format!("Cannot read sandbox configuration: {err}")),
                );
            }
        };
        if sandbox_required {
            let action_dir = self.effective_action_dir_for_context(context);
            return self
                .run_sandboxed(ctx, command, requested_timeout, &action_dir, context, meter)
                .await;
        }

        // Clear the environment to prevent leaking API keys and other secrets
        // (CWE-200), then re-add only safe, functional variables.
        let action_dir = self.effective_action_dir_for_context(context);
        #[cfg(feature = "modules")]
        if let Some((mut request, mount_workspace)) =
            self.runtime.build_docker_request(command, &action_dir)
        {
            let Some(config) = self.analysis_config.as_deref() else {
                return (
                    false,
                    ToolResult::error("Docker module execution is unavailable."),
                );
            };

            let explicit_timeout = self.explicit_timeout(requested_timeout);
            if let Some(timeout) = explicit_timeout {
                let timeout_ms = timeout
                    .as_nanos()
                    .saturating_add(999_999)
                    .checked_div(1_000_000)
                    .unwrap_or(0)
                    .max(1);
                let Ok(timeout_ms) = u64::try_from(timeout_ms) else {
                    return (
                        false,
                        ToolResult::error("Docker timeout exceeds the supported limit."),
                    );
                };
                if timeout_ms > crate::modules::tinybox::contract::MAX_DOCKER_RUN_TIMEOUT_MS {
                    return (
                        false,
                        ToolResult::error("Docker timeout exceeds the supported limit."),
                    );
                }
                request.timeout_ms = timeout_ms;
            }

            let Some(mut env) = container_environment(command_environment.as_deref()) else {
                return (
                    false,
                    ToolResult::error("Docker environment contains a non-UTF-8 value."),
                );
            };
            for (name, value) in tinyagents_harness_tools::fs::filesystem::shell_git_env() {
                let (Ok(name), Ok(value)) = (name.into_string(), value.into_string()) else {
                    return (
                        false,
                        ToolResult::error("Docker environment contains a non-UTF-8 value."),
                    );
                };
                env.insert(name, value);
            }

            let scratch_dir = crate::security::openhuman_scratch_dir();
            if scratch_dir.is_dir() {
                let Some(scratch_dir) = scratch_dir.to_str() else {
                    return (
                        false,
                        ToolResult::error("Docker scratch directory is not valid UTF-8."),
                    );
                };
                for name in ["TMPDIR", "TMP", "TEMP"] {
                    env.insert(name.to_owned(), scratch_dir.to_owned());
                }
            }
            request.env = env;

            let result = if mount_workspace {
                crate::modules::tinybox::run_docker(config, request).await
            } else {
                crate::modules::tinybox::run_docker_without_workspace(config, request).await
            };
            let result = match result {
                Ok(output) => docker_module_result(output, explicit_timeout),
                Err(_) => ToolResult::error("Docker module execution failed."),
            };
            return (true, result);
        }

        // The module spawns the shell: it selects cmd or bash, starts from its
        // own safe environment (no API keys or other secrets, CWE-200) and owns
        // the per-call scratch directory. The host adds only what it approves.
        if let Err(e) = crate::config::ensure_usable_cwd(&action_dir) {
            return (
                true,
                ToolResult::error(format!("Failed to build shell command: {e}")),
            );
        }
        let mut env = std::collections::BTreeMap::new();

        if let Some(overrides) = command_environment.as_deref() {
            env.extend(
                overrides
                    .iter()
                    .map(|(name, value)| (name.clone(), value.clone())),
            );
        }

        // A turn that owns its child environment (library agents) keeps it: the
        // module starts from the process snapshot, so the turn's own values for
        // the functional variables are passed as approved overrides.
        if tinyagents_harness_tools::exec::timeout::CommandEnvironment::is_active() {
            for name in crate::sandbox::ops::SANDBOX_ENV_PASSTHROUGH {
                if let Some(value) =
                    tinyagents_harness_tools::exec::timeout::CommandEnvironment::var_os(name)
                {
                    let Ok(value) = value.into_string() else {
                        return (
                            false,
                            ToolResult::error("Shell environment contains a non-UTF-8 value."),
                        );
                    };
                    env.insert((*name).to_owned(), value);
                }
            }
        }

        // Keep command-valued repository Git settings from executing host
        // programs when a shell command happens to invoke git.
        for (key, value) in tinyagents_harness_tools::fs::filesystem::shell_git_env() {
            let (Ok(key), Ok(value)) = (key.into_string(), value.into_string()) else {
                return (
                    false,
                    ToolResult::error("Shell environment contains a non-UTF-8 value."),
                );
            };
            env.insert(key, value);
        }

        // Point the child's temp dir at the agent's granted scratch dir
        // (`/tmp/openhuman`, a ReadWrite trusted root — see SecurityPolicy
        // `from_config`) so `python3 tempfile` / `mktemp` / `$TMPDIR` writes land
        // in a sandboxed, readable location instead of the world-shared /tmp.
        let scratch_dir = crate::security::openhuman_scratch_dir();
        if let (true, Some(scratch)) = (scratch_dir.is_dir(), scratch_dir.to_str()) {
            tracing::debug!(
                scratch_dir = %scratch_dir.display(),
                "[shell] overriding TMPDIR/TMP/TEMP to the openhuman scratch dir"
            );
            for name in ["TMPDIR", "TMP", "TEMP"] {
                env.insert(name.to_owned(), scratch.to_owned());
            }
        } else {
            tracing::debug!(
                scratch_dir = %scratch_dir.display(),
                "[shell] scratch dir missing — leaving TMPDIR/TMP/TEMP to the module"
            );
        }

        // No default deadline — only a caller-supplied `timeout_secs` bounds the
        // run (issue #4023). TinyBox needs a finite budget, so an unbounded call
        // gets the module's ceiling.
        let explicit_timeout = self.explicit_timeout(requested_timeout);
        let effective = explicit_timeout.unwrap_or_else(|| {
            Duration::from_secs(tinyagents_harness_tools::exec::timeout::SANDBOX_UNBOUNDED_CAP_SECS)
        });
        tracing::debug!(
            timeout_secs = effective.as_secs(),
            requested_timeout_secs = ?requested_timeout,
            unbounded = explicit_timeout.is_none(),
            "[shell] starting command through TinyBox"
        );
        let config = match self.analysis_config.as_deref() {
            Some(config) => config.clone(),
            None => match ctx.load_config().await {
                Ok(config) => config,
                Err(e) => {
                    return (
                        true,
                        ToolResult::error(format!("Cannot read module configuration: {e}")),
                    );
                }
            },
        };
        let mut run = crate::sandbox::shell_exec::NativeShellRun::new(
            command,
            &action_dir,
            &self.security.workspace_dir,
            effective,
        );
        run.env = super::shell_support::with_unbuffered_python(env, command_environment.is_some());
        run.python_utf8 = true;
        run.hide_window = self.runtime.hide_window();
        run.meter = meter;
        let result = crate::sandbox::shell_exec::run(ctx, &config, run).await;
        (true, exec_result(result, effective))
    }

    /// Execute a command through the sandbox backend. Called when the
    /// agent's `SandboxMode` is `Sandboxed`.
    async fn run_sandboxed(
        &self,
        ctx: &DispatchCtx,
        command: &str,
        requested_timeout: Option<u64>,
        action_dir: &Path,
        context: Option<&dyn ToolRunContext>,
        meter: Option<crate::sandbox::shell_progress::ShellOutputMeter>,
    ) -> (bool, ToolResult) {
        let config = crate::config::RuntimeConfig::default();
        let policy = crate::sandbox::resolve_sandbox_policy(
            ctx,
            tinyagents_definition::SandboxMode::Sandboxed,
            action_dir,
            &self.security.workspace_dir,
            &config,
            false,
        );
        self.run_in_policy(
            ctx,
            policy,
            command,
            requested_timeout,
            action_dir,
            context,
            meter,
        )
        .await
    }

    /// Execute a command under a resolved sandbox `policy`.
    async fn run_in_policy(
        &self,
        ctx: &DispatchCtx,
        policy: crate::sandbox::SandboxPolicy,
        command: &str,
        requested_timeout: Option<u64>,
        action_dir: &Path,
        context: Option<&dyn ToolRunContext>,
        meter: Option<crate::sandbox::shell_progress::ShellOutputMeter>,
    ) -> (bool, ToolResult) {
        use crate::sandbox;

        tracing::debug!(
            backend = ?policy.backend,
            command = command,
            "[shell] routing to sandbox backend"
        );

        let mut extra_env =
            std::collections::HashMap::<std::ffi::OsString, std::ffi::OsString>::new();
        let command_environment = command_environment_for_tool(context);
        super::shell_support::extend_explicit_environment(
            &mut extra_env,
            command_environment.as_deref(),
        );
        // Apply the same Git config hardening to local and sandboxed shells.
        extra_env.extend(tinyagents_harness_tools::fs::filesystem::shell_git_env());
        // And the same Python defaults the native path sets.
        for (var, val) in super::shell_support::python_env_defaults(|name| {
            command_environment
                .as_ref()
                .and_then(|overrides| overrides.get(name).map(std::ffi::OsString::from))
                .or_else(|| {
                    command_environment
                        .is_none()
                        .then(|| std::env::var_os(name))
                        .flatten()
                })
        }) {
            extra_env.insert(var.into(), val);
        }

        // Sandbox backends require a finite deadline. Without an explicit
        // `timeout_secs`, substitute the generous effective-unbounded cap so a
        // long command isn't killed while still bounding a wedged sandbox.
        let explicit_timeout = self.explicit_timeout(requested_timeout);
        let effective = explicit_timeout.unwrap_or_else(|| {
            Duration::from_secs(tinyagents_harness_tools::exec::timeout::SANDBOX_UNBOUNDED_CAP_SECS)
        });
        tracing::debug!(
            timeout_secs = effective.as_secs(),
            requested_timeout_secs = ?requested_timeout,
            unbounded = explicit_timeout.is_none(),
            "[shell] starting sandboxed command"
        );

        let result = sandbox::ops::execute_in_sandbox_metered(
            ctx, &policy, command, action_dir, extra_env, effective, meter,
        )
        .await;
        (true, exec_result(result, effective))
    }
}

#[cfg(test)]
#[path = "shell_tests.rs"]
mod tests;
