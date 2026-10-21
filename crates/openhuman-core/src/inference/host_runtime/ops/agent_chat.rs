//! Agent chat turns: the full tool-using turn and the simple no-tools variant.

use crate::agent::OpenHumanSessionHost;
use crate::config::Config;
use crate::core::Outcome;
use crate::inference::provider as providers;

mod simple;
#[path = "agent_chat/types.rs"]
mod types;
pub use simple::agent_chat_simple;
pub use types::{AgentChatEnv, AgentChatReply, AgentChatTarget};

use super::turn_guards::{
    effective_agent_chat_origin, enforce_user_prompt_or_reject, grant_turn_cwd,
    normalize_model_override, resolve_turn_cwd,
};

/// Executes a single chat turn with an AI agent.
///
/// This function initializes an agent from the provided configuration and
/// processes the input message.
///
/// # Arguments
///
/// * `config` - The configuration used to build the agent. May be updated with model/temp overrides.
/// * `message` - The user message to process.
/// * `model_override` - Optional model name to use for this call.
/// * `temperature` - Optional sampling temperature override.
/// * `cwd` - Optional per-turn working directory. When present and non-empty the
///   agent's filesystem / shell tools are rooted there for this turn only: a
///   relative path resolves inside it and an absolute path under it is
///   permitted. Absent or empty behaves exactly as before (the configured
///   `action_dir`). The override is applied to a *clone* of the config, so it
///   never leaks into concurrent turns the way a process-global would.
///
/// # Errors
///
/// Returns an error when the prompt is rejected by the injection guard, when
/// `cwd` names something that is not an accessible directory, or when building
/// or running the agent fails.
///
/// # Progress
///
/// An in-process embedder hands a
/// [`AgentProgress`](crate::progress::AgentProgress) channel through
/// [`AgentChatEnv`] (see [`agent_chat_for`]); it is attached to the agent built
/// here, so the embedder observes the turn's tool calls and deltas live instead
/// of only its final string.
///
/// # Per-call inference route
///
/// `route` names an endpoint and bearer for this call alone. It is applied to
/// `config` in memory before the agent is built — including before the `cwd`
/// clone below, so a turn that is both rooted and routed gets both — and is
/// never persisted. See
/// [`ephemeral_route`](crate::config::schema::ephemeral_route).
pub async fn agent_chat(
    config: &mut Config,
    message: &str,
    model_override: Option<String>,
    temperature: Option<f64>,
    thread_id: Option<String>,
    cwd: Option<String>,
    route: Option<crate::config::schema::EphemeralRoute>,
) -> Result<Outcome<String>, String> {
    agent_chat_for(
        config,
        AgentChatTarget::Orchestrator,
        message,
        model_override,
        temperature,
        thread_id,
        cwd,
        route,
        AgentChatEnv::default(),
    )
    .await
}

// Definitions are reusable snapshots. A turn override belongs only to the
// definition passed to this call's factory, which otherwise wins over Config.
fn definition_for_turn(
    definition: &crate::agent_registry::host_definition::AgentDefinition,
    temperature: Option<f64>,
) -> std::borrow::Cow<'_, crate::agent_registry::host_definition::AgentDefinition> {
    match temperature {
        Some(temperature) => {
            let mut scoped = definition.clone();
            scoped.temperature = temperature;
            std::borrow::Cow::Owned(scoped)
        }
        None => std::borrow::Cow::Borrowed(definition),
    }
}

