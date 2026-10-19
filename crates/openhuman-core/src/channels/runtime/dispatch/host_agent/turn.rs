//! Running one channel turn as a host agent.

use tokio::sync::mpsc;

use super::posture;
use crate::agent::bus::AgentTurnResponse;
use crate::agent::host_agents::HostAgent;
use crate::agent::turn_origin::AgentTurnOrigin;
use tinyagents_harness::progress::AgentProgress;

/// One channel message, as a host agent's turn needs it.
pub(crate) struct HostChannelTurn {
    /// Existing trusted relay correlation used by scoped tools.
    pub(crate) relay_thread: Option<String>,
    /// The channel the message came in on (`"telegram"`).
    pub(crate) channel: String,
    /// TinyChannels' canonical conversation identity, tenant-scoped by the
    /// host before it reaches this runner.
    pub(crate) session_key: String,
    /// Stable event/approval identity retained by the existing channel
    /// surface. It is attribution, not the transcript key.
    pub(crate) event_session_id: String,
    /// The message the agent answers.
    pub(crate) message: String,
    /// Who asked: the channel's `ExternalChannel` origin.
    pub(crate) origin: AgentTurnOrigin,
    /// Live progress for the channel's draft updates.
    pub(crate) progress: Option<mpsc::Sender<AgentProgress>>,
    /// Where this turn's parked approvals surface; default for a channel
    /// without an approval surface.
    pub(crate) approval: crate::security::approval::ApprovalTurnContext,
    pub(crate) cancellation: tinyagents_harness::CancellationToken,
    pub(crate) turn_deadline: Option<std::time::Duration>,
}

/// Run `turn` as `host`: its definition and host tools, inside its context,
/// under `turn.origin` and the posture that origin calls for.
pub(crate) async fn run_host_agent_turn(
    host: HostAgent,
    turn: HostChannelTurn,
) -> anyhow::Result<AgentTurnResponse> {
    let HostChannelTurn {
        relay_thread,
        channel,
        session_key,
        event_session_id,
        message,
        origin,
        progress,
        approval,
        cancellation,
        turn_deadline,
    } = turn;
    let mut config = host.config.clone();
    let profile_ceiling = host
        .context
        .host_overrides()
        .and_then(|host| host.profile_capabilities())
        .map(|caps| caps.channel_permission(&channel));
    if let Some(ceiling) =
        profile_ceiling.filter(|level| *level > tinytools::PermissionLevel::ReadOnly)
    {
        posture::cap_channel(&mut config, &channel, ceiling);
    } else if let Some(ceiling) = posture::tool_ceiling(&origin) {
        posture::cap_channel(&mut config, &channel, ceiling);
        host.host_tools = host
            .host_tools
            .take()
            .map(|tools| posture::guard_host_tools(tools, ceiling));
    }
    let agent_id = host.definition.id.clone();
    tracing::info!(
        channel = %channel,
        agent_id = %agent_id,
        "[channels::dispatch::host_agent] running channel turn as host agent"
    );
    host.config = config;
    host.context = host
        .context
        .derive_channel_turn(relay_thread, Some(origin.clone()));
    let model = host.config.default_model.clone().unwrap_or_default();
    let text = host
        .run_external_channel_turn(
            &session_key,
            &event_session_id,
            &channel,
            &message,
            origin,
            approval,
            progress,
            cancellation,
            turn_deadline,
        )
        .await?;
    Ok(AgentTurnResponse::with_resolved_route(text, format!("agent:{agent_id}"), model))
}
