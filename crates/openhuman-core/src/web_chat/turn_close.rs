//! The reply that closes a turn a web-turn guard stopped.
//!
//! When the idle guard or the wall-clock backstop drops a turn, the turn's own
//! loop cannot write its close: it is stuck, or was never reached. Instead of
//! a bare `turn_timeout` error, the guard closes the turn with a reply that
//! says what the turn got done and where it got stuck. The evidence is what
//! the turn recorded while it ran: the turn-state snapshot the progress
//! bridge mirrors (tool timeline and narration), plus the [`StuckPoint`] the
//! activity record saw.
//!
//! One tools-free `summarization` call writes the reply, bounded by
//! [`CLOSE_TIMEOUT`]. The provider may be what stalled, so when that call
//! fails, times out or returns nothing, a plain reply built from the same
//! evidence is used instead. Either way the turn ends in an ordinary
//! delivered reply that is saved in the thread.

use std::time::Duration;

use tinyagents_session::turn_state::types::TranscriptItem;
use tinyagents_session::turn_state::{ToolTimelineStatus, TurnState, TurnStateStore};
use tinyinference_llm::message::Message;
use tinyinference_llm::model::ModelRequest;

use crate::config::rpc as config_rpc;
use crate::inference::provider;

use super::turn_activity::StuckPoint;
use super::types::WebChatTaskResult;

/// Upper bound on the closing model call.
const CLOSE_TIMEOUT: Duration = Duration::from_secs(45);

/// Most tool rows listed in the evidence (the latest ones are kept).
const MAX_STEPS: usize = 40;

/// Most characters of a tool row's detail carried in the evidence.
const MAX_DETAIL_CHARS: usize = 160;

/// Most characters of the agent's latest narration carried in the evidence.
const MAX_NARRATION_CHARS: usize = 1_500;

const LOG_PREFIX: &str = "[web-channel:turn-close]";

const CLOSE_SYSTEM_PROMPT: &str = "You write the closing message of an AI assistant's turn \
    that was stopped before it finished. Write to the user, in the assistant's voice, in \
    plain language. Use only the evidence given; do not invent results. Keep it short, in \
    three parts: what got done, where it got stuck and why the turn was stopped, and what \
    the user can say to continue. Do not apologize at length.";

/// Why a guard stopped the turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StopCause {
    /// No progress, and nothing in flight, for this long.
    Idle(Duration),
    /// The fixed wall-clock ceiling, which the harness wind-down did not beat.
    Ceiling(Duration),
}

impl StopCause {
    fn describe(self) -> String {
        match self {
            Self::Idle(idle) => format!("nothing happened for {}", minutes(idle)),
            Self::Ceiling(limit) => format!("it hit its time limit of {}", minutes(limit)),
        }
    }
}

fn minutes(d: Duration) -> String {
    let (n, unit) = match d.as_secs() {
        secs @ 0..=59 => (secs, "second"),
        secs => (secs / 60, "minute"),
    };
    if n == 1 {
        format!("1 {unit}")
    } else {
        format!("{n} {unit}s")
    }
}

/// A turn a guard stopped, and why.
pub(crate) struct StoppedTurn<'a> {
    pub(crate) thread_id: &'a str,
    pub(crate) request_id: &'a str,
    pub(crate) cause: StopCause,
    pub(crate) stuck: StuckPoint,
}

/// What the turn recorded while it ran, as evidence lines.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Evidence {
    pub(crate) steps: Vec<String>,
    pub(crate) narration: Option<String>,
}

fn truncate(text: &str, max: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= max {
        return text.to_string();
    }
    let kept: String = text.chars().take(max).collect();
    format!("{kept}…")
}

impl Evidence {
    pub(crate) fn from_snapshot(state: Option<&TurnState>) -> Self {
        let Some(state) = state else {
            return Self::default();
        };
        let skip = state.tool_timeline.len().saturating_sub(MAX_STEPS);
        let steps = state
            .tool_timeline
            .iter()
            .skip(skip)
            .map(|entry| {
                let name = entry.display_name.as_deref().unwrap_or(&entry.name);
                let detail = entry
                    .detail
                    .as_deref()
                    .filter(|d| !d.trim().is_empty())
                    .map(|d| format!(" ({})", truncate(d, MAX_DETAIL_CHARS)))
                    .unwrap_or_default();
                match entry.status {
                    ToolTimelineStatus::Success => format!("done: {name}{detail}"),
                    ToolTimelineStatus::Running => format!("still running: {name}{detail}"),
                    ToolTimelineStatus::Error => {
                        let cause = entry
                            .failure
                            .as_ref()
                            .map(|f| format!(" — {}", f.cause_plain))
                            .unwrap_or_default();
                        format!("failed: {name}{detail}{cause}")
                    }
                }
            })
            .collect();
        let narration = state
            .transcript
            .iter()
            .rev()
            .find_map(|item| match item {
                TranscriptItem::Narration { text, .. } if !text.trim().is_empty() => {
                    Some(text.as_str())
                }
                _ => None,
            })
            .or_else(|| Some(state.streaming_text.as_str()).filter(|t| !t.trim().is_empty()))
            .map(|text| truncate(text, MAX_NARRATION_CHARS));
        Self { steps, narration }
    }

