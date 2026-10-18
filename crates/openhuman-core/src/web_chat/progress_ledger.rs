//! The run ledger a turn's progress writes into: one run row per turn, one
//! per sub-agent, with their events and telemetry. Host persistence policy;
//! none of it reaches the wire.

use serde_json::json;
use tinyagents_session::run_ledger::{
    AgentRunKind, AgentRunStatus, AgentRunUpsert, RunEventAppend, RunTelemetry, RunTelemetryUpsert,
};

use crate::agent::session_store::ledger_sync;
use crate::config::Config;

/// Writer for one turn's ledger rows.
pub(super) struct Ledger<'a> {
    pub(super) config: &'a Config,
    pub(super) client_id: &'a str,
    pub(super) thread_id: &'a str,
    pub(super) request_id: &'a str,
}

/// What a finished sub-agent adds to its ledger row.
pub(super) struct ChildDone<'a> {
    pub(super) agent_id: &'a str,
    pub(super) task_id: &'a str,
    pub(super) elapsed_ms: u64,
    pub(super) iterations: u32,
    pub(super) output_chars: usize,
    pub(super) worktree_path: &'a Option<String>,
    pub(super) changed_files: &'a [String],
    pub(super) dirty_status: Option<bool>,
    pub(super) parent_call_id: &'a Option<String>,
    pub(super) tool_count: Option<u64>,
}

