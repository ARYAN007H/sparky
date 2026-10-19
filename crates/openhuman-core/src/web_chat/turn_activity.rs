//! What one web-chat turn last did, kept for the reply that closes it when a
//! guard stops it.
//!
//! The idle guard itself is TinyAgents' turn runner: it ends a turn that has
//! been **silent with nothing in flight** for the idle window. Two inputs are
//! this host's:
//!
//! * [`TurnActivity`] remembers the last step, so the stop reply can say where
//!   the turn got stuck ([`StuckPoint`]), and the config and reply locale the
//!   turn resolved, so it is written to the turn's own workspace.
//! * TinyAgents' [`tinyagents_harness::turn_runner::WorkTracker`] folds generic
//!   progress into the balanced work signals consumed by its idle guard.

use std::borrow::Cow;
use std::sync::{Arc, Mutex};

use tinyagents_harness::progress::AgentProgress;

/// Default idle window for a web-chat turn, in seconds.
pub(crate) const DEFAULT_TURN_IDLE_SECS: u64 = 900;

/// Environment override for the idle window, in seconds; `0` disables it.
pub(crate) const TURN_IDLE_ENV: &str = "OPENHUMAN_WEB_TURN_IDLE_SECS";

/// A window long enough to stand for "never" where the runner needs a value.
const IDLE_DISABLED: std::time::Duration = std::time::Duration::from_secs(10 * 365 * 24 * 3600);

/// Pure core of [`configured_idle_window`]: an absent or unparseable value
/// falls back to [`DEFAULT_TURN_IDLE_SECS`]; `0` disables the idle guard.
pub(crate) fn parse_idle_window(env_value: Option<&str>) -> std::time::Duration {
    let secs = env_value
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_TURN_IDLE_SECS);
    if secs > 0 {
        std::time::Duration::from_secs(secs)
    } else {
        IDLE_DISABLED
    }
}

/// The idle window in force for this process (env [`TURN_IDLE_ENV`]).
pub(crate) fn configured_idle_window() -> std::time::Duration {
    parse_idle_window(std::env::var(TURN_IDLE_ENV).ok().as_deref())
}

#[derive(Debug, Default)]
struct State {
    /// What the last event said the turn did, for the stop reply.
    last_step: Option<Cow<'static, str>>,
    turn_completed: bool,
    /// The config and reply locale the turn itself resolved.
    turn_config: Option<Arc<crate::config::Config>>,
    locale: Option<String>,
}

/// Where a turn was when it went silent, for the reply that closes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StuckPoint {
    /// No progress event at all: the turn never got past session setup.
    BeforeRun,
    /// The agent loop was between steps; `after` names its last step.
    BetweenSteps { after: String },
    /// The agent loop had finished; the reply was being saved or delivered.
    AfterRun,
}

impl StuckPoint {
    /// One plain-language clause naming where the turn stopped.
    pub(crate) fn describe(&self) -> String {
        match self {
            Self::BeforeRun => {
                "before the agent started: the turn never got past setting up the chat \
                 session"
                    .to_string()
            }
            Self::BetweenSteps { after } => {
                format!("between steps: {after}, and the agent's next step never started")
            }
            Self::AfterRun => {
                "after the agent finished, while its reply was being saved".to_string()
            }
        }
    }
}

/// The step an event reports, as a clause for [`StuckPoint::BetweenSteps`].
/// `None` for bookkeeping (cost, usage, captured content) that says nothing
/// about what the turn was doing.
fn step_of(event: &AgentProgress) -> Option<Cow<'static, str>> {
    Some(match event {
        AgentProgress::TurnStarted => "the turn started".into(),
        AgentProgress::IterationStarted { .. } => "a model call started".into(),
        AgentProgress::ModelCallCompleted {
            subagent_task_id: None,
            ..
        } => "the model replied".into(),
        AgentProgress::ToolCallStarted { tool_name, .. } => {
            format!("the `{tool_name}` tool started").into()
        }
        AgentProgress::ToolCallCompleted {
            tool_name, success, ..
        } => {
            let outcome = if *success { "finished" } else { "failed" };
            format!("the `{tool_name}` tool {outcome}").into()
        }
        AgentProgress::TextDelta { .. } | AgentProgress::ThinkingDelta { .. } => {
            "the model was writing".into()
        }
        AgentProgress::ToolCallProgress { .. } => "a tool reported progress".into(),
        AgentProgress::SubagentSpawned { .. }
        | AgentProgress::SubagentCompleted { .. }
        | AgentProgress::SubagentFailed { .. }
        | AgentProgress::SubagentAwaitingUser { .. } => "a sub-agent reported progress".into(),
        _ => return None,
    })
}

/// Shared activity record for one turn. Cheap to clone; every clone sees the
/// same state.
#[derive(Clone, Debug, Default)]
pub(crate) struct TurnActivity {
    state: Arc<Mutex<State>>,
}

impl TurnActivity {
    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Fold one progress event into the record.
    pub(crate) fn observe(&self, event: &AgentProgress) {
        let mut state = self.lock();
        if let Some(step) = step_of(event) {
            state.last_step = Some(step);
        }
        if matches!(event, AgentProgress::TurnCompleted { .. }) {
            state.turn_completed = true;
        }
    }

    /// Record the config and reply locale the turn resolved. A stop reply
    /// uses these rather than whatever profile is active when it fires.
    pub(crate) fn remember_turn(&self, config: Arc<crate::config::Config>, locale: Option<String>) {
        let mut state = self.lock();
        state.turn_config = Some(config);
        state.locale = locale;
    }

    /// The config and locale recorded by [`Self::remember_turn`], if the turn
    /// got that far.
    pub(crate) fn turn_context(&self) -> (Option<Arc<crate::config::Config>>, Option<String>) {
        let state = self.lock();
        (state.turn_config.clone(), state.locale.clone())
    }

    /// Where the turn was when its last event arrived.
    pub(crate) fn stuck_point(&self) -> StuckPoint {
        let state = self.lock();
        if state.turn_completed {
            return StuckPoint::AfterRun;
        }
        match &state.last_step {
            None => StuckPoint::BeforeRun,
            Some(step) => StuckPoint::BetweenSteps {
                after: step.to_string(),
            },
        }
    }
}

#[cfg(test)]
#[path = "turn_activity_tests.rs"]
mod tests;
