//! Cross-cutting system tools: shell, Node/npm/Python execution, tool
//! detection/install, time helpers, LSP, proxy config, update check/apply.
//! Each family, its tool structs, and its registration gates are catalogued in
//! `tools/impl/README.md`.

mod install_tool;
mod proxy_config;
mod retrieve_tool_output;
mod schedule;
mod shell;
mod shell_description;
mod shell_saas;
mod shell_support;
mod update_apply;
mod update_check;

use crate::security::SecurityPolicy;
use crate::tools::implementations::filesystem::gate::security_scoped_to_root;
use tinytools::ToolRunContext;

pub use install_tool::InstallToolTool;
pub use proxy_config::ProxyConfigTool;
pub use retrieve_tool_output::{retrieve_tool_output_tool, RetrieveToolOutputTool};
pub use schedule::ScheduleTool;
pub use shell::ShellTool;
pub use update_apply::UpdateApplyTool;
pub use update_check::UpdateCheckTool;

/// Clone and scope `security` to the run's workspace descriptor, if any.
pub(in crate::tools::implementations) fn security_for_tool_context(
    security: &SecurityPolicy,
    context: Option<&dyn ToolRunContext>,
    tool: &str,
) -> SecurityPolicy {
    if let Some(workspace) = context.and_then(|ctx| ctx.workspace()) {
        tracing::debug!(
            tool,
            workspace_root = %workspace.root.display(),
            policy_id = %workspace.policy_id,
            "[tools:system] granting TinyAgents workspace descriptor as action dir + trusted root"
        );
        security_scoped_to_root(security, &workspace.root)
    } else {
        security.clone()
    }
}
