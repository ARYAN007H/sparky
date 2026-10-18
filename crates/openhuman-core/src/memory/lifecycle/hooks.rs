//! The turn-side hooks: pre-turn, post-turn and compaction.
//!
//! The agent loop calls these; each is bounded, logs instead of failing, and
//! returns nothing the turn depends on. See `crates/openhuman-core/src/agent/
//! session_host/runtime_session.rs` (pre-turn), `memory::bus` (post-turn,
//! through `ConversationTurnCommitted`) and `agent/tinyagents/
//! memory_summarizer.rs` (compaction).
//!
//! Turn indices follow the committed transcript: the user message of the
//! turn that follows `n` committed turns is `2n`, its reply `2n + 1`. They
//! are stable across retries and restarts, which is what lets TinyMemory
//! recognise a retried turn as a duplicate.

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::Serialize;
use tinymemory_api::{Error, ToolCallRef, Turn};
use tinymemory_tools::recall::SkipReason;
use tinymemory_tools::{Compaction, ContextPack, PostTurn, PreTurn, SessionStart};

use crate::config::Config;
use crate::memory::engine;
use tinymemory_tools::layout::identity::ResolvedIdentity;

use super::{agent_memory_on, jobs, log_only};

/// Opening tag of an injected pack.
pub const OPEN_TAG: &str = "<memory-context>";
/// Closing tag of an injected pack.
pub const CLOSE_TAG: &str = "</memory-context>";

/// Longest tool result line appended to a logged reply, in characters.
const MAX_TOOL_LINE_CHARS: usize = 240;

/// The user message's index for a turn that follows `committed_turns`.
#[must_use]
pub fn user_turn_index(committed_turns: usize) -> u32 {
    u32::try_from(committed_turns.saturating_mul(2)).unwrap_or(u32::MAX - 1)
}

/// The context a turn is given: one rendered pack and what it cites.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TurnPack {
    /// The block, rendered by TinyMemory; never empty.
    pub markdown: String,
    /// Its estimated tokens.
    pub tokens: usize,
    /// The ids of every item it cites, in order.
    pub refs: Vec<String>,
    /// The engine that answered.
    pub engine: String,
    /// The cited items, for the chat's memory chips.
    #[serde(skip)]
    pub citations: Vec<crate::web_chat::TurnCitation>,
    /// Set when nothing was recalled because the engine refused the whole
    /// account (a `Error` code such as `INSUFFICIENT_CREDITS`): the
    /// block is then a notice saying so, not recalled memory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<&'static str>,
}

impl TurnPack {
    fn from_pack(pack: ContextPack) -> Option<Self> {
        if pack.is_empty() {
            return None;
        }
        let citations = pack
            .sections
            .iter()
            .flat_map(|section| section.hits.iter())
            .filter(|hit| pack.refs.contains(&hit.id))
            .map(crate::web_chat::TurnCitation::from)
            .collect();
        Some(Self {
            markdown: pack.markdown,
            tokens: pack.tokens,
            refs: pack.refs.iter().map(ToString::to_string).collect(),
            engine: pack.engine,
            citations,
            refusal: None,
        })
    }

    /// The notice a turn is given when its recall was refused outright, so
    /// the model does not read an empty pack as "nothing is stored" and tell
    /// the user so (#6718). `None` for a failure that is not account-wide.
    fn refused(error: &Error, engine: String) -> Option<Self> {
        let why = match error {
            Error::InsufficientCredits(_) => {
                "the account is out of credits; topping up restores it"
            }
            Error::Unauthorized(_) => "the memory engine rejected the sign-in or API key",
            Error::Unavailable(_) => "the memory engine could not be reached",
            _ => return None,
        };
        let markdown = format!(
            "# Memory\n\nMemory could not be read for this turn: {why}. Nothing was \
             recalled, which does not mean nothing is stored. If the user refers to \
             something they told you before, say that memory is unavailable right now \
             and why, instead of saying you do not know it."
        );
        Some(Self {
            tokens: tinymemory_tools::recall::estimate_tokens(&markdown),
            markdown,
            refs: Vec::new(),
            engine,
            citations: Vec::new(),
            refusal: Some(error.code()),
        })
    }

    /// The block as the model sees it.
    #[must_use]
    pub fn injection(&self) -> String {
        format!("{OPEN_TAG}\n{}\n{CLOSE_TAG}", self.markdown.trim())
    }
}

/// One turn's memory, carried on its run context: what the compaction hook
/// needs to recall for this thread under this identity, and the pack the
/// turn was given.
#[derive(Debug, Clone)]
pub struct MemoryTurn {
    /// The config the session runs under.
    pub config: std::sync::Arc<Config>,
    /// Who the turn runs as.
    pub identity: ResolvedIdentity,
    /// The conversation thread.
    pub thread_id: String,
    /// The pack the turn was given, if any.
    pub pack: Option<TurnPack>,
    /// The first turn of the thread still verbatim in the turn's prompt: the
    /// turns before it were compacted into a checkpoint (0 when nothing was).
    /// Live: a compaction during the turn moves it forward.
    pub in_prompt_from: std::sync::Arc<super::window::InPromptFrom>,
    /// The context the turn runs under, held weakly so a finished tenant's
    /// lease is not kept alive by a lingering turn handle.
    pub core: Option<std::sync::Weak<crate::core::runtime::CoreContext>>,
}

