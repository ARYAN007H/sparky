//! Host adapters served to the compiled TinyFlows module for one run.

mod agent;
mod binding;
mod code;
mod error;
mod http;
mod llm;
mod memory;
mod prompt;
mod service;
mod tier;
pub(crate) mod tools;

pub(crate) use binding::{CapsHostRegistration, serve};
pub(crate) use error::{CapsError, EngineError, Result};
pub(crate) use llm::OpenHumanLlm;
pub(crate) use service::CapsHostService;
pub(crate) use tools::OpenHumanTools;
pub(crate) use tier::{
    close_audit, current_tier_policy, enforce_current_node_tier_gate,
    enforce_node_tier_gate, gate_call_for_tier,
};
pub(crate) use tools::NATIVE_TOOL_PREFIX;

#[cfg(test)]
#[path = "ops_tests.rs"]
mod tests;

