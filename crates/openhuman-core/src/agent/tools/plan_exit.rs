//! OpenHuman's tenant-aware adapter for the harness-owned `plan_exit` tool.

use std::hash::{Hash, Hasher};
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tinyagents_harness::middleware::RunModeHandle;
use tinyagents_harness_tools::run_mode::RunModeStore;
use tinyagents_harness_tools::{ParkedPlanReviewLookup, PlanExitHost};
use tinytools::{PermissionLevel, Tool, ToolCallOptions, ToolResult, ToolRunContext};

#[derive(Clone)]
struct PlanExitScope {
    context: crate::core::runtime::DispatchCtx,
    tenant: crate::core::runtime::Tenant,
    valid: bool,
}

impl PartialEq for PlanExitScope {
    fn eq(&self, other: &Self) -> bool {
        self.valid == other.valid && self.tenant == other.tenant
    }
}

impl Eq for PlanExitScope {}

impl Hash for PlanExitScope {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.valid.hash(state);
        self.tenant.hash(state);
    }
}

struct OpenHumanPlanExitHost;

impl PlanExitHost for OpenHumanPlanExitHost {
    type Scope = PlanExitScope;

    fn scope_and_thread(&self, context: &dyn ToolRunContext) -> (Self::Scope, String) {
        let dispatch =
            crate::agent::tinyagents::host::run_context::dispatch_ctx_for_tool(Some(context));
        let tenant = dispatch.tenant();
        let (tenant, valid) = match tenant {
            Ok(tenant) => (tenant, true),
            Err(_) => (crate::core::runtime::Tenant::default(), false),
        };
        let scope = PlanExitScope {
            context: dispatch,
            tenant,
            valid,
        };
        (scope, context.thread_id().unwrap_or_default().to_owned())
    }
}

struct OpenHumanRunModes;

impl RunModeStore<PlanExitScope> for OpenHumanRunModes {
    fn handle_for_thread(&self, scope: &PlanExitScope, thread_id: &str) -> RunModeHandle {
        crate::threads::run_mode::handle_for_thread(&scope.context, thread_id)
    }

    fn get_mode(&self, scope: &PlanExitScope, thread_id: &str) -> tinyagents_harness::middleware::RunMode {
        crate::threads::run_mode::get_mode(&scope.context, thread_id)
    }

    fn set_mode(
        &self,
        scope: &PlanExitScope,
        thread_id: &str,
        mode: tinyagents_harness::middleware::RunMode,
    ) {
        if scope.valid && !thread_id.is_empty() {
            crate::threads::run_mode::set_mode(&scope.context, thread_id, mode);
        }
    }
}

struct OpenHumanParkedReviews;

impl ParkedPlanReviewLookup<PlanExitScope> for OpenHumanParkedReviews {
    fn has_parked_review(&self, scope: &PlanExitScope, thread_id: &str) -> bool {
        scope.valid
            && crate::plan_review::gate::global()
                .parked_review_for_thread(&scope.tenant, thread_id)
                .is_some()
    }
}

fn owner_tool() -> tinyagents_harness_tools::PlanExitTool<PlanExitScope> {
    tinyagents_harness_tools::PlanExitTool::new(
        Arc::new(OpenHumanPlanExitHost),
        Arc::new(OpenHumanRunModes),
        Some(Arc::new(OpenHumanParkedReviews)),
    )
}

/// Public unit-shaped compatibility adapter around TinyAgents' tool engine.
pub struct PlanExitTool;

impl PlanExitTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PlanExitTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for PlanExitTool {
    fn name(&self) -> &str {
        tinyagents_harness_tools::PlanExitTool::<PlanExitScope>::NAME
    }

    fn description(&self) -> &str {
        tinyagents_harness_tools::PlanExitTool::<PlanExitScope>::DESCRIPTION
    }

    fn parameters_schema(&self) -> Value {
        owner_tool().parameters_schema()
    }

    fn permission_level(&self) -> PermissionLevel {
        owner_tool().permission_level()
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        owner_tool().execute(args).await
    }

    async fn execute_with_context(
        &self,
        args: Value,
        options: ToolCallOptions,
        context: Option<&dyn ToolRunContext>,
    ) -> anyhow::Result<ToolResult> {
        owner_tool()
            .execute_with_context(args, options, context)
            .await
    }
}

#[cfg(test)]
#[path = "plan_exit_tests.rs"]
mod tests;