impl MemoryTurn {
    /// The thread's turns still verbatim in the turn's prompt, which the
    /// agent's own memory reads leave out (the compacted ones stay readable).
    #[must_use]
    pub fn in_prompt(&self) -> tinymemory_api::ThreadWindow {
        tinymemory_api::ThreadWindow {
            thread_id: self.thread_id.clone(),
            from_turn: self.in_prompt_from.get(),
        }
    }

    /// The dispatch context the turn's memory work runs under: its own, else
    /// the calling task's.
    #[must_use]
    pub fn dispatch_ctx(&self) -> crate::core::runtime::DispatchCtx {
        crate::core::runtime::DispatchCtx::from_weak(self.core.as_ref())
    }
}

/// The direct lifecycle operation for bounded background workers. Unlike the
/// manual timeout wrapper, its future owns all I/O and can be cancelled.
pub(crate) async fn pre_turn_work(
    ctx: &crate::core::runtime::DispatchCtx,
    config: &Config,
    identity: &ResolvedIdentity,
    thread_id: String,
    turn_index: u32,
    user_text: String,
    in_prompt_from: u32,
    at: DateTime<Utc>,
    resumed_after_compaction: bool,
    observed_actor: Option<tinymemory_api::ObservedActor>,
) -> Option<TurnPack> {
    let logging = config.memory.conversations.enabled;
    if !logging && !identity.recall {
        return None;
    }
    let bound = match engine::resolve(config).engine() {
        Ok(bound) => bound,
        Err(error) => {
            tracing::debug!(%error, "[memory:hooks] pre_turn skipped: memory off");
            return None;
        }
    };
    let engine_id = bound.engine.descriptor().id.to_string();
    let memory = match agent_memory_on(bound.engine, config, identity) {
        Ok(memory) => memory,
        Err(error) => {
            tracing::warn!(%error, "[memory:hooks] pre_turn skipped: no agent memory");
            return None;
        }
    };
    let memory = if identity.recall {
        memory
    } else {
        let quiet = log_only(memory.policy().clone());
        memory.with_policy(quiet)
    };
    let recall = identity.recall;
    let timeout = Duration::from_millis(config.memory.recall.pre_turn_timeout_ms.max(1));
    // Runs beside the turn log and the reads (tinymemory joins all three) and
    // answers by the turn's own deadline less a margin, counted from now, so
    // the pack is still ranked and returned before the turn stops waiting.
    let date_hint = (recall && logging && config.memory.recall.date_hint).then(|| {
        let config = config.clone();
        let ctx = ctx.clone();
        let text = user_text.clone();
        let deadline =
            tokio::time::Instant::now() + timeout.saturating_sub(Duration::from_millis(100));
        async move {
            let zone = config.time_zone();
            tokio::time::timeout_at(
                deadline,
                super::date_hint::extract(&ctx, &config, &text, &zone),
            )
            .await
            .ok()
            .flatten()
        }
    });
    let pack = async move {
        // The first turn after a compaction gets one pack that leads with the
        // thread's earlier turns, under the turn's own budget and dedupe.
        let resumed = recall && resumed_after_compaction;
        let turn = async {
            if logging {
                let mut pre = PreTurn::new(&thread_id, turn_index, &user_text);
                pre.in_prompt_from = in_prompt_from;
                pre.at = Some(at);
                pre.observed_actor = observed_actor.clone();
                let context = match date_hint {
                    Some(hint) => memory.pre_turn_dated(pre, resumed, hint).await,
                    None if resumed => memory.pre_turn_resumed(pre).await,
                    None => memory.pre_turn(pre).await,
                };
                match context {
                    Ok(context) => {
                        if let Some(error) = &context.log_error {
                            tracing::warn!(
                            thread_id = %thread_id,
                                error = %error,
                                "[memory:hooks] user turn not logged"
                            );
                        }
                        Ok(context.pack)
                    }
                    Err(error) => {
                        tracing::warn!(%error, "[memory:hooks] pre_turn refused");
                        Err(error)
                    }
                }
            } else if resumed {
                memory
                    .start_session(SessionStart {
                        thread_id: Some(thread_id.clone()),
                        focus: Some(user_text.clone()),
                    })
                    .await
                    .map_err(|error| {
                        tracing::warn!(%error, "[memory:hooks] recall failed");
                        error
                    })
            } else {
                memory.recall(&user_text).await.map_err(|error| {
                    tracing::warn!(%error, "[memory:hooks] recall failed");
                    error
                })
            }
        }
        .await;
        if !recall {
            return None;
        }
        let outcomes = [turn];
        let refusal = refusal_of(&outcomes);
        outcomes
            .into_iter()
            .find_map(Result::ok)
            .and_then(TurnPack::from_pack)
            .or_else(|| {
                let error = refusal?;
                tracing::warn!(
                    code = error.code(),
                    %error,
                    "[memory:hooks] recall refused; the turn is told memory is unavailable"
                );
                TurnPack::refused(&error, engine_id)
            })
    }
    .await;
    pack
}

