//! Trusted execution boundary shared by agent entry points.
//!
//! Entry points resolve the agent, origin, approval context and lifecycle
//! before calling this module. This helper binds those trusted values to one
//! `OpenHumanSessionHost` turn without deriving authority from request metadata.

use std::{sync::Arc, time::Duration};

use tinyagents_harness::{CancellationToken, run_queue::RunQueue};
use tokio::sync::mpsc;

use crate::{
    agent::{
        queued_turn::QueuedTurn,
        session_host::OpenHumanSessionHost,
        turn_env::HostTurnEnv,
        turn_origin::{AgentTurnOrigin, with_origin},
    },
    security::approval::ApprovalTurnContext,
};

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
    pub(crate) workspace: Option<tinytools::WorkspaceDescriptor>,
    pub(crate) host_env: HostTurnEnv,
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
        host_env.cancellation = Some(request.cancellation);
        host_env.turn_deadline = request
            .turn_deadline
            .map(tinyagents_harness::turn_deadline::TurnDeadline::starting_now);
        session.set_host_env(host_env);
        if let Some(workspace) = request.workspace {
            session.set_workspace_descriptor(Some(workspace));
        }

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

