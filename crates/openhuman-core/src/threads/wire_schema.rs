//! JSON Schema for the thread and persisted turn records consumed by the UI.

use schemars::JsonSchema;

use crate::threads::rpc_models::{ConversationMessageRecord, ConversationThreadSummary};

/// Root schema whose properties provide the UI-facing thread record aliases.
#[derive(JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ThreadWireTypes {
    pub thread: ConversationThreadSummary,
    pub message: ConversationMessageRecord,
    pub turn_state: tinyagents_session::turn_state::types::TurnState,
    pub agent_run: tinyagents_session::run_ledger::types::AgentRun,
    pub run_event: tinyagents_session::run_ledger::types::RunEvent,
    pub run_telemetry: tinyagents_session::run_ledger::types::RunTelemetry,
}

/// Serialize the authoritative core/session record schemas for TypeScript codegen.
pub fn thread_wire_schema_json() -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&schemars::schema_for!(ThreadWireTypes))
}

#[cfg(test)]
#[path = "wire_schema_tests.rs"]
mod tests;
