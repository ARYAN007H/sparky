//! Per-call and per-turn cost accounting for an agent's tool-call loop.
//!
//! A call's cost is one of three things, never a guess:
//!
//! - **Charged**: the amount the provider actually billed, reported back on
//!   the response (`openhuman.billing.charged_amount_usd` from the managed
//!   backend, `total_cost_usd` from a CLI provider). It may be zero for a
//!   free route; that is still a known cost.
//! - **Estimated**: no charge was reported, but the model is in the vendor
//!   price catalog ([`crate::platform::cost::catalog`]), so its published
//!   list rates price the token counts. Reported as an estimate.
//! - **Unknown**: no charge and no catalogued price. Nothing is reported for
//!   it; a turn with any unknown call has an unknown total.
//!
//! There is deliberately no default rate for an unrecognised model. One used
//! to exist ($3 / $0.30 / $15 per MTok) and priced a glm-5.3-flash thread at
//! $4.25 when the provider billed about $0.30.

use crate::inference::provider::BilledUsage;
use tinyagents_harness::cost::{CallCost, CallUsage, CostTally};

/// Whether a model names an explicitly free OpenRouter variant.
pub(crate) fn is_free_model(model: &str) -> bool {
    model.trim().to_ascii_lowercase().ends_with(":free")
}

/// Supply product catalog rates as data to the harness accounting API.
pub(crate) fn lookup_pricing(model: &str) -> Option<tinyagents_harness::cost::ModelPricing> {
    if is_free_model(model) {
        return Some(tinyagents_harness::cost::ModelPricing {
            input_per_token: Some(0.0),
            cache_read_input_per_token: Some(0.0),
            output_per_token: Some(0.0),
            ..Default::default()
        });
    }
    let price = tinyagents_registry::pricing::lookup(model.trim())?;
    Some(tinyagents_registry::pricing::flat_catalog_entry(price, "openhuman-cost-catalog").pricing)
}

/// Estimate a call from product list prices when no provider charge was reported.
pub fn estimate_call_cost_usd(model: &str, usage: &BilledUsage) -> Option<f64> {
    tinyagents_harness::cost::estimate_call_cost_usd(
        lookup_pricing(model).as_ref(),
        &call_usage(usage),
    )
}

/// The harness accounting view of one billed call.
pub(crate) fn call_usage(usage: &BilledUsage) -> CallUsage {
    CallUsage {
        usage: usage.usage,
        charged_amount_usd: usage.charged_amount_usd,
        cost_is_estimate: usage.cost_is_estimate,
        charge_reported: usage.charge_reported,
    }
}

/// The cost of one call: the provider's charge when it reported one, else
/// the catalog estimate, else unknown.
pub fn call_cost(model: &str, usage: &BilledUsage) -> CallCost {
    tinyagents_harness::cost::call_cost(lookup_pricing(model).as_ref(), &call_usage(usage))
}

/// Running cost / token tally across every provider call inside a
/// single turn of the tool-call loop.
#[derive(Debug, Clone, Default)]
pub struct TurnCost {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_input_tokens: u64,
    pub cost: CostTally,
    pub call_count: u32,
}

impl TurnCost {
    /// New empty accumulator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold a single provider call's usage into the running totals.
    pub fn add_call(&mut self, model: &str, usage: &BilledUsage) {
        self.input_tokens = self.input_tokens.saturating_add(usage.input_tokens);
        self.output_tokens = self.output_tokens.saturating_add(usage.output_tokens);
        self.cached_input_tokens = self
            .cached_input_tokens
            .saturating_add(usage.cached_input_tokens());
        self.cost.add(call_cost(model, usage));
        self.call_count = self.call_count.saturating_add(1);
    }

    /// The turn's cost so far, or `None` when any call's cost is unknown.
    pub fn total_usd(&self) -> Option<f64> {
        self.cost.usd()
    }
}

#[cfg(test)]
#[path = "cost_tests.rs"]
mod tests;
