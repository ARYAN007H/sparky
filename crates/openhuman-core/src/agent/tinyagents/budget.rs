//! Host budget policy, handed to the turn once at admission and carried
//! explicitly to children (`HostTurnEnv::model_budget` -> `OpenHumanRunContext`).

/// Product recursion ceiling shared by native and MCP delegation.
pub(crate) const MAX_SPAWN_DEPTH: usize = 3;
pub use tinyagents_harness::cost::ModelBudget;
pub use tinyinference_llm::model::budget::{
    Budget, BudgetExceeded, BudgetSnapshot, CallBudget, Spend, SpendLimits,
};
/// Effective host depth ceiling, never wider than product policy.
pub(crate) fn depth(context: &super::host::OpenHumanRunContext) -> usize {
    context
        .max_spawn_depth
        .unwrap_or(crate::agent::tinyagents::budget::MAX_SPAWN_DEPTH)
        .min(crate::agent::tinyagents::budget::MAX_SPAWN_DEPTH)
}
pub(super) fn install_depth(
    harness: &mut tinyagents_harness::runtime::AgentHarness<(), super::host::OpenHumanRunContext>,
    context: &super::host::OpenHumanRunContext,
) {
    if let Some(max_depth) = context.max_spawn_depth {
        let mut policy = harness.policy().clone();
        policy.limits.max_depth = policy.limits.max_depth.min(max_depth);
        harness.with_policy(policy);
    }
}

impl crate::inference::turn_models::TurnModels {
    /// Wrap every concrete route and summarizer below harness retries/fallback.
    pub(crate) fn with_budget(mut self, budget: Option<&ModelBudget>) -> Self {
        let Some(budget) = budget else {
            return self;
        };
        self.primary = budget.wrap_model(self.primary);
        self.summarizer = budget.wrap_model(self.summarizer);
        for (_, route) in &mut self.routes {
            *route = budget.wrap_model(route.clone());
        }
        self
    }
}