    fn render(&self) -> String {
        let mut out = String::new();
        if self.steps.is_empty() {
            out.push_str("Steps taken: none recorded.\n");
        } else {
            out.push_str("Steps taken, oldest first:\n");
            for step in &self.steps {
                out.push_str("- ");
                out.push_str(step);
                out.push('\n');
            }
        }
        if let Some(narration) = &self.narration {
            out.push_str("\nThe assistant's last message before it stopped:\n");
            out.push_str(narration);
            out.push('\n');
        }
        out
    }
}

/// The plain reply used when the closing model call is unavailable.
pub(crate) fn plain_reply(turn: &StoppedTurn<'_>, evidence: &Evidence) -> String {
    let mut out = format!(
        "I stopped this turn because {}. It got stuck {}.\n\n",
        turn.cause.describe(),
        turn.stuck.describe()
    );
    if evidence.steps.is_empty() {
        out.push_str("No steps had been recorded yet.\n");
    } else {
        out.push_str("What happened before it stopped:\n");
        for step in &evidence.steps {
            out.push_str("- ");
            out.push_str(step);
            out.push('\n');
        }
    }
    out.push_str("\nSay \"continue\" and I'll pick up from here.");
    out
}

fn close_request(turn: &StoppedTurn<'_>, evidence: &Evidence) -> ModelRequest {
    let user_prompt = format!(
        "The turn was stopped because {}.\nWhere it got stuck: {}.\n\n{}",
        turn.cause.describe(),
        turn.stuck.describe(),
        evidence.render()
    );
    provider::openhuman_backend_model::without_reasoning(
        ModelRequest::new(vec![
            Message::system(CLOSE_SYSTEM_PROMPT),
            Message::user(user_prompt),
        ])
        .with_temperature(0.2),
    )
}

/// Tests run offline: the closing model call is skipped and the plain reply
/// is used.
#[cfg(test)]
static PLAIN_ONLY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

async fn model_reply(
    config: &crate::config::Config,
    turn: &StoppedTurn<'_>,
    evidence: &Evidence,
) -> Option<String> {
    #[cfg(test)]
    if PLAIN_ONLY.load(std::sync::atomic::Ordering::SeqCst) {
        return None;
    }
    let request_id = turn.request_id;
    let (chat_model, model) =
        match provider::create_chat_model_with_model_id("summarization", config, 0.2) {
            Ok(resolved) => resolved,
            Err(error) => {
                log::warn!("{LOG_PREFIX} provider init failed request_id={request_id}: {error}");
                return None;
            }
        };
    let call = chat_model.invoke(&(), close_request(turn, evidence));
    match tokio::time::timeout(CLOSE_TIMEOUT, call).await {
        Ok(Ok(response)) => Some(response.text()).filter(|text| !text.trim().is_empty()),
        Ok(Err(error)) => {
            log::warn!(
                "{LOG_PREFIX} close call failed request_id={request_id} model={model}: {error}"
            );
            None
        }
        Err(_) => {
            log::warn!(
                "{LOG_PREFIX} close call timed out after {CLOSE_TIMEOUT:?} \
                 request_id={request_id} model={model}"
            );
            None
        }
    }
}

/// The reply that closes `turn`, or `None` when even the configuration
/// cannot be read (the caller then reports the stop as an error).
pub(crate) async fn close_reply(turn: StoppedTurn<'_>) -> Option<WebChatTaskResult> {
    let config = match config_rpc::load_config_with_timeout().await {
        Ok(config) => config,
        Err(error) => {
            log::warn!(
                "{LOG_PREFIX} config load failed request_id={}: {error}",
                turn.request_id
            );
            return None;
        }
    };
    let snapshot = TurnStateStore::new(config.workspace_dir.clone())
        .get_turn(turn.thread_id, turn.request_id)
        .unwrap_or_else(|error| {
            log::warn!(
                "{LOG_PREFIX} turn snapshot unreadable request_id={}: {error}",
                turn.request_id
            );
            None
        });
    let evidence = Evidence::from_snapshot(snapshot.as_ref());
    let reply = match model_reply(&config, &turn, &evidence).await {
        Some(reply) => reply,
        None => plain_reply(&turn, &evidence),
    };
    log::info!(
        "{LOG_PREFIX} closed stopped turn request_id={} steps={} stuck={:?}",
        turn.request_id,
        evidence.steps.len(),
        turn.stuck
    );
    Some(WebChatTaskResult {
        full_response: reply,
        citations: Vec::new(),
        usage: None,
        workspace_dir: config.workspace_dir.clone(),
        timing: None,
    })
}

#[cfg(test)]
#[path = "turn_close_tests.rs"]
mod tests;
