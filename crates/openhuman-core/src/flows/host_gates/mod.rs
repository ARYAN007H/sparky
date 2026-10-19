//! OpenHuman-owned stages supplied to TinyFlows authoring operations.

mod agent_refs;
mod connections;
mod tool_contracts;
mod upload_paths;

pub(crate) use agent_refs::validate_agent_refs;
pub(crate) use connections::validate_connection_refs;
#[cfg(test)]
pub(crate) use connections::validate_connection_refs_against;
pub(crate) use tool_contracts::validate_tool_contracts;
pub(crate) use upload_paths::validate_upload_paths;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "agent_refs_tests.rs"]
mod agent_refs_tests;
