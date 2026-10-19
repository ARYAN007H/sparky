//! The web chat's [`TurnHost`]: session checkout and the agent run.
//!
//! The runner decides when a turn starts, may be cancelled or has overrun;
//! this port does what only OpenHuman can. [`checkout`](TurnHost::checkout)
//! resolves the turn's config and working folder and takes the thread's cached
//! session agent (or builds and cold-boot resumes one, `session.rs`).
//! [`run`](TurnHost::run) drives the agent under the turn's origin and
//! approval context, taps its progress stream for the runner (the idle guard's
//! signals, the idle record and the event projection), applies the thread's
//! budget correlation, speaks the reply when asked, and keeps the reply for
//! the runner's reply store and terminal event. [`checkin`](TurnHost::checkin)
//! returns a healthy agent to the cache, and drops a failed one so the next
//! request cold-boots from the durable transcript.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tinyagents_harness::run_queue::RunQueue;
use tinyagents_harness::turn_runner::{
    QueueDisposition, TurnHost, TurnRequest, TurnResult, TurnSession, TurnSignal, WorkTracker,
};
use tinyagents_harness::CancellationToken;
use tinyinference_llm::failure::is_empty_provider_response_text;
use tokio::sync::{mpsc, watch, Mutex as AsyncMutex};

use super::ops::{self, BudgetCorrelation};
use super::progress::{ProgressProjection, INFERENCE_HEARTBEAT_SECS};
use super::runner::{Admission, Shared};
use super::session::{
    checkin_session_agent, checkout_session_agent, normalize_model_override, CheckoutPolicy,
};
use super::surface::TurnIds;
use super::turn_activity::TurnActivity;
use super::types::WebChatTaskResult;
use super::web_errors::{
    inference_budget_exceeded_user_message, is_inference_budget_exceeded_error,
};
use crate::agent::queued_turn::QueuedPayload;
use crate::agent::turn_origin::AgentTurnOrigin;
use crate::core::events::DomainEvent;
use crate::core::runtime::DispatchCtx;
use tinyagents_harness::progress::AgentProgress;

/// How long a finished turn waits for its progress tap to forward every event
/// the turn queued before the terminal `chat_done`/`chat_error` is published.
/// Bounded: a detached sub-agent can hold a sender clone and keep the channel
/// open past the turn, and a missing `TurnCompleted` (failed turn) must not
/// stall delivery.
pub(super) const PROGRESS_DRAIN_TIMEOUT: Duration = Duration::from_secs(3);

/// Capacity of the agent's progress channel. A heavy-event sub-agent (50+
/// events per run) can overflow a smaller buffer, and the sender side uses
/// `try_send`, so an overflow silently drops events, including the ones that
/// create a sub-agent's timeline row.
const PROGRESS_CHANNEL_CAPACITY: usize = 256;

pub(super) struct WebHost {
    shared: Arc<Shared>,
}

