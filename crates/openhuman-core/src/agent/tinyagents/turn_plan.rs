//! The host's [`TurnPlanner`]: what one OpenHuman turn plugs into a
//! [`TurnDriver`](tinyagents_runtime::turn_driver::TurnDriver).
//!
//! The driver assembles the harness; this file decides the product-shaped
//! inputs it assembles from: the provider models (primary first, then the
//! workload-tier routes), the run policy (retry, cross-route fallback, tool
//! discovery, dialect, reasoning, tool rules) and the tool registration (shared
//! sets, fail-closed admission, session deferral, typed dispatches, turn-origin
//! scope). The middleware stack that wraps model and tool calls is installed on
//! the prepared harness by [`turn_middleware`](super::turn_middleware).

use std::collections::HashSet;
use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;
use tinyagents_harness::runtime::RunPolicy;
use tinyagents_registry::RegistryDiagnostic;
use tinyagents_runtime::turn_driver::{SharedToolSet, TurnPlan, TurnPlanner};
use tinyagents_runtime::{DriverRequest, RuntimeError};
use tinyinference_llm::model::ErrorSlotModel;

use crate::agent::tinyagents::host::OpenHumanRunContext;
use crate::agent::tinyagents::model::TurnChatModel;
use crate::agent::tinyagents::routes;
use crate::agent::tinyagents::tool_registration::{
    capability_diagnostics, host_tool_set_builder, turn_admission,
};
use crate::agent::tinyagents::turn_policy::run_policy_for_provider;

/// The run policy for one turn, and whether it carries a cross-route fallback.
///
/// Cross-route fallback ownership (issue #4249, Workstream 02.2): the SDK
/// `RunPolicy.fallback` holds the ordered same-family route chain for the
/// turn's primary model, so the harness fails over to a sibling workload tier
/// (e.g. `hint:chat` to `hint:burst`) when the primary route errors. Retry
/// stays pinned to a single attempt (see `run_policy_for`): fallback and retry
/// are independent knobs, and only fallback is enabled here because
/// `ReliableProvider` (still wrapped) does not fail over across the registered
/// tier routes.
pub(super) fn turn_run_policy(
    model: &str,
    max_iterations: usize,
    deterministic_cacheable: bool,
    local_provider: bool,
    tool_dialect: tinyagents_harness::config::ToolDispatcher,
    reasoning: Option<tinyinference_llm::model::ReasoningConfig>,
    tool_rules: Option<Arc<tinyagents_harness::tool::ToolRulePolicy>>,
) -> (RunPolicy, bool) {
    let mut policy =
        run_policy_for_provider(max_iterations, deterministic_cacheable, local_provider);
    let route_fallback = routes::route_fallback_policy(model);
    policy.fallback = route_fallback.clone();
    // Tool discovery: the harness advertises its `tool_search` bridge over the
    // `Deferred` registrations the allowlist admits, ranked by whatever the
    // process installed (`agent::tinyagents::discovery`).
    policy.discovery = super::discovery::discovery_policy();
    policy.tool_dialect = tool_dialect;
    policy.default_reasoning = reasoning;
    // The session composes its prompt for this same dialect: `ToolsSection`
    // renders the protocol block and the catalogue of the visible tools into
    // the system prompt (inside the cacheable prefix, counted by
    // `prompt-size`). Without this the harness appended a second copy of both
    // on every text-dialect call.
    policy.host_renders_tool_catalogue = true;
    crate::tools::rules::install_turn_rules(&mut policy, tool_rules);
    tracing::debug!(
        model,
        ?tool_dialect,
        fallback_chain = ?route_fallback.as_ref().map(|f| &f.models),
        "[models] turn policy: dialect pinned from the session, SDK retry/fallback applied"
    );
    (policy, route_fallback.is_some())
}

/// The models a turn registers, primary first. A non-hosted run owns its
/// registry, so its models record into the run's error slot (#6724). A hosted
/// root shares its runtime registry with its sub-agents; there only the
/// resolver's lead model records.
///
/// Each workload route is an additive registry entry carrying its per-route
/// capability profile; the primary stays the default and dispatch target.
pub(super) fn turn_models_for_plan(
    model: &str,
    primary: TurnChatModel,
    tier_routes: Vec<(String, TurnChatModel)>,
    error_slot: &tinyinference_llm::model::ModelErrorSlot,
    hosted_security_gate: bool,
) -> Vec<(String, TurnChatModel)> {
    let slotted = |model: TurnChatModel| -> TurnChatModel {
        if hosted_security_gate {
            model
        } else {
            Arc::new(ErrorSlotModel::new(model, error_slot.clone()))
        }
    };
    std::iter::once((model.to_string(), primary))
        .chain(tier_routes)
        .map(|(name, route)| (name, slotted(route)))
        .collect()
}

/// Everything the planner decides a turn from.
pub(super) struct OpenHumanTurnPlanner {
    pub(super) models: Vec<(String, TurnChatModel)>,
    pub(super) policy: RunPolicy,
    pub(super) tool_sets: Vec<SharedToolSet>,
    pub(super) allowed: Option<HashSet<String>>,
    pub(super) has_thread: bool,
    pub(super) is_subagent: bool,
    pub(super) session_deferred: Arc<HashSet<String>>,
    /// Receives the capability registry's health diagnostics; the turn path
    /// aborts on an error-severity one before the first model dispatch.
    pub(super) diagnostics: Arc<Mutex<Vec<RegistryDiagnostic>>>,
}

#[async_trait]
impl TurnPlanner<(), OpenHumanRunContext> for OpenHumanTurnPlanner {
    async fn plan(
        &self,
        _request: &DriverRequest<OpenHumanRunContext>,
    ) -> Result<TurnPlan<(), OpenHumanRunContext>, RuntimeError> {
        let mut admission = turn_admission(&self.tool_sets, &self.allowed, self.has_thread);
        admission.is_subagent_run = self.is_subagent;
        *self
            .diagnostics
            .lock()
            .unwrap_or_else(PoisonError::into_inner) =
            capability_diagnostics(&self.models, &self.tool_sets, &admission);

        let mut plan = TurnPlan::new(self.models.clone());
        plan.policy = self.policy.clone();
        plan.tools = host_tool_set_builder(
            &self.tool_sets,
            &self.allowed,
            self.has_thread,
            &self.session_deferred,
        );
        // A turn driven outside a `Session` has no frozen tool declaration to
        // compare against; the registration above is the authority.
        plan.enforce_tool_snapshot = false;
        Ok(plan)
    }
}

