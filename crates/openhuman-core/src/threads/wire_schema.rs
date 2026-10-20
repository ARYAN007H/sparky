//! JSON Schema for the thread and persisted turn records consumed by the UI.

use schemars::JsonSchema;

use crate::threads::rpc_models::{
    ConversationMessageRecord, ConversationMessagesResponse, ConversationThreadSummary,
    ConversationThreadsListResponse, DeleteConversationThreadResponse,
    PurgeConversationThreadsResponse,
};

#[derive(JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunLedgerGetWireResponse {
    pub run: Option<tinyagents_session::run_ledger::types::AgentRun>,
}

/// Root schema whose properties provide the UI-facing thread record aliases.
#[derive(JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ThreadWireTypes {
    pub thread: ConversationThreadSummary,
    pub message: ConversationMessageRecord,
    pub threads_list: ConversationThreadsListResponse,
    pub thread_messages: ConversationMessagesResponse,
    pub thread_delete: DeleteConversationThreadResponse,
    pub purge_result: PurgeConversationThreadsResponse,
    pub thread_search_hit: super::ops::ThreadSearchHit,
    pub thread_search: super::ops::ThreadSearchResponse,
    pub get_turn_state: super::turn_state::rpc_types::GetTurnStateResponse,
    pub list_turn_states: super::turn_state::rpc_types::ListTurnStatesResponse,
    pub clear_turn_state: super::turn_state::rpc_types::ClearTurnStateResponse,
    pub agent_run: tinyagents_session::run_ledger::types::AgentRun,
    pub run_event: tinyagents_session::run_ledger::types::RunEvent,
    pub run_telemetry: tinyagents_session::run_ledger::types::RunTelemetry,
    pub agent_run_list: tinyagents_session::run_ledger::types::AgentRunListResponse,
    pub agent_run_get: RunLedgerGetWireResponse,
    pub run_event_list: tinyagents_session::run_ledger::types::RunEventListResponse,
}

/// Serialize the authoritative core/session record schemas for TypeScript codegen.
pub fn thread_wire_schema_json() -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&schemars::schema_for!(ThreadWireTypes))
}

#[cfg(test)]
#[path = "wire_schema_tests.rs"]
mod tests;
