//! Trusted execution boundary shared by agent entry points.
//!
//! Entry points resolve the agent, origin, approval context and lifecycle
//! before calling this module. This helper binds those trusted values to one
//! `OpenHumanSessionHost` turn without deriving authority from request metadata.

use std::{sync::Arc, time::Duration};

use tinyagents_harness::{run_queue::RunQueue, CancellationToken};
use tokio::sync::mpsc;

use crate::{
    agent::{
        host_agents::HostAgent,
        queued_turn::QueuedTurn,
        session_host::OpenHumanSessionHost,
        turn_env::HostTurnEnv,
        turn_origin::{with_origin, AgentTurnOrigin},
    },
    security::approval::ApprovalTurnContext,
};

/// Trusted identity and authority selected at an inbound host boundary.
///
/// This value is carried internally through queue rehydration. It is never
/// serialized into chat metadata or accepted from an RPC/model request.
#[derive(Clone)]
pub(crate) struct TrustedAgentTurn {
    pub(crate) host_agent: HostAgent,
    pub(crate) channel: String,
    pub(crate) session_key: String,
    pub(crate) event_session_id: String,
    pub(crate) origin: AgentTurnOrigin,
    pub(crate) approval: ApprovalTurnContext,
    pub(crate) relay_thread: Option<String>,
    pub(crate) imported_history: Vec<tinyagents_session::transcript::TranscriptMessage>,
}

impl std::fmt::Debug for TrustedAgentTurn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrustedAgentTurn")
            .field("agent_id", &self.host_agent.definition.id)
            .field("channel", &self.channel)
            .field("session_scoped", &!self.session_key.is_empty())
            .field("relay_thread", &self.relay_thread)
            .field("imported_history_rows", &self.imported_history.len())
            .finish_non_exhaustive()
    }
}

/// The trusted, already-routed inputs for one agent turn.
pub(crate) struct AgentTurnRequest {
    pub(crate) message: String,
    pub(crate) thread_id: Option<String>,
    pub(crate) origin: AgentTurnOrigin,
    pub(crate) approval: ApprovalTurnContext,
    pub(crate) progress: Option<mpsc::Sender<tinyagents_harness::progress::AgentProgress>>,
    pub(crate) run_queue: Option<Arc<RunQueue<QueuedTurn>>>,
    pub(crate) cancellation: CancellationToken,
    pub(crate) turn_deadline: Option<Duration>,
    pub(crate) workspace: WorkspaceBinding,
    pub(crate) host_env: HostTurnEnv,
}

/// Whether a caller leaves the session's configured workspace alone or
/// explicitly resets/sets this turn's workspace.
pub(crate) enum WorkspaceBinding {
    Preserve,
    Clear,
    Set(tinytools::WorkspaceDescriptor),
}

impl WorkspaceBinding {
    fn apply(self, current: Option<tinytools::WorkspaceDescriptor>) -> Option<tinytools::WorkspaceDescriptor> {
        match self {
            Self::Preserve => current,
            Self::Clear => None,
            Self::Set(workspace) => Some(workspace),
        }
    }
}

/// Shared agent-domain execution port used by web and external-channel hosts.
pub(crate) struct AgentTurnHost;

/// Run a turn with the caller's resolved trusted session and policy context.
///
/// Cancellation and the harness deadline are captured in `host_env`; outer
/// callers remain responsible for their own idle/backstop lifecycle. Queue,
/// workspace and progress are bound only to this turn and never reused as
/// ambient authority by the next session.
impl AgentTurnHost {
    pub(crate) async fn run(
        session: &mut OpenHumanSessionHost,
        request: AgentTurnRequest,
    ) -> anyhow::Result<String> {
        session.set_on_progress(request.progress);
        session.set_run_queue(request.run_queue);
        session.set_thread_id(request.thread_id);
        let mut host_env = request.host_env;
        host_env.approval = request.approval.clone();
        host_env.cancellation = Some(request.cancellation);
        host_env.turn_deadline = request
            .turn_deadline
            .map(tinyagents_harness::turn_deadline::TurnDeadline::starting_now);
        session.set_host_env(host_env);
        let workspace = request
            .workspace
            .apply(session.live_workspace_descriptor());
        session.set_workspace_descriptor(workspace);

        // Keep the large agent future behind a pointer while the origin scope
        // is active. The origin is explicit and independent from RPC metadata.
        Box::pin(with_origin(
            request.origin.clone(),
            session.run_single_with_approval(
                &request.message,
                Some(request.origin),
                request.approval,
            ),
        ))
        .await
    }
}

#[cfg(test)]
#[path = "turn_host_tests.rs"]
mod tests;