fn build_turn_agent(
    ctx: &crate::core::runtime::DispatchCtx,
    config: &Config,
    target: &AgentChatTarget<'_>,
    session_id: Option<&str>,
    temperature: Option<f64>,
    turn_model_source: Option<(crate::agent::tinyagents::TurnModelSource, String)>,
) -> Result<OpenHumanSessionHost, String> {
    match target {
        AgentChatTarget::Orchestrator => OpenHumanSessionHost::from_config(ctx, config),
        AgentChatTarget::AgentId(id) => {
            log::debug!("[inference] agent_chat building agent_id={id}");
            OpenHumanSessionHost::from_config_for_agent(ctx, config, id)
        }
        AgentChatTarget::Definition {
            definition,
            host,
            host_only: true,
            ..
        } => {
            let definition = definition_for_turn(definition, temperature);
            OpenHumanSessionHost::from_config_host_only(ctx, config, &definition, *host, session_id)
        }
        AgentChatTarget::Definition {
            definition, host, ..
        } => {
            let definition = definition_for_turn(definition, temperature);
            match (host, turn_model_source) {
                (None, Some((source, model))) => {
                    OpenHumanSessionHost::from_config_with_definition_and_turn_model_source(
                        ctx,
                        config,
                        &definition,
                        source,
                        model,
                    )
                }
                (host, None) => match host {
                    Some(host) => OpenHumanSessionHost::from_config_with_host_tools(
                        ctx,
                        config,
                        &definition,
                        host,
                        session_id,
                    ),
                    None => {
                        OpenHumanSessionHost::from_config_with_definition(ctx, config, &definition)
                    }
                },
                (Some(_), Some(_)) => Err(anyhow::anyhow!(
                    "explicit turn model source cannot be combined with host tools"
                )),
            }
        }
    }
    .map_err(|e| e.to_string())
}

/// [`agent_chat`] on an explicit [`AgentChatTarget`].
///
/// Two differences from the historical `agent_chat` beyond the target:
///
/// * A non-empty `thread_id` binds the session's durable identity, so the turn
///   resumes **that conversation's** transcript exactly
///   (`ResumeMode::Session`). A fresh thread simply has nothing to resume; it
///   can no longer fall back to the agent's newest transcript from some other
///   thread.
/// * The agent is built by `target`, so a library host can run one booted
///   core with many independently defined agents.
///
/// Returned boxed and `#[inline(never)]` on purpose: other crates await this
/// (`openhuman-rpc`, `openhuman-embed`), and an `async fn` body is otherwise
/// re-instantiated inside every calling crate's state machine. Boxing here
/// keeps one copy, compiled in this crate.
#[allow(clippy::too_many_arguments)]
#[inline(never)]
pub fn agent_chat_for<'a>(
    config: &'a mut Config,
    target: AgentChatTarget<'a>,
    message: &'a str,
    model_override: Option<String>,
    temperature: Option<f64>,
    thread_id: Option<String>,
    cwd: Option<String>,
    route: Option<crate::config::schema::EphemeralRoute>,
    env: AgentChatEnv,
) -> futures::future::BoxFuture<'a, Result<Outcome<String>, String>> {
    Box::pin(async move {
        agent_chat_reply_for_inner(
            config,
            target,
            message,
            model_override,
            temperature,
            thread_id,
            cwd,
            route,
            env,
            None,
            false,
        )
        .await
        .map(AgentChatReply::into_outcome)
        .map_err(|error| error.to_string())
    })
}

/// Runs a controlled direct agent turn with the caller's selected model source.
#[allow(clippy::too_many_arguments)]
pub(crate) fn agent_chat_for_with_turn_model_source<'a>(
    config: &'a mut Config,
    target: AgentChatTarget<'a>,
    message: &'a str,
    model_override: Option<String>,
    temperature: Option<f64>,
    thread_id: Option<String>,
    cwd: Option<String>,
    route: Option<crate::config::schema::EphemeralRoute>,
    env: AgentChatEnv,
    turn_model_source: (crate::agent::tinyagents::TurnModelSource, String),
) -> futures::future::BoxFuture<'a, anyhow::Result<Outcome<String>>> {
    Box::pin(async move {
        agent_chat_reply_for_inner(
            config,
            target,
            message,
            model_override,
            temperature,
            thread_id,
            cwd,
            route,
            env,
            Some(turn_model_source),
            false,
        )
        .await
        .map(AgentChatReply::into_outcome)
    })
}

