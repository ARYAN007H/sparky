//! The sub-agent arms of [`ProgressProjection::on_event`]: lifecycle, child
//! tool calls and child deltas, each mirrored into the run ledger.

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
        let round = self.round;
        match event {
            AgentProgress::SubagentSpawned {
                agent_id,
                task_id,
                mode,
                dedicated_thread,
                prompt_chars,
                worker_thread_id,
                display_name,
                parent_call_id: _,
                ..
            } => {
                self.surface
                    .output(map::sub_spawned(
                        &agent_id,
                        &task_id,
                        round,
                        mode,
                        dedicated_thread,
                        prompt_chars,
                        worker_thread_id,
                        display_name,
                        ledger_context.parent_call_id,
                    ))
                    .await;
            }
            AgentProgress::SubagentCompleted {
                agent_id,
                task_id,
                elapsed_ms,
                iterations,
                output_chars,
                output,
                usage,
                worktree_path,
                changed_files,
                dirty_status,
                ..
            } => {
                self.surface
                    .output(map::sub_completed(
                        &agent_id,
                        &task_id,
                        elapsed_ms,
                        iterations,
                        output_chars,
                        output,
                        usage.as_ref(),
                        worktree_path,
                        changed_files,
                        dirty_status,
                        ledger_context.parent_call_id,
                    ))
                    .await;
            }
            AgentProgress::SubagentFailed {
                agent_id,
                task_id,
                error,
            } => {
                log::warn!(
                    "[web_channel][bridge] subagent_failed agent_id={agent_id} err={error} client_id={} thread_id={} request_id={}",
                    self.ids.client_id,
                    self.ids.thread_id,
                    self.ids.request_id
                );
                self.surface
                    .output(map::sub_failed(
                        &agent_id,
                        &task_id,
                        round,
                        error,
                        ledger_context.parent_call_id,
                    ))
                    .await;
            }
            AgentProgress::SubagentAwaitingUser {
                agent_id,
                task_id,
                question,
                worker_thread_id,
                ..
            } => {
                self.surface
                    .output(map::sub_awaiting_user(
                        &agent_id,
                        &task_id,
                        round,
                        question,
                        worker_thread_id,
                        ledger_context.parent_call_id,
                    ))
                    .await;
            }
            AgentProgress::SubagentIterationStarted {
                agent_id,
                task_id,
                iteration,
                max_iterations,
                extended_policy,
            } => {
                self.surface
                    .output(map::sub_iteration(
                        &agent_id,
                        &task_id,
                        iteration,
                        max_iterations,
                        extended_policy,
                    ))
                    .await;
            }
            AgentProgress::SubagentToolCallStarted {
                agent_id,
                task_id,
                call_id,
                tool_name,
                arguments,
                iteration,
                display_label,
                display_detail,
            } => {
                self.surface
                    .output(map::sub_tool_started(
                        &agent_id,
                        &task_id,
                        call_id,
                        tool_name,
                        arguments,
                        iteration,
                        display_label,
                        display_detail,
                    ))
                    .await;
            }
            AgentProgress::SubagentToolCallCompleted { .. } => {
                if let Some(output) = map::sub_tool_completed(event) {
                    self.surface.output(output).await;
                }
            }
            AgentProgress::SubagentTextDelta {
                agent_id,
                task_id,
                delta,
                iteration,
            } => {
                self.surface
                    .output(map::sub_delta(&agent_id, &task_id, false, delta, iteration))
                    .await;
            }
            AgentProgress::SubagentThinkingDelta {
                agent_id,
                task_id,
                delta,
                iteration,
            } => {
                self.surface
                    .output(map::sub_delta(&agent_id, &task_id, true, delta, iteration))
                    .await;
            }
            other => {
                log::debug!(
                    "[web_channel][bridge] unprojected event={:?} request_id={}",
                    std::mem::discriminant(&other),
                    self.ids.request_id,
                );
            }
        }
    }
}
