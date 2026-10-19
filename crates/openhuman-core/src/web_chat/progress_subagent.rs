//! The sub-agent arms of [`ProgressProjection::on_event`]: lifecycle, child
//! tool calls and child deltas, each mirrored into the run ledger.

use serde_json::json;

use tinyagents_harness::progress::AgentProgress;

use super::super::progress_ledger::ChildDone;
use super::super::progress_map as map;
use super::ProgressProjection;

impl ProgressProjection {
    pub(super) async fn on_subagent_event(&mut self, event: AgentProgress) {
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
                parent_call_id,
                ..
            } => {
                self.subagent_parent_call_ids
                    .insert(task_id.clone(), parent_call_id.clone());
                self.ledger().child_spawned(
                    &agent_id,
                    &task_id,
                    &mode,
                    dedicated_thread,
                    prompt_chars,
                    &worker_thread_id,
                    &display_name,
                    &parent_call_id,
                );
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
                        parent_call_id,
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
                let parent_call_id = self.subagent_parent_call_ids.remove(&task_id).flatten();
                self.ledger().child_completed(&ChildDone {
                    agent_id: &agent_id,
                    task_id: &task_id,
                    elapsed_ms,
                    iterations,
                    output_chars,
                    worktree_path: &worktree_path,
                    changed_files: &changed_files,
                    dirty_status,
                    parent_call_id: &parent_call_id,
                    tool_count: self.child_tool_counts.get(&task_id).copied(),
                });
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
                        parent_call_id,
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
                let parent_call_id = self.subagent_parent_call_ids.remove(&task_id).flatten();
                self.ledger().child_failed(
                    &agent_id,
                    &task_id,
                    &error,
                    &parent_call_id,
                    self.child_tool_counts.get(&task_id).copied(),
                );
                self.surface
                    .output(map::sub_failed(
                        &agent_id,
                        &task_id,
                        round,
                        error,
                        parent_call_id,
                    ))
                    .await;
            }
            AgentProgress::SubagentAwaitingUser {
                agent_id,
                task_id,
                question,
                worker_thread_id,
                checkpoint_path,
            } => {
                let parent_call_id = self
                    .subagent_parent_call_ids
                    .get(&task_id)
                    .cloned()
                    .flatten();
                self.ledger().child_awaiting_user(
                    &agent_id,
                    &task_id,
                    &question,
                    &worker_thread_id,
                    &checkpoint_path,
                    &parent_call_id,
                );
                self.surface
                    .output(map::sub_awaiting_user(
                        &agent_id,
                        &task_id,
                        round,
                        question,
                        worker_thread_id,
                        parent_call_id,
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
                let count = self.child_tool_counts.entry(task_id.clone()).or_insert(0);
                *count += 1;
                let count = *count;
                self.ledger().child_tool_started(
                    &agent_id, &task_id, &call_id, &tool_name, iteration, count,
                );
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
                if let AgentProgress::SubagentToolCallCompleted {
                    agent_id,
                    task_id,
                    call_id,
                    tool_name,
                    success,
                    output_chars,
                    elapsed_ms,
                    iteration,
                    failure,
                    ..
                } = &event
                {
                    self.ledger().child_tool_completed(
                        task_id,
                        json!({
                            "agentId": agent_id,
                            "callId": call_id,
                            "toolName": tool_name,
                            "success": success,
                            "outputChars": output_chars,
                            "elapsedMs": elapsed_ms,
                            "iteration": iteration,
                            "failure": failure.as_ref().and_then(|f| serde_json::to_value(f).ok()),
                        }),
                    );
                }
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