/// [`agent_chat_for`], keeping whether the turn stopped at its iteration cap.
/// The `inference.agent_chat` controller answers from this.
#[allow(clippy::too_many_arguments)]
pub async fn agent_chat_reply_for(
    config: &mut Config,
    target: AgentChatTarget<'_>,
    message: &str,
    model_override: Option<String>,
    temperature: Option<f64>,
    thread_id: Option<String>,
    cwd: Option<String>,
    route: Option<crate::config::schema::EphemeralRoute>,
    env: AgentChatEnv,
) -> Result<AgentChatReply, String> {
    agent_chat_reply_for_with_headless(
        config,
        target,
        message,
        model_override,
        temperature,
        thread_id,
        cwd,
        route,
        env,
        false,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn agent_chat_reply_for_with_headless(
    config: &mut Config,
    target: AgentChatTarget<'_>,
    message: &str,
    model_override: Option<String>,
    temperature: Option<f64>,
    thread_id: Option<String>,
    cwd: Option<String>,
    route: Option<crate::config::schema::EphemeralRoute>,
    env: AgentChatEnv,
    headless: bool,
) -> Result<AgentChatReply, String> {
    agent_chat_reply_for_inner(
        config,
        target,
        message,
        model_override,
        temperature,
        thread_id,
        cwd,
        route,
        env,
        None,
        headless,
    )
    .await
    .map_err(|error| error.to_string())
}

/// Retains typed host-policy errors for callers that need to distinguish
/// dispatch refusal from provider failure.
#[allow(clippy::too_many_arguments)]
pub async fn agent_chat_reply_for_controlled(
    config: &mut Config,
    target: AgentChatTarget<'_>,
    message: &str,
    model_override: Option<String>,
    temperature: Option<f64>,
    thread_id: Option<String>,
    cwd: Option<String>,
    route: Option<crate::config::schema::EphemeralRoute>,
    env: AgentChatEnv,
) -> anyhow::Result<AgentChatReply> {
    agent_chat_reply_for_inner(
        config,
        target,
        message,
        model_override,
        temperature,
        thread_id,
        cwd,
        route,
        env,
        None,
        false,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn agent_chat_reply_for_inner(
    config: &mut Config,
    target: AgentChatTarget<'_>,
    message: &str,
    model_override: Option<String>,
    temperature: Option<f64>,
    thread_id: Option<String>,
    cwd: Option<String>,
    route: Option<crate::config::schema::EphemeralRoute>,
    env: AgentChatEnv,
    turn_model_source: Option<(crate::agent::tinyagents::TurnModelSource, String)>,
    headless: bool,
) -> anyhow::Result<AgentChatReply> {
    match target {
        AgentChatTarget::Definition {
            untrusted_input: true,
            host_only,
            ..
        } => {
            if !host_only {
                return Err(anyhow::anyhow!(
                    "untrusted_input is only allowed on a host-only agent"
                ));
            }
            log::debug!(
                "[inference] agent_chat untrusted input on a host-only agent; guard skipped"
            );
        }
        _ => enforce_user_prompt_or_reject(message, "local_ai.ops.agent_chat")
            .map_err(anyhow::Error::msg)?,
    }

    // TAURI-RUST-RS: an upstream caller (frontend, JSON-RPC client) can pass
    // `model_override: Some("")`. See `normalize_model_override` for the
    // rationale — an empty / whitespace-only override collapses to `None`.
    if let Some(model) = normalize_model_override(model_override) {
        config.default_model = Some(model);
    }
    if let Some(temp) = temperature {
        config.default_temperature = temp;
    }
    // After the model override, because the route pins its roles to
    // `"<slug>:<model>"` and the model it uses is the one this call resolved.
    // Before the `cwd` clone below, so a rooted turn is routed too.
    if let Some(route) = route {
        crate::config::schema::ephemeral_route::apply(config, route);
    }
    // The conversation this turn runs in, normalised once: the factory below is
    // told the same id `set_thread_id` will bind, so a belt keyed on the
    // session cannot disagree with the session it is built for.
    let turn_session_id = thread_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty());
    let turn_cwd = resolve_turn_cwd(cwd).map_err(anyhow::Error::msg)?;
    // The turn's context: the host environment's bound one, else the task's.
    let turn_ctx = crate::core::runtime::DispatchCtx::from_weak(env.host.core.as_ref());
    let mut agent = match turn_cwd.as_ref() {
        // Per-turn root. Building from a config clone whose `action_dir` is the
        // requested directory is what makes this a *turn-scoped* override: the
        // session's `SecurityPolicy`, its tool registry and the builder's
        // `action_dir` are all derived from that one field, so they agree, and
        // nothing process-global is mutated (unlike `live_policy::set_action_dir`,
        // which would race concurrent turns).
        Some(root) => {
            let mut scoped = config.clone();
            scoped.action_dir = root.clone();
            // `action_dir` alone is inert for access control — grant the same
            // root so the turn's tools may actually read and write in it.
            grant_turn_cwd(&mut scoped, root);
            log::debug!(
                "[inference] agent_chat rooting turn tools at cwd={}",
                root.display()
            );
            let mut agent = build_turn_agent(
                &turn_ctx,
                &scoped,
                &target,
                turn_session_id,
                temperature,
                turn_model_source.clone(),
            )
            .map_err(anyhow::Error::msg)?;
            // Also thread it as the turn's workspace descriptor so acting tools
            // that read `ToolExecutionContext::workspace` (shell) resolve their
            // default cwd here, and so spawned sub-agents inherit the same root.
            agent.set_workspace_descriptor(Some(tinytools::WorkspaceDescriptor::new(root.clone())));
            agent
        }
        None => build_turn_agent(
            &turn_ctx,
            config,
            &target,
            turn_session_id,
            temperature,
            turn_model_source,
        )
        .map_err(anyhow::Error::msg)?,
    };
    // Thread-correct resume. `OpenHumanSessionHost::turn` would otherwise auto-load the
    // newest transcript for the agent *name*, which is another thread's
    // history whenever the same agent serves several threads (every library
    // host does exactly that). Seed from this thread's transcript when it has
    // one; when it has none, keep the turn from falling back to that autoload.
    if let Some(id) = turn_session_id {
        // Binding the thread also binds the session's durable identity, and
        // the turn resumes by that identity. Nothing here has to seed history
        // by hand, suppress an autoload, or reason about which transcript is
        // newest: one conversation resolves to one transcript, scoped to this
        // agent so a caller-supplied thread id shared by several runtime
        // agents cannot splice one agent's history into another's turn.
        agent.set_thread_id(Some(id));
        log::debug!("[inference] agent_chat bound session for thread_id={id}");
    }
    // Host attachments, staged where the provider will look for them: the
    // turn's root when it has one, the configured `action_dir` otherwise.
    // Their references lead the message, so the model reads the files first
    // and the caption after, as an upload with a caption reads.
    let staged_message;
    let message = match target {
        AgentChatTarget::Definition { attachments, .. } if !attachments.is_empty() => {
            let staging_config = match turn_cwd.as_ref() {
                Some(root) => {
                    let mut scoped = config.clone();
                    scoped.action_dir = root.clone();
                    scoped
                }
                None => config.clone(),
            };
            let scope = crate::attachments::AttachmentAccessScope {
                external_channel: false,
                workspace: turn_cwd.clone(),
            };
            let references = crate::attachments::host::stage_host(
                attachments,
                turn_session_id.unwrap_or("direct"),
                &staging_config,
                &scope,
            )
            .await?;
            log::debug!(
                "[inference] agent_chat staged {} host attachment(s)",
                attachments.len()
            );
            staged_message = format!("{references}{message}");
            staged_message.as_str()
        }
        _ => message,
    };
    if let AgentChatTarget::Definition {
        untrusted_input: true,
        ..
    } = target
    {
        agent.set_untrusted_input(true)?;
    }
    // A seeded turn replaces resume rather than adding to it.
    //
    // The three calls are one operation and the order is load-bearing:
    // `clear_history` drops the composed session so `seed_resume_from_messages`
    // -- which returns `Ok(())` and seeds NOTHING on a live one, so getting
    // this order wrong runs the turn blind rather than failing -- can take
    // effect, and the override
    // stops the runtime reloading from its own durable transcript the history
    // that was just replaced. A caller seeding from its own log means that log
    // to be the whole of what this turn has seen; leaving either of the other
    // two off would quietly reunite it with a second source.
    if let AgentChatTarget::Definition {
        seed: Some(seed), ..
    } = target
    {
        agent.clear_history();
        agent.seed_resume_from_messages(seed.to_vec(), message)?;
        agent.set_next_turn_overrides(crate::agent::session_host::TurnOverrides {
            suppress_transcript_autoload: true,
            ..Default::default()
        });
        log::debug!(
            "[inference] agent_chat seeded {} row(s); transcript autoload suppressed",
            seed.len()
        );
    }
    // The response shape rides the session explicitly (no task-local): the
    // next turn takes it into its run context.
    if let AgentChatTarget::Definition {
        shape: Some(scope), ..
    } = target
    {
        agent.set_next_turn_response_shape(Some(std::sync::Arc::clone(scope)));
    }
    // The host environment of this turn (budget, cancellation, deadline, hooks,
    // tool-belt authority) rides the session explicitly.
    let approval = env.host.approval.clone();
    if headless {
        agent.enable_headless_descendant_settlement();
    }
    let mut host_env = env.host;
    if headless && host_env.turn_deadline.is_none() {
        host_env.turn_deadline = Some(
            tinyagents_harness::turn_deadline::TurnDeadline::starting_now(
                std::time::Duration::from_secs(
                    tinyagents_harness::turn_deadline::DEFAULT_TURN_BACKSTOP_SECS,
                ),
            ),
        );
    }
    agent.set_host_env(host_env);
    if let Some(queue) = env.run_queue {
        log::debug!("[inference] agent_chat attached a run queue for mid-turn steering");
        agent.set_run_queue(Some(queue));
    }
    // Live progress for in-process embedders. `OpenHumanSessionHost::from_config` never
    // attaches a sink itself, so there is nothing to clobber here; callers that
    // set one explicitly (web chat, platform socket, flows, skills) hold their
    // own `Agent` and never reach this path.
    if let Some(tx) = env.progress {
        agent.set_on_progress(Some(tx));
    } else {
        // The turn runner streams model calls only when a progress sink is
        // attached (`streaming = on_progress.is_some()`). With none, a bare RPC
        // turn fell back to unary `invoke`, so time-to-first-token equalled the
        // full completion latency. Attach a drain-only sink so the call streams;
        // the task ends when the agent drops its sender.
        let (tx, mut rx) = tokio::sync::mpsc::channel::<crate::progress::AgentProgress>(256);
        tokio::spawn(async move { while rx.recv().await.is_some() {} });
        agent.set_on_progress(Some(tx));
        log::debug!(
            "[inference] agent_chat no progress sink; attached drain sink to stream model calls"
        );
    }
    // Direct `agent_chat` RPC — invoked by trusted clients (desktop UI,
    // operator CLI). Label as CLI so the approval gate doesn't fail
    // closed on an unlabelled call site — *unless* the caller already scoped a
    // more specific origin around this dispatch, which an in-process embedder
    // can do (a workflow node labels its turn `TrustedAutomation::Workflow`).
    // Overwriting that with `Cli` would silently discard the caller's own
    // trust statement and hand every such turn the blanket CLI allowance
    // instead of the narrower one it asked for.
    let run = crate::security::turn_origin::with_origin(
        effective_agent_chat_origin(&turn_ctx),
        agent.run_single_with_approval(message, None, approval),
    );
    // `untrusted_input` was set on the session above; the turn's run context
    // carries it to the security gate, so no extra scope is needed here.
    let run: futures::future::BoxFuture<'_, _> = Box::pin(run);
    let outcome = match target {
        AgentChatTarget::Definition {
            observer: Some(observer),
            ..
        } => {
            tinyagents_harness::observability::turn_observer::with_observer(
                std::sync::Arc::clone(observer),
                run,
            )
            .await
        }
        _ => run.await,
    };
    // Before the `?`. A turn that failed still spent what it spent, and the
    // session that counted it is about to go out of scope with the error.
    if let AgentChatTarget::Definition {
        usage: Some(sink), ..
    } = target
    {
        *sink
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = agent.last_turn_usage();
    }
    let text = outcome?;
    let hit_cap = agent.last_turn_hit_cap();
    if hit_cap {
        log::info!("[inference] agent_chat turn paused at its iteration cap; reporting hit_cap");
    }
    Ok(AgentChatReply { text, hit_cap })
}

#[cfg(test)]
#[path = "agent_chat_tests.rs"]
mod tests;