/// Aborts a task when dropped.
struct AbortOnDrop(tokio::task::JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl WebHost {
    pub(super) fn new(shared: Arc<Shared>) -> Self {
        Self { shared }
    }

    fn ctx_of(request: &TurnRequest<QueuedPayload>) -> Result<DispatchCtx, String> {
        request
            .payload
            .ctx
            .clone()
            .ok_or_else(|| "web chat turn carries no core context".to_owned())
    }

    async fn open(
        &self,
        request: &TurnRequest<QueuedPayload>,
        policy: CheckoutPolicy,
    ) -> Result<TurnSession, String> {
        let ctx = Self::ctx_of(request)?;
        let id = request.request_id.as_str();
        let payload = &request.payload;
        // A follow-up the runner dispatches after its predecessor finished was
        // admitted when it was queued; one started directly still has its
        // admission to answer.
        if self.shared.was_queued(id) && request.disposition == QueueDisposition::Followup {
            crate::core::bus::BUS.publish(DomainEvent::RunQueueFollowupDispatched {
                thread_id: request.thread_id.clone(),
                followup_count: 1,
                item_id: Some(id.to_owned()),
                text_preview: Some(crate::agent::queued_turn::text_preview(&request.text)),
            });
        }
        self.shared.admit(id, Admission::Started);
        let session = TurnSession {
            session_id: request.thread_id.clone(),
            fingerprint: None,
        };

        // A test that parks or fails the turn does so before any session is
        // built, like a turn wedged in checkout.
        #[cfg(any(test, debug_assertions))]
        if ops::test_hooks_active().await {
            log::debug!("[web-channel][test] checkout skipped by a test hook request_id={id}");
            return Ok(session);
        }

        let mut config = ctx.load_config().await?;
        // A thread started in a chosen folder acts there, not in the global
        // `action_dir`. Applied before checkout so the session agent, its
        // security policy (which grants `action_dir` as a trusted root) and its
        // tools are all built on the thread's folder.
        let thread_dir = crate::threads::ops::thread_working_dir(
            &ctx,
            config.workspace_dir.clone(),
            &request.thread_id,
        )
        .await?;
        if let Some(dir) = &thread_dir {
            log::debug!(
                "[web-channel] thread working folder applied thread_id={}",
                request.thread_id
            );
            config.action_dir = dir.clone();
        }
        // A guard that stops this turn closes it from this turn's own config
        // and reply language, not from whatever profile is active when it fires.
        let activity = self.shared.with(id, |slot| slot.activity.clone());
        activity.remember_turn(Arc::new(config.clone()), payload.locale.clone());
        let checked = checkout_session_agent(
            &ctx,
            &config,
            &payload.client_id,
            &request.thread_id,
            normalize_model_override(payload.model_override.clone()),
            payload.temperature,
            payload.locale.as_deref(),
            policy,
        )
        .await?;

        // Stamp the resolved agent onto the metadata so the trace exporter can
        // attribute the run, and hold the streamed answer for a TinyMemes
        // treatment thread, which surfaces only the remixed reply.
        let mut metadata = payload.metadata.clone();
        metadata.agent_id = Some(checked.fingerprint.target_agent_id.clone());
        #[cfg(feature = "tinymemes")]
        {
            metadata.hold_text_stream =
                crate::tinymemes::holds_text_stream(&config, &request.thread_id);
        }
        let projection = ProgressProjection::new(
            &ctx,
            TurnIds {
                client_id: payload.client_id.clone(),
                thread_id: request.thread_id.clone(),
                request_id: id.to_owned(),
            },
            metadata,
            config.clone(),
            crate::agent::session_store::turn_states_for(&config.workspace_dir),
        )
        .await;
        self.shared.with(id, |slot| {
            slot.session = Some(checked);
            slot.fork = policy == CheckoutPolicy::Fork;
            slot.config = Some(Arc::new(config));
            slot.thread_dir = thread_dir;
            slot.projection = Some(Arc::new(AsyncMutex::new(projection)));
        });
        Ok(session)
    }

    /// The turn body: `run_single` on the checked-out agent, and everything
    /// that follows from its outcome.
    async fn run_turn(
        &self,
        request: &TurnRequest<QueuedPayload>,
        cancellation: CancellationToken,
        queue: Arc<RunQueue<TurnRequest<QueuedPayload>>>,
        progress: mpsc::Sender<TurnSignal>,
    ) -> Result<TurnResult, String> {
        let ctx = Self::ctx_of(request)?;
        let id = request.request_id.as_str();
        let thread_id = request.thread_id.as_str();
        let payload = &request.payload;
        // One turn at a time per profile: wait behind its relay and cron turns.
        let _turn = crate::profiles::turn_gate::acquire(&ctx).await;

        #[cfg(any(test, debug_assertions))]
        if let Some(outcome) = ops::run_test_hooks(&self.shared, request).await {
            return outcome;
        }

        let (config, thread_dir, checked, projection, activity) = self.shared.with(id, |slot| {
            (
                slot.config.clone(),
                slot.thread_dir.clone(),
                slot.session.take(),
                slot.projection.clone(),
                slot.activity.clone(),
            )
        });
        let (Some(config), Some(mut checked), Some(projection)) = (config, checked, projection)
        else {
            return Err("web chat turn started without a checked-out session".to_owned());
        };
        let fingerprint = checked.fingerprint.clone();
        let agent = &mut checked.agent;

        // The agent's progress goes through a tap: every event is folded into
        // the idle record and the work tracker, then handed to the runner,
        // which projects it in order.
        let (progress_tx, progress_rx) = mpsc::channel(PROGRESS_CHANNEL_CAPACITY);
        // The channel is fresh here. Record the user input even if the turn
        // fails before a committed reply can emit its final TurnContent event.
        let _ = progress_tx.try_send(AgentProgress::TurnContent {
            input: Some(request.text.clone()),
            output: None,
        });
        let (drained_tx, mut drained_rx) = watch::channel(false);
        let tap = tap_progress(&ctx, progress_rx, progress, activity, drained_tx);
        let _tap = AbortOnDrop(tap);
        let _beat = AbortOnDrop(spawn_heartbeat(&ctx, projection.clone()));

        let origin = AgentTurnOrigin::WebChat {
            thread_id: thread_id.to_owned(),
            client_id: payload.client_id.clone(),
            request_id: Some(id.to_owned()),
        };
        let approval = crate::security::approval::ApprovalTurnContext::chat(
            crate::security::approval::ApprovalChatContext {
                thread_id: thread_id.to_owned(),
                client_id: payload.client_id.clone(),
                request_id: Some(id.to_owned()),
            },
        );

        // `run_single`'s future is very large; box it so the origin scope holds
        // a pointer rather than inlining it into this frame.
        #[cfg(feature = "tinymemes")]
        let turn_started = std::time::Instant::now();
        let turn = Box::pin(crate::agent::turn_host::AgentTurnHost::run(
            agent,
            crate::agent::turn_host::AgentTurnRequest {
                message: request.text.clone(),
                thread_id: Some(thread_id.to_owned()),
                origin,
                approval,
                progress: Some(progress_tx),
                run_queue: Some(queue),
                cancellation,
                turn_deadline: request.deadline,
                // Also the turn's workspace, so attachment reads and
                // delegated sub-agents use the upload folder (#7714).
                workspace: thread_dir
                    .map(tinytools::WorkspaceDescriptor::new)
                    .map(crate::agent::turn_host::WorkspaceBinding::Set)
                    .unwrap_or(crate::agent::turn_host::WorkspaceBinding::Clear),
                host_env: Default::default(),
            },
        ));
        let outcome = turn.await;
        let result = match outcome {
            Ok(response) => {
                // A successful turn proves the thread's balance is usable, so
                // drop any stale budget-exhausted signal before it could
                // mislabel a later genuine empty response. See #3386.
                ops::clear_budget_signal(&ctx, thread_id).await;
                Ok(WebChatTaskResult {
                    full_response: response,
                    citations: crate::memory::tools::take_turn_citations(&ctx, thread_id),
                    usage: agent.take_last_turn_usage_totals(),
                    workspace_dir: config.workspace_dir.clone(),
                    timing: None,
                })
            }
            Err(err) => {
                self.budget_outcome(
                    &ctx,
                    request,
                    &fingerprint.provider_binding,
                    &config,
                    err.to_string(),
                )
                .await
            }
        };
        #[cfg(feature = "tinymemes")]
        let mut result = result;
        #[cfg(feature = "tinymemes")]
        let turn_elapsed = turn_started.elapsed();

        if let Ok(task_result) = result.as_ref() {
            speak_and_publish(&config, request, task_result).await;
        }

        // TinyMemes: remix the final reply (slang + memes) when the flag puts
        // this thread in the treatment arm. Runs after reply speech so TTS
        // reads the original wording, and skips the budget-exhausted
        // placeholder. Fails open.
        #[cfg(feature = "tinymemes")]
        if let Ok(task_result) = result.as_mut() {
            crate::tinymemes::remix_task_reply(
                &ctx,
                &config,
                thread_id,
                id,
                &request.text,
                &mut task_result.full_response,
                inference_budget_exceeded_user_message(),
                turn_elapsed,
            )
            .await;
        }

        // Test hook: wedge the turn after its run, as a stuck save would.
        #[cfg(any(test, debug_assertions))]
        if ops::TEST_PARK_AFTER_RUN.load(std::sync::atomic::Ordering::SeqCst) {
            log::debug!("[web-channel][test] parking the turn after the run request_id={id}");
            std::future::pending::<()>().await;
        }

        agent.set_on_progress(None);

        // The runner publishes the terminal event as soon as this returns. Let
        // the tap forward everything the turn queued first, so the terminal
        // cannot overtake the turn's own last tool results and narration.
        // Bounded (see `PROGRESS_DRAIN_TIMEOUT`).
        if tokio::time::timeout(PROGRESS_DRAIN_TIMEOUT, drained_rx.wait_for(|done| *done))
            .await
            .is_err()
        {
            log::warn!(
                "[web-channel] progress did not drain within {PROGRESS_DRAIN_TIMEOUT:?}; delivering anyway thread={thread_id} request_id={id}"
            );
        }
        settle_turn_snapshot(&config, request, result.is_ok());

        match result {
            Ok(mut task_result) => {
                // The projection stamps its timing once it has seen the
                // parent's `TurnCompleted`, which the drain above waited for.
                task_result.timing = projection.lock().await.timing_snapshot();
                let reply = task_result.full_response.clone();
                self.shared.with(id, |slot| slot.result = Some(task_result));
                // Only a turn that completed leaves its agent for the cache.
                // A harness error can leave its session prefix committed even
                // when the turn did not complete (an in-stream credential
                // failure after partial text); re-caching that host makes the
                // next request fail with "cannot change a session prefix after
                // a committed turn". The durable transcript stays
                // authoritative, so a failed agent is dropped and cold-boots.
                self.shared.with(id, |slot| slot.session = Some(checked));
                Ok(TurnResult {
                    reply,
                    data: serde_json::Value::Null,
                })
            }
            Err(error) => {
                log::warn!(
                    "[web-channel] dropping session agent after failed turn — next turn cold-boots from the durable transcript client={} thread={thread_id} request_id={id}",
                    payload.client_id
                );
                Err(error)
            }
        }
    }

    /// A failed turn that is really an exhausted balance becomes the
    /// actionable out-of-credits reply (#3386); anything else is the error.
    async fn budget_outcome(
        &self,
        ctx: &DispatchCtx,
        request: &TurnRequest<QueuedPayload>,
        provider_binding: &str,
        config: &crate::config::Config,
        err_message: String,
    ) -> Result<WebChatTaskResult, String> {
        let thread_id = request.thread_id.as_str();
        let is_budget = is_inference_budget_exceeded_error(&err_message);
        let is_empty = is_empty_provider_response_text(&err_message.to_lowercase());
        // Only consult the cross-turn signal for an empty response that is not
        // already a budget error: avoids taking the lock on every turn.
        let has_fresh_signal = !is_budget
            && is_empty
            && ops::has_fresh_budget_signal(ctx, thread_id, provider_binding).await;
        let placeholder = || WebChatTaskResult {
            full_response: inference_budget_exceeded_user_message().to_string(),
            citations: Vec::new(),
            usage: None,
            workspace_dir: config.workspace_dir.clone(),
            timing: None,
        };
        match ops::classify_budget_correlation(is_budget, is_empty, has_fresh_signal) {
            BudgetCorrelation::BudgetExhausted => {
                // Remember the exhaustion (scoped to this provider binding) so
                // a follow-up empty 200 on this thread+provider (the managed
                // route closes the SSE clean under credit exhaustion, with no
                // inline marker) reclassifies as budget. #3386.
                ops::record_budget_signal(ctx, thread_id, provider_binding).await;
                log::warn!(
                    "[web-channel] inference budget exhausted for client={} thread={thread_id} request_id={} error_category=budget_exhausted",
                    request.payload.client_id,
                    request.request_id
                );
                Ok(placeholder())
            }
            BudgetCorrelation::UpgradeEmptyToBudget => {
                // An empty provider response inside the budget-signal window
                // most likely failed for the same out-of-credits reason but
                // arrived as a clean empty 200 with no budget marker. #3386.
                log::warn!(
                    "[web-channel] reclassifying empty provider response as budget-exhausted from recent same-thread signal client={} thread={thread_id} request_id={} error_category=budget_exhausted_correlated",
                    request.payload.client_id,
                    request.request_id
                );
                Ok(placeholder())
            }
            BudgetCorrelation::PassThrough => Err(err_message),
        }
    }
}

/// Speak the reply when the request, the thread or the voice mode asks for it,
/// and tell a push-to-talk session what was committed.
async fn speak_and_publish(
    config: &crate::config::Config,
    request: &TurnRequest<QueuedPayload>,
    task_result: &WebChatTaskResult,
) {
    let metadata = &request.payload.metadata;
    let thread_id = request.thread_id.as_str();
    let speak_reply = super::voice_notes::should_speak_reply(
        config,
        thread_id,
        metadata.speak_reply,
        metadata.source.as_deref(),
    );
    if speak_reply && !task_result.full_response.trim().is_empty() {
        let opts = crate::voice::reply_speech::ReplySpeechOptions {
            voice_id: config
                .channel_voice
                .resolve_voice("web")
                .map(str::to_string),
            ..Default::default()
        };
        match crate::voice::reply_speech::synthesize_reply(
            config,
            &task_result.full_response,
            &opts,
        )
        .await
        {
            Ok(outcome) => {
                let stored = super::voice_notes::store_synthesized_reply(
                    &config.workspace_dir,
                    &request.request_id,
                    &outcome.value.audio_base64,
                    &outcome.value.audio_mime,
                );
                log::debug!(
                    "[web_channel] reply_speech dispatched chars={} stored={stored} client_id={} thread_id={thread_id} request_id={}",
                    task_result.full_response.len(),
                    request.payload.client_id,
                    request.request_id,
                );
            }
            Err(err) => log::warn!(
                "[web_channel] reply_speech failed: {err} client_id={} thread_id={thread_id} request_id={}",
                request.payload.client_id,
                request.request_id,
            ),
        }
    }
    if metadata.source.as_deref() == Some("ptt") {
        if let Some(session_id) = metadata.session_id {
            crate::voice::publish_ptt_transcript_committed(
                thread_id.to_string(),
                session_id,
                task_result.full_response.chars().count(),
                0,
                false,
            );
        }
    }
}

/// Settle the turn's own snapshot now the turn is over.
///
/// The projection marks the snapshot terminal on its way out, but it only
/// finishes once the terminal event has been published, and a detached
/// sub-agent may still hold a progress sender past the turn. A snapshot left
/// `Streaming` on disk would hydrate a live "Thinking..." indicator under a
/// reply that was already delivered. The turn has ended here by construction,
/// so record that; a projection that later observes `TurnCompleted` overwrites
/// it with `Completed`, terminal either way.
fn settle_turn_snapshot(
    config: &crate::config::Config,
    request: &TurnRequest<QueuedPayload>,
    completed: bool,
) {
    let lifecycle = if completed {
        tinyagents_session::turn_state::TurnLifecycle::Completed
    } else {
        tinyagents_session::turn_state::TurnLifecycle::Interrupted
    };
    let now = chrono::Utc::now().to_rfc3339();
    if let Err(err) = crate::agent::session_store::turn_states_for(&config.workspace_dir)
        .settle_turn(&request.thread_id, &request.request_id, lifecycle, &now)
    {
        log::warn!(
            "[web-channel] failed to settle turn snapshot client={} thread={} request_id={}: {err}",
            request.payload.client_id,
            request.thread_id,
            request.request_id
        );
    }
}

/// Forward the agent's progress to the runner, folding each event into the
/// idle record and the work tracker on the way. Signals `drained` once the
/// parent's `TurnCompleted` was forwarded or the stream ended.
fn tap_progress(
    ctx: &DispatchCtx,
    mut rx: mpsc::Receiver<AgentProgress>,
    progress: mpsc::Sender<TurnSignal>,
    activity: TurnActivity,
    drained: watch::Sender<bool>,
) -> tokio::task::JoinHandle<()> {
    ctx.spawn(async move {
        let mut work = WorkTracker::default();
        while let Some(event) = rx.recv().await {
            activity.observe(&event);
            let completed = matches!(event, AgentProgress::TurnCompleted { .. });
            if let Some(signal) = work.observe(&event) {
                if progress.send(signal).await.is_err() {
                    break;
                }
            }
            if progress
                .send(TurnSignal::Progress(Box::new(event)))
                .await
                .is_err()
            {
                break;
            }
            if completed {
                let _ = drained.send(true);
            }
        }
        let _ = drained.send(true);
    })
}

/// The `inference_heartbeat` liveness beat for the turn's projection.
fn spawn_heartbeat(
    ctx: &DispatchCtx,
    projection: Arc<AsyncMutex<ProgressProjection>>,
) -> tokio::task::JoinHandle<()> {
    spawn_heartbeat_with_interval(
        ctx,
        projection,
        Duration::from_secs(INFERENCE_HEARTBEAT_SECS),
    )
}

fn spawn_heartbeat_with_interval(
    ctx: &DispatchCtx,
    projection: Arc<AsyncMutex<ProgressProjection>>,
    interval: Duration,
) -> tokio::task::JoinHandle<()> {
    ctx.spawn(async move {
        let mut beat = tokio::time::interval(interval);
        // Wall-clock cadence: a slow turn must not produce a burst of catch-up
        // beats once it finally yields control.
        beat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // The first tick resolves at once; the first real beat lands one full
        // interval after the turn begins.
        beat.tick().await;
        loop {
            beat.tick().await;
            projection.lock().await.heartbeat().await;
        }
    })
}

#[async_trait]
impl TurnHost<QueuedPayload> for WebHost {
    async fn checkout(&self, request: &TurnRequest<QueuedPayload>) -> Result<TurnSession, String> {
        self.open(request, CheckoutPolicy::Exact).await
    }

    async fn checkout_fork(
        &self,
        request: &TurnRequest<QueuedPayload>,
    ) -> Result<TurnSession, String> {
        self.open(request, CheckoutPolicy::Fork).await
    }

    fn run_origin(&self, request: &TurnRequest<QueuedPayload>) -> serde_json::Value {
        serde_json::json!({
            "channel": "web",
            "thread_id": request.thread_id,
            "client_id": request.payload.client_id,
            "request_id": request.request_id,
        })
    }

    async fn run(
        &self,
        request: &TurnRequest<QueuedPayload>,
        _session: &TurnSession,
        _origin: &serde_json::Value,
        cancellation: CancellationToken,
        queue: Arc<RunQueue<TurnRequest<QueuedPayload>>>,
        progress: mpsc::Sender<TurnSignal>,
    ) -> Result<TurnResult, String> {
        self.run_turn(request, cancellation, queue, progress).await
    }

    async fn checkin(&self, request: &TurnRequest<QueuedPayload>) {
        let (session, fork) = self
            .shared
            .with(&request.request_id, |slot| (slot.session.take(), slot.fork));
        let (Some(checked), false, Some(ctx)) = (session, fork, request.payload.ctx.as_ref())
        else {
            return;
        };
        checkin_session_agent(ctx, &request.thread_id, checked.agent, checked.fingerprint).await;
    }

    async fn cancel_children(&self, request: &TurnRequest<QueuedPayload>) {
        // A scoped cancel names one turn and leaves detached sub-agents alone;
        // an unscoped Stop stops them in `cancel_chat` before it cancels the
        // turn. The guards close the turn, they do not widen a cancel.
        log::debug!(
            "[web-channel] turn stopped; detached sub-agents are left running thread_id={} request_id={}",
            request.thread_id,
            request.request_id
        );
    }
}

#[cfg(test)]
#[path = "turn_host_tests.rs"]
mod tests;
