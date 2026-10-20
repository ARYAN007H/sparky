//! Surface-specific logging around the shared sub-agent progress mapper.

use tinyagents_harness::progress::AgentProgress;

use super::super::progress_ledger::ProgressLedgerContext;
use super::super::progress_map as map;
use super::ProgressProjection;

impl ProgressProjection {
    pub(super) async fn on_subagent_event(
        &mut self,
        event: AgentProgress,
        ledger_context: ProgressLedgerContext,
    ) {
        if let AgentProgress::SubagentFailed {
            agent_id, error, ..
        } = &event
        {
            log::warn!(
                "[web_channel][bridge] subagent_failed agent_id={agent_id} err={error} client_id={} thread_id={} request_id={}",
                self.ids.client_id,
                self.ids.thread_id,
                self.ids.request_id
            );
        }

        if let Some(output) = map::subagent_event(event, self.round, ledger_context.parent_call_id)
        {
            self.surface.output(output).await;
        }
    }
}