impl Ledger<'_> {
    fn run(&self, upsert: AgentRunUpsert) {
        if let Err(err) = ledger_sync::upsert_agent_run(&self.config.workspace_dir, upsert) {
            log::warn!("[run_ledger][web_channel] failed to upsert run: {err}");
        }
    }

    fn event(&self, run_id: &str, event_type: &str, payload: serde_json::Value) {
        let append = RunEventAppend {
            run_id: run_id.to_owned(),
            event_type: event_type.to_owned(),
            payload,
        };
        if let Err(err) = ledger_sync::append_run_event(&self.config.workspace_dir, append) {
            log::warn!("[run_ledger][web_channel] failed to append event: {err}");
        }
    }

    fn telemetry(&self, telemetry: RunTelemetryUpsert) {
        if let Err(err) = ledger_sync::upsert_run_telemetry(&self.config.workspace_dir, telemetry) {
            log::warn!("[run_ledger][web_channel] failed to upsert telemetry: {err}");
        }
    }

    /// The turn's telemetry as written so far, for the trace export.
    pub(super) fn read_telemetry(&self) -> Option<RunTelemetry> {
        match ledger_sync::get_agent_run(&self.config.workspace_dir, self.request_id) {
            Ok(run) => run.and_then(|run| run.telemetry),
            Err(err) => {
                log::warn!(
                    "[run_ledger][web_channel] failed to read telemetry run_id={} err={err}",
                    self.request_id
                );
                None
            }
        }
    }

    fn turn_row(
        &self,
        status: AgentRunStatus,
        summary: Option<String>,
        error: Option<String>,
        metadata: serde_json::Value,
        done: bool,
    ) -> AgentRunUpsert {
        AgentRunUpsert {
            id: self.request_id.to_owned(),
            kind: AgentRunKind::BackgroundAgent,
            parent_run_id: None,
            parent_thread_id: Some(self.thread_id.to_owned()),
            agent_id: Some("orchestrator".to_owned()),
            status,
            prompt_ref: Some(format!(
                "thread:{}:request:{}",
                self.thread_id, self.request_id
            )),
            worker_thread_id: None,
            checkpoint_path: None,
            checkpoint: None,
            summary,
            error,
            metadata,
            started_at: None,
            completed_at: done.then(chrono::Utc::now),
        }
    }

    pub(super) fn turn_started(&self) {
        self.run(self.turn_row(
            AgentRunStatus::Running,
            None,
            None,
            json!({ "clientId": self.client_id, "source": "web_channel", "schemaVersion": 1 }),
            false,
        ));
        self.event(
            self.request_id,
            "turn_started",
            json!({ "threadId": self.thread_id, "clientId": self.client_id }),
        );
    }

    pub(super) fn turn_completed(&self, iterations: u32) {
        self.run(self.turn_row(
            AgentRunStatus::Completed,
            Some(format!("Completed in {iterations} iteration(s)")),
            None,
            json!({}),
            true,
        ));
        self.event(
            self.request_id,
            "turn_completed",
            json!({ "iterations": iterations }),
        );
    }

    pub(super) fn turn_interrupted(&self, events_seen: u64) {
        self.run(self.turn_row(
            AgentRunStatus::Interrupted,
            None,
            Some("progress bridge exited before turn completion".to_owned()),
            json!({}),
            true,
        ));
        self.event(
            self.request_id,
            "turn_interrupted",
            json!({ "eventsSeen": events_seen }),
        );
    }

    pub(super) fn tool_started(&self, call_id: &str, tool_name: &str, iteration: u32, count: u64) {
        self.event(
            self.request_id,
            "tool_call_started",
            json!({ "callId": call_id, "toolName": tool_name, "iteration": iteration }),
        );
        self.telemetry(RunTelemetryUpsert {
            run_id: self.request_id.to_owned(),
            tool_count: Some(count),
            ..Default::default()
        });
    }

    pub(super) fn tool_completed(&self, payload: serde_json::Value) {
        self.event(self.request_id, "tool_call_completed", payload);
    }

    pub(super) fn cost(
        &self,
        model: &str,
        input_tokens: u64,
        output_tokens: u64,
        cached_input_tokens: u64,
        cost_usd: f64,
    ) {
        self.telemetry(RunTelemetryUpsert {
            run_id: self.request_id.to_owned(),
            input_tokens: Some(input_tokens),
            output_tokens: Some(output_tokens),
            cached_input_tokens: Some(cached_input_tokens),
            cost_usd: Some(cost_usd),
            model: Some(model.to_owned()),
            ..Default::default()
        });
    }

    fn child_row(
        &self,
        task_id: &str,
        agent_id: &str,
        kind: AgentRunKind,
        status: AgentRunStatus,
    ) -> AgentRunUpsert {
        AgentRunUpsert {
            id: task_id.to_owned(),
            kind,
            parent_run_id: Some(self.request_id.to_owned()),
            parent_thread_id: Some(self.thread_id.to_owned()),
            agent_id: Some(agent_id.to_owned()),
            status,
            prompt_ref: None,
            worker_thread_id: None,
            checkpoint_path: None,
            checkpoint: None,
            summary: None,
            error: None,
            metadata: json!({}),
            started_at: None,
            completed_at: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn child_spawned(
        &self,
        agent_id: &str,
        task_id: &str,
        mode: &str,
        dedicated_thread: bool,
        prompt_chars: usize,
        worker_thread_id: &Option<String>,
        display_name: &Option<String>,
        parent_call_id: &Option<String>,
    ) {
        let kind = if worker_thread_id.is_some() {
            AgentRunKind::WorkerThread
        } else {
            AgentRunKind::Subagent
        };
        let mut row = self.child_row(task_id, agent_id, kind, AgentRunStatus::Running);
        row.prompt_ref = worker_thread_id
            .as_ref()
            .map(|id| format!("thread:{id}:message:seed"));
        row.worker_thread_id.clone_from(worker_thread_id);
        row.metadata = json!({
            "mode": mode,
            "dedicatedThread": dedicated_thread,
            "promptChars": prompt_chars,
            "displayName": display_name,
            "parentCallId": parent_call_id,
            "source": "agent_progress",
            "schemaVersion": 1
        });
        self.run(row);
        self.event(
            task_id,
            "subagent_spawned",
            json!({
                "agentId": agent_id,
                "parentRunId": self.request_id,
                "threadId": self.thread_id,
                "workerThreadId": worker_thread_id,
                "mode": mode,
                "dedicatedThread": dedicated_thread,
                "promptChars": prompt_chars,
                "displayName": display_name,
                "parentCallId": parent_call_id
            }),
        );
    }

    pub(super) fn child_completed(&self, done: &ChildDone<'_>) {
        let mut row = self.child_row(
            done.task_id,
            done.agent_id,
            AgentRunKind::Subagent,
            AgentRunStatus::Completed,
        );
        row.summary = Some(format!(
            "Completed in {} iteration(s), {} output chars",
            done.iterations, done.output_chars
        ));
        row.completed_at = Some(chrono::Utc::now());
        self.run(row);
        self.telemetry(RunTelemetryUpsert {
            run_id: done.task_id.to_owned(),
            elapsed_ms: Some(done.elapsed_ms),
            tool_count: done.tool_count,
            ..Default::default()
        });
        self.event(
            done.task_id,
            "subagent_completed",
            json!({
                "agentId": done.agent_id,
                "elapsedMs": done.elapsed_ms,
                "iterations": done.iterations,
                "outputChars": done.output_chars,
                "worktreePath": done.worktree_path,
                "changedFiles": done.changed_files,
                "dirtyStatus": done.dirty_status,
                "parentCallId": done.parent_call_id
            }),
        );
    }

    pub(super) fn child_failed(
        &self,
        agent_id: &str,
        task_id: &str,
        error: &str,
        parent_call_id: &Option<String>,
        tool_count: Option<u64>,
    ) {
        let mut row = self.child_row(
            task_id,
            agent_id,
            AgentRunKind::Subagent,
            AgentRunStatus::Failed,
        );
        row.error = Some(error.to_owned());
        row.completed_at = Some(chrono::Utc::now());
        self.run(row);
        self.telemetry(RunTelemetryUpsert {
            run_id: task_id.to_owned(),
            tool_count,
            error: Some(error.to_owned()),
            ..Default::default()
        });
        self.event(
            task_id,
            "subagent_failed",
            json!({ "agentId": agent_id, "error": error, "parentCallId": parent_call_id }),
        );
    }

    pub(super) fn child_awaiting_user(
        &self,
        agent_id: &str,
        task_id: &str,
        question: &str,
        worker_thread_id: &Option<String>,
        checkpoint_path: &Option<String>,
        parent_call_id: &Option<String>,
    ) {
        let kind = if worker_thread_id.is_some() {
            AgentRunKind::WorkerThread
        } else {
            AgentRunKind::Subagent
        };
        let mut row = self.child_row(task_id, agent_id, kind, AgentRunStatus::AwaitingUser);
        row.worker_thread_id.clone_from(worker_thread_id);
        // What the runner actually wrote; a rebuild from `workspace_dir` would
        // assert a checkpoint that may never have been written (#5928).
        row.checkpoint_path.clone_from(checkpoint_path);
        row.checkpoint = Some(json!({
            "resumeTool": "continue_subagent",
            "taskId": task_id,
            "agentId": agent_id,
            "question": question,
            "workerThreadId": worker_thread_id,
            "checkpointPersisted": checkpoint_path.is_some()
        }));
        row.summary = Some(question.to_owned());
        self.run(row);
        self.event(
            task_id,
            "subagent_awaiting_user",
            json!({
                "agentId": agent_id,
                "question": question,
                "workerThreadId": worker_thread_id,
                "parentCallId": parent_call_id
            }),
        );
    }

    pub(super) fn child_tool_started(
        &self,
        agent_id: &str,
        task_id: &str,
        call_id: &str,
        tool_name: &str,
        iteration: u32,
        count: u64,
    ) {
        self.telemetry(RunTelemetryUpsert {
            run_id: task_id.to_owned(),
            tool_count: Some(count),
            ..Default::default()
        });
        self.event(
            task_id,
            "subagent_tool_call_started",
            json!({
                "agentId": agent_id,
                "callId": call_id,
                "toolName": tool_name,
                "iteration": iteration
            }),
        );
    }

    pub(super) fn child_tool_completed(&self, task_id: &str, payload: serde_json::Value) {
        self.event(task_id, "subagent_tool_call_completed", payload);
    }
}