/// The account-wide refusal behind a recall that came back empty, if any:
/// the first refusing error, else the first refusing skipped section.
fn refusal_of(outcomes: &[std::result::Result<ContextPack, Error>]) -> Option<Error> {
    let failed = outcomes.iter().find_map(|outcome| {
        outcome
            .as_ref()
            .err()
            .filter(|error| error.is_account_wide())
            .cloned()
    });
    failed.or_else(|| {
        outcomes
            .iter()
            .filter_map(|outcome| outcome.as_ref().ok())
            .flat_map(|pack| pack.skipped.iter())
            .find_map(|skipped| account_wide_skip_reason(&skipped.reason).cloned())
    })
}

fn account_wide_skip_reason(reason: &SkipReason) -> Option<&Error> {
    match reason {
        SkipReason::Engine(error) if error.is_account_wide() => Some(error),
        SkipReason::Empty | SkipReason::Engine(_) => None,
    }
}

/// A tool call the reply made, with a one-line result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallSummary {
    /// The tool's name.
    pub name: String,
    /// The call id.
    pub id: Option<String>,
    /// The start of its result, if it produced one.
    pub result: Option<String>,
}

/// The reply as logged: its text, then one `tool → result` line per call,
/// so what a tool returned survives in memory even when the reply does not
/// repeat it.
#[must_use]
pub fn logged_reply(text: &str, calls: &[ToolCallSummary]) -> String {
    let lines: Vec<String> = calls
        .iter()
        .filter_map(|call| {
            let result = call
                .result
                .as_deref()?
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            (!result.is_empty()).then(|| {
                let result: String = result.chars().take(MAX_TOOL_LINE_CHARS).collect();
                format!("- {} → {result}", call.name)
            })
        })
        .collect();
    if lines.is_empty() {
        text.trim().to_string()
    } else {
        format!("{}\n\nTools:\n{}", text.trim(), lines.join("\n"))
    }
}

/// Logs a committed reply and queues the belief builds TinyMemory hands
/// back. Logs instead of failing.
pub async fn post_turn(
    config: &Config,
    identity: &ResolvedIdentity,
    thread_id: String,
    turn_index: u32,
    assistant_text: String,
    tool_calls: Vec<ToolCallSummary>,
    at: DateTime<Utc>,
    channel: Option<String>,
) {
    if !config.memory.conversations.enabled {
        return;
    }
    let text = logged_reply(&assistant_text, &tool_calls);
    if text.is_empty() {
        return;
    }
    let bound = match engine::resolve(config).engine() {
        Ok(bound) => bound,
        Err(_) => return,
    };
    let memory = match agent_memory_on(bound.engine, config, identity) {
        Ok(memory) => memory,
        Err(error) => {
            tracing::warn!(%error, "[memory:hooks] post_turn skipped");
            return;
        }
    };
    let mut post = PostTurn::new(&thread_id, turn_index, text);
    post.channel = channel;
    post.at = Some(at);
    post.tool_calls = tool_calls
        .iter()
        .map(|call| ToolCallRef {
            name: call.name.clone(),
            id: call.id.clone(),
        })
        .collect();
    let started = std::time::Instant::now();
    let posted = memory.post_turn(post).await;
    let elapsed_ms = started.elapsed().as_millis() as u64;
    match posted {
        Ok(report) => {
            tracing::debug!(
                thread_id = %thread_id,
                agent_id = %identity.agent_id,
                turn = turn_index,
                jobs = report.jobs.len(),
                elapsed_ms,
                "[memory:hooks] reply logged"
            );
            jobs::enqueue(config, identity.root(), report.jobs).await;
        }
        Err(error) => {
            tracing::warn!(
                thread_id = %thread_id,
                %error,
                elapsed_ms,
                "[memory:hooks] reply not logged"
            );
        }
    }
}

/// Direct compaction recall, owned and cancelled by the background worker.
pub(crate) async fn compaction_work(
    config: &Config,
    identity: &ResolvedIdentity,
    thread_id: &str,
    dropped: Vec<Turn>,
) -> Option<TurnPack> {
    if !identity.recall || dropped.is_empty() || thread_id.trim().is_empty() {
        return None;
    }
    let bound = engine::resolve(config).engine().ok()?;
    let memory = agent_memory_on(bound.engine, config, identity).ok()?;
    let count = dropped.len();
    let request = Compaction {
        thread_id: thread_id.to_string(),
        dropped,
        focus: None,
    };
    let pack = match memory.recall_for_compaction(request).await {
        Ok(pack) => TurnPack::from_pack(pack),
        Err(error) => {
            tracing::warn!(%error, "[memory:hooks] compaction recall failed");
            None
        }
    };
    tracing::debug!(
        thread_id,
        dropped = count,
        recalled = pack.is_some(),
        "[memory:hooks] compaction"
    );
    pack
}

#[cfg(test)]
#[path = "hooks_tests.rs"]
mod tests;
