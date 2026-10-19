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

#[cfg(test)]
#[path = "binding_tests.rs"]
mod binding_tests;

pub(crate) use binding::{CapsHostRegistration, serve};
pub(crate) use agent::OpenHumanAgentRunner;
pub(crate) use code::OpenHumanCode;
pub(crate) use error::{CapsError, EngineError, Result};
pub(crate) use http::{OpenHumanHttp, resolve_http_credential};
pub(crate) use llm::{OpenHumanLlm, resolve_host_model};
pub(crate) use memory::OpenHumanMemory;
pub(crate) use prompt::model_response_to_completion_value;
pub(crate) use service::CapsHostService;
pub(crate) use tier::{
    close_audit, current_tier_policy, enforce_current_node_tier_gate, enforce_node_tier_gate,
    gate_call_for_tier,
};
pub(crate) use tools::OpenHumanTools;
pub(crate) use tools::{NATIVE_TOOL_PREFIX, classify_composio_action_for_tier};

#[cfg(test)]
#[path = "ops_tests.rs"]
mod tests;
