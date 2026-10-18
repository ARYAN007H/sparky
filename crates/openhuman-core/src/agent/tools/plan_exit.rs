//! OpenHuman's tenant-aware lookup for the harness-owned `plan_exit` tool.

use tinyagents_harness::middleware::RunModeHandle;
use tinytools::ToolRunContext;

/// Resolve the live thread mode only when no plan review is still pending.
pub(crate) fn mode_lookup(context: &dyn ToolRunContext) -> Option<RunModeHandle> {
    let thread_id = context.thread_id()?;
    let dispatch_ctx = crate::agent::tinyagents::host::run_context::dispatch_ctx_for_tool(Some(context));
    let tenant = dispatch_ctx.tenant().ok()?;
    if crate::plan_review::gate::global()
        .parked_review_for_thread(&tenant, thread_id)
        .is_some()
    {
        return None;
    }
    Some(crate::threads::run_mode::handle_for_thread(&dispatch_ctx, thread_id))
}

#[cfg(test)]
#[path = "plan_exit_tests.rs"]
mod tests;
