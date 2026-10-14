//! The immutable authority view the run exposes to shared tools through the
//! harness's typed state-view seam, and the readers tools use on it.

use crate::agent::turn_origin::AgentTurnOrigin;
use tinyagents_definition::SandboxMode;

/// Minimal immutable authority view exposed to shared tools through the
/// harness's typed state-view seam.
#[derive(Clone, Debug, Default)]
pub(crate) struct HostOperationContext {
    pub(crate) origin: Option<AgentTurnOrigin>,
    pub(crate) sandbox_mode: Option<SandboxMode>,
    /// The agent definition the run's parent execution context belongs to, for
    /// tools that act on behalf of the spawning agent when no memory identity
    /// is scoped (see `memory::tools::CallFacts::gather`).
    pub(crate) parent_agent_id: Option<String>,
}

/// Reads the spawning agent's definition id from the explicit tool carrier.
/// Direct tool dispatch has no parent.
pub(crate) fn parent_agent_id_for_tool(
    context: Option<&dyn tinytools::ToolRunContext>,
) -> Option<String> {
    context
        .and_then(tinytools::ToolRunContext::host_extension)
        .and_then(|extension| {
            extension.downcast_ref::<tinyagents_harness::tool::ToolExecutionContext>()
        })
        .and_then(|context| context.state::<HostOperationContext>())
        .and_then(|host| host.parent_agent_id.clone())
}

/// Reads the calling agent's sandbox policy from the explicit tool carrier.
/// Direct tool dispatch has no agent-specific mode.
pub(crate) fn sandbox_mode_for_tool(
    context: Option<&dyn tinytools::ToolRunContext>,
) -> Option<SandboxMode> {
    context
        .and_then(tinytools::ToolRunContext::host_extension)
        .and_then(|extension| {
            extension.downcast_ref::<tinyagents_harness::tool::ToolExecutionContext>()
        })
        .and_then(|context| context.state::<HostOperationContext>())
        .and_then(|host| host.sandbox_mode)
}
