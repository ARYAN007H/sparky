//! OpenHuman supplies plans, execution, persistence, and DTO projection;
//! TinyAgents owns subagent lifecycle coordination.

use std::{
    collections::HashMap,
    fs::OpenOptions,
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use fs2::FileExt;
use sha2::{Digest, Sha256};
use tinyagents_harness::context::{RunConfig, RunContext};
use tinyagents_orchestration::subagent::{
    ArtifactReference, PreparedSubagent, SubagentCapabilities, SubagentDriver, SubagentError,
    SubagentExecution, SubagentExecutor, SubagentIncomplete, SubagentOutcome, SubagentOutcomeKind,
    SubagentPause, SubagentPausePersistenceDisposition, SubagentPlanner, SubagentRequest,
    SubagentResume, SubagentRunResult, SubagentTaskKey, SubagentTerminalPersistenceDisposition,
};
use tinyagents_runtime::ToolSnapshot;
use tinyinference_llm::{
    message::Message,
    usage::{ChargedAmount, Usage, UsageTotals},
};
use tokio::sync::{Mutex as AsyncMutex, Notify};

#[path = "lifecycle_checkpoint.rs"]
mod checkpoint;
#[path = "lifecycle_outcome.rs"]
mod outcome;
use checkpoint::absolute_checkpoint_dir;
pub(super) use driver_adapter::outcome_to_host;
use driver_adapter::HostInvocation;
use outcome::HostInFlightMap;
pub(super) use persistence::OpenHumanPersistence;

use super::{
    SubagentRunError, SubagentRunOptions, SubagentRunOutcome, SubagentRunStatus, SubagentUsage,
};

/// Creates the detached child carrier used by OpenHuman's async delegation
/// adapters while preserving the authenticated parent lineage and detached
/// cancellation token for TinyAgents' driver.
pub(crate) fn detached_subagent_parent(
    parent: &RunContext<crate::agent::tinyagents::host::OpenHumanRunContext>,
    run_name: &str,
) -> anyhow::Result<RunContext<crate::agent::tinyagents::host::OpenHumanRunContext>> {
    let data = parent.data.detached_child();
    let cancellation = data.cancellation.clone();
    let config = RunConfig::new(format!("{run_name}-{}", uuid::Uuid::new_v4()));
    parent
        .child(config, data)
        .map(|context| context.with_cancellation(cancellation))
        .map_err(|error| anyhow::anyhow!(error.to_string()))
}

/// Runs one host subagent through a neutral driver using a real direct child.
///
/// Callers that are already inside a TinyAgents turn must use this entrypoint:
/// it derives the durable key from `parent` before constructing the child and
/// therefore preserves root, parent, thread, and cancellation lineage.
pub async fn run_subagent_with_parent(
    parent: &RunContext<crate::agent::tinyagents::host::OpenHumanRunContext>,
    definition: crate::agent_registry::host_definition::AgentDefinition,
    input: impl Into<String>,
    options: SubagentRunOptions,
) -> Result<SubagentRunOutcome, SubagentRunError> {
    // Boxed so the nested lifecycle futures stay off this poll frame (#6379).
    Box::pin(OpenHumanSubagentHost::new(definition, options).run(parent, input.into())).await
}

/// Root host entrypoint for callers that begin outside a retained live parent
/// context. It creates the one explicit root context, then drives the same
/// neutral lifecycle as nested callers.
pub async fn run_subagent(
    definition: &crate::agent_registry::host_definition::AgentDefinition,
    input: &str,
    options: SubagentRunOptions,
) -> Result<SubagentRunOutcome, SubagentRunError> {
    let mut root_data = options.run_context.clone();
    let root_config = root_data.root_run_config("subagent-host");
    let root = root_data.into_tinyagents(root_config);
    run_subagent_with_parent(&root, definition.clone(), input, options).await
}

/// Resume a durable lifecycle with its recovered original task key.  The fresh
/// child supplies live execution lineage only; all coalescing, pause lookup and
/// terminal persistence remain scoped to `original_key`.
pub async fn continue_subagent_with_parent(
    parent: &RunContext<crate::agent::tinyagents::host::OpenHumanRunContext>,
    original_key: SubagentTaskKey,
    definition: crate::agent_registry::host_definition::AgentDefinition,
    input: impl Into<String>,
    options: SubagentRunOptions,
) -> Result<SubagentRunOutcome, SubagentRunError> {
    OpenHumanSubagentHost::new(definition, options)
        .continue_with_key(parent, original_key, input.into())
        .await
}

/// Continuation counterpart for host surfaces that retain only the explicit
/// OpenHuman carrier. The original durable key is mandatory.
pub async fn continue_subagent(
    original_key: SubagentTaskKey,
    definition: &crate::agent_registry::host_definition::AgentDefinition,
    input: &str,
    options: SubagentRunOptions,
) -> Result<SubagentRunOutcome, SubagentRunError> {
    let mut root_data = options.run_context.clone();
    let root_config = root_data.root_run_config("subagent-host");
    let root = root_data.into_tinyagents(root_config);
    continue_subagent_with_parent(&root, original_key, definition.clone(), input, options).await
}

pub(super) fn merged_resume_history(mut history: Vec<Message>, input: String) -> Vec<Message> {
    if !input.is_empty() {
        history.push(Message::user(input));
    }
    history
}

/// Runs a host-prepared subagent through TinyAgents' registered driver
/// dispatcher and shared Jobs/Message registry. Product preflight and result
/// delivery stay with the caller; the generic driver owns child registration,
/// execution, control queues, and terminal lifecycle transitions.
pub(crate) async fn run_subagent_via_driver_tool(
    parent: &RunContext<crate::agent::tinyagents::host::OpenHumanRunContext>,
    definition: crate::agent_registry::host_definition::AgentDefinition,
    input: String,
    options: SubagentRunOptions,
) -> Result<(SubagentRunOutcome, String), SubagentRunError> {
    let task_id = options
        .task_id
        .clone()
        .unwrap_or_else(|| format!("sub-{}", uuid::Uuid::new_v4()));
    let mut options = options;
    options.task_id = Some(task_id.clone());
    let parent_thread_id = parent.thread_id().map(|id| id.as_str().to_owned());
    let task_key = SubagentTaskKey::from_context(parent, task_id.clone(), parent_thread_id);
    let (tool_result, bundle) = invoke_async_spawn_driver_tool(
        parent, definition, input, options, "inline", None, None, None,
    )
    .await?;
    let public_job_id = decode_driver_job_result(tool_result, &bundle, &task_key)?;
    if let Some(outcome) = bundle
        .host_outcome
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(&task_key)
    {
        return Ok((outcome, public_job_id));
    }
    if let Some(error) = bundle
        .failures
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(&task_key)
    {
        return Err(error);
    }
    Err(SubagentRunError::Provider(anyhow::anyhow!(
        "driver-backed async spawn returned without a host outcome"
    )))
}

/// Runs the configured one-shot `delegate` executor inside the shared Tiny
/// subagent job lifecycle. Its captured Tool still owns config lookup, model
/// routing, security checks and response formatting; this adapter supplies the
/// authenticated parent, cancellation, stable target identity and Jobs record.
pub(crate) async fn run_configured_delegate_via_driver_tool(
    parent: &RunContext<crate::agent::tinyagents::host::OpenHumanRunContext>,
    tool: Arc<dyn tinytools::Tool>,
    arguments: serde_json::Value,
    agent_id: String,
) -> Result<tinytools::ToolResult, SubagentRunError> {
    let task_id = format!("delegate-{}", uuid::Uuid::new_v4());
    let mut options = SubagentRunOptions::default();
    options.task_id = Some(task_id.clone());
    options.thread_id = parent.thread_id().map(|id| id.as_str().to_owned());
    options.run_context = parent.data.clone();
    let input = arguments
        .get("prompt")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let request = HostRequest {
        invocation: HostInvocation::ConfiguredTool {
            agent_id,
            tool,
            arguments,
        },
        options,
    };
    let (tool_result, bundle) =
        invoke_driver_tool_with_request(parent, request, input, "inline", None, None, None).await?;
    let task_key = SubagentTaskKey::from_context(
        parent,
        task_id,
        parent.thread_id().map(|id| id.as_str().to_owned()),
    );
    decode_driver_job_result(tool_result, &bundle, &task_key)?;
    let result = bundle
        .configured_tool_result
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(&task_key)
        .ok_or_else(|| {
            SubagentRunError::Provider(anyhow::anyhow!(
                "driver-backed configured delegate completed without a host result"
            ))
        });
    result
}

/// Continues a host-authorized pause through TinyAgents' registered Jobs
/// dispatcher, preserving the original lifecycle key and checkpoint payload.
pub(crate) async fn continue_subagent_via_driver_tool(
    parent: &RunContext<crate::agent::tinyagents::host::OpenHumanRunContext>,
    original_key: SubagentTaskKey,
    definition: crate::agent_registry::host_definition::AgentDefinition,
    input: String,
    mut options: SubagentRunOptions,
    resume: SubagentResume,
) -> Result<(SubagentRunOutcome, String), SubagentRunError> {
    options.task_id = Some(original_key.task_id.clone());
    let (tool_result, bundle) = invoke_async_spawn_driver_tool(
        parent,
        definition,
        input,
        options,
        "inline",
        None,
        None,
        Some((original_key.clone(), resume)),
    )
    .await?;
    let public_job_id = decode_driver_job_result(tool_result, &bundle, &original_key)?;
    if let Some(outcome) = bundle
        .host_outcome
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(&original_key)
    {
        return Ok((outcome, public_job_id));
    }
    if let Some(error) = bundle
        .failures
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(&original_key)
    {
        return Err(error);
    }
    Err(SubagentRunError::Provider(anyhow::anyhow!(
        "driver-backed subagent continuation returned without a host outcome"
    )))
}

/// Starts a detached host-prepared child through TinyAgents' registered
/// dispatcher. The generic tool owns detachment, job registration and terminal
/// status; the callback is reserved for OpenHuman's product-side delivery.
pub(crate) async fn spawn_async_via_driver_tool(
    parent: &RunContext<crate::agent::tinyagents::host::OpenHumanRunContext>,
    definition: crate::agent_registry::host_definition::AgentDefinition,
    input: String,
    options: SubagentRunOptions,
    completion: impl Fn(
            String,
            Result<SubagentRunOutcome, String>,
            bool,
        ) -> Pin<Box<dyn Future<Output = ()> + Send>>
        + Send
        + Sync
        + 'static,
) -> Result<String, SubagentRunError> {
    spawn_async_via_driver_tool_using_jobs(parent, definition, input, options, None, completion)
        .await
}

#[cfg(test)]
pub(crate) async fn spawn_async_via_driver_tool_with_jobs(
    parent: &RunContext<crate::agent::tinyagents::host::OpenHumanRunContext>,
    definition: crate::agent_registry::host_definition::AgentDefinition,
    input: String,
    options: SubagentRunOptions,
    jobs: tinyagents_orchestration::subagent::SubAgentJobRegistry,
    completion: impl Fn(
            String,
            Result<SubagentRunOutcome, String>,
            bool,
        ) -> Pin<Box<dyn Future<Output = ()> + Send>>
        + Send
        + Sync
        + 'static,
) -> Result<String, SubagentRunError> {
    spawn_async_via_driver_tool_using_jobs(
        parent,
        definition,
        input,
        options,
        Some(jobs),
        completion,
    )
    .await
}

async fn spawn_async_via_driver_tool_using_jobs(
    parent: &RunContext<crate::agent::tinyagents::host::OpenHumanRunContext>,
    definition: crate::agent_registry::host_definition::AgentDefinition,
    input: String,
    options: SubagentRunOptions,
    jobs: Option<tinyagents_orchestration::subagent::SubAgentJobRegistry>,
    completion: impl Fn(
            String,
            Result<SubagentRunOutcome, String>,
            bool,
        ) -> Pin<Box<dyn Future<Output = ()> + Send>>
        + Send
        + Sync
        + 'static,
) -> Result<String, SubagentRunError> {
    let task_id = options
        .task_id
        .clone()
        .unwrap_or_else(|| format!("sub-{}", uuid::Uuid::new_v4()));
    let mut options = options;
    options.task_id = Some(task_id.clone());
    let task_key = SubagentTaskKey::from_context(parent, task_id, None);
    let (tool_result, bundle) = invoke_async_spawn_driver_tool(
        parent,
        definition,
        input,
        options,
        "background",
        Some(Arc::new(completion)),
        jobs,
        None,
    )
    .await?;
    decode_driver_job_result(tool_result, &bundle, &task_key)
}

type HostCompletion = Arc<
    dyn Fn(
            String,
            Result<SubagentRunOutcome, String>,
            bool,
        ) -> Pin<Box<dyn Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

async fn invoke_async_spawn_driver_tool(
    parent: &RunContext<crate::agent::tinyagents::host::OpenHumanRunContext>,
    definition: crate::agent_registry::host_definition::AgentDefinition,
    input: String,
    options: SubagentRunOptions,
    mode: &str,
    completion: Option<HostCompletion>,
    jobs_override: Option<tinyagents_orchestration::subagent::SubAgentJobRegistry>,
    continuation: Option<(SubagentTaskKey, SubagentResume)>,
) -> Result<(tinytools::ToolResult, OpenHumanDriverBundle), SubagentRunError> {
    invoke_driver_tool_with_request(
        parent,
        HostRequest {
            invocation: HostInvocation::Agent(definition),
            options,
        },
        input,
        mode,
        completion,
        jobs_override,
        continuation,
    )
    .await
}

async fn invoke_driver_tool_with_request(
    parent: &RunContext<crate::agent::tinyagents::host::OpenHumanRunContext>,
    host_request: HostRequest,
    input: String,
    mode: &str,
    completion: Option<HostCompletion>,
    jobs_override: Option<tinyagents_orchestration::subagent::SubAgentJobRegistry>,
    continuation: Option<(SubagentTaskKey, SubagentResume)>,
) -> Result<(tinytools::ToolResult, OpenHumanDriverBundle), SubagentRunError> {
    use tinyagents_harness::tool::ToolDispatch;
    use tinyagents_orchestration::subagent::{DriverSubAgentTool, SubAgentJobRegistry};
    use tinytools::{PermissionLevel, ToolCallOptions, ToolCategory, ToolScope, ToolTimeout};

    let task_id = host_request
        .options
        .task_id
        .clone()
        .unwrap_or_else(|| format!("sub-{}", uuid::Uuid::new_v4()));
    let checkpoint_dir = absolute_checkpoint_dir(
        host_request
            .options
            .checkpoint_dir
            .clone()
            .unwrap_or_else(|| {
                parent
                    .data
                    .parent
                    .as_ref()
                    .map(|context| {
                        context
                            .workspace_dir
                            .join(".openhuman/subagent_checkpoints")
                    })
                    .unwrap_or_else(|| PathBuf::from(".openhuman/subagent_checkpoints"))
            }),
    );
    let bundle = OpenHumanDriverBundle::new(checkpoint_dir)?;
    let request_host = host_request.clone();
    let request_task_id = task_id.clone();
    let request_continuation = continuation;
    let dispatcher = DriverSubAgentTool::new(
        "spawn_async_subagent",
        "Host-prepared asynchronous subagent invocation.",
        bundle.driver.clone(),
        move |_state: &(), parent, args, _call_id| {
            let input = args
                .get("input")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let (key, child) = crate::agent::tinyagents::host::direct_subagent_child(
                parent,
                request_task_id.clone(),
                RunConfig::new(format!("subagent-{}", uuid::Uuid::new_v4())),
            )
            .map_err(|error| SubagentError::InvalidRequest(error.to_string()))?;
            debug_assert_eq!(key.task_id, request_task_id);
            let request = if let Some((original_key, resume)) = request_continuation.clone() {
                SubagentRequest::continue_with_task_key(
                    original_key,
                    child,
                    request_host.clone(),
                    input,
                    Some(resume),
                )
            } else {
                SubagentRequest::fresh_from_parent(
                    parent,
                    child,
                    request_task_id.clone(),
                    request_host.clone(),
                    input,
                    None,
                )
            };
            request.map(|request| request.with_target(request_host.agent_id().to_owned()))
        },
        |request| Some(request.task_id().to_owned()),
    );
    let dispatch_ctx = parent.data.dispatch_ctx();
    let jobs = jobs_override.unwrap_or_else(|| {
        (*crate::core::runtime::slot_for::<SubAgentJobRegistry>(&dispatch_ctx)).clone()
    });
    let parent_session = parent
        .data
        .parent
        .as_ref()
        .map(|context| context.session_id.clone())
        .unwrap_or_default();
    let thread_id = parent.thread_id().map(|id| id.as_str().to_owned());
    let owner = crate::agent::tinyagents::tool_registration::subagent_owner_key(
        &parent_session,
        thread_id.as_deref(),
    );
    let owner_scope: Arc<dyn Fn(u64) -> String + Send + Sync> = Arc::new(move |_| owner.clone());
    let mut dispatcher = dispatcher
        .with_job_registry(jobs)
        .with_owner_scope(owner_scope)
        .with_metadata(
            PermissionLevel::Execute,
            ToolScope::All,
            ToolCategory::System,
            ToolTimeout::Unbounded,
        );
    if let Some(completion) = completion {
        let callback_task_key = SubagentTaskKey::from_context(parent, task_id.clone(), None);
        let host_outcome = Arc::clone(&bundle.host_outcome);
        let failures = Arc::clone(&bundle.failures);
        let persistence = Arc::clone(&bundle.persistence);
        let fallback_agent_id = host_request.agent_id().to_owned();
        dispatcher = dispatcher.with_async_completion_handler(move |target, result| {
            let completion = Arc::clone(&completion);
            let host_outcome = Arc::clone(&host_outcome);
            let failures = Arc::clone(&failures);
            let persistence = Arc::clone(&persistence);
            let fallback_agent_id = fallback_agent_id.clone();
            let key = callback_task_key.clone();
            Box::pin(async move {
                let projected = match result {
                    Ok(result) => {
                        let cached = host_outcome
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner())
                            .remove(&key);
                        Ok(outcome_to_host(
                            result,
                            cached,
                            &persistence,
                            &key,
                            &fallback_agent_id,
                        ))
                    }
                    Err(error) => {
                        let host_error = failures
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner())
                            .remove(&key)
                            .map(|failure| failure.to_string());
                        Err(host_error.unwrap_or(error))
                    }
                };
                let emit_effects = match &projected {
                    Ok(outcome) => outcome.persistence_disposition.should_emit_host_effects(),
                    Err(_) => true,
                };
                completion(target, projected, emit_effects).await;
            })
        });
    }
    if let (Some(thread_id), Some(parent_context)) = (
        parent.thread_id().map(|id| id.as_str().to_owned()),
        parent.data.parent.as_ref(),
    ) {
        if crate::agent::orchestration::background_completions::mark_stopped_task_if_thread_stopped(
            &dispatch_ctx,
            &parent_context.workspace_dir,
            &thread_id,
            &task_id,
        ) {
            return Err(SubagentRunError::Provider(anyhow::anyhow!(
                "parent thread is stopped or deleted"
            )));
        }
    }
    let args = serde_json::json!({ "input": input, "mode": mode });
    let tool_result = dispatcher
        .execute(
            &(),
            tinyagents_harness::CallId::new(format!("spawn-{task_id}")),
            args,
            ToolCallOptions::default(),
            parent,
        )
        .await
        .map_err(|error| SubagentRunError::Provider(anyhow::anyhow!(error.to_string())))?;
    Ok((tool_result, bundle))
}

pub(crate) fn checkpoint_resume(checkpoint: &super::SubagentCheckpointData) -> SubagentResume {
    OpenHumanPersistence::checkpoint_resume(checkpoint)
}

pub(super) fn decode_driver_job_result(
    tool_result: tinytools::ToolResult,
    bundle: &OpenHumanDriverBundle,
    task_key: &SubagentTaskKey,
) -> Result<String, SubagentRunError> {
    if tool_result.is_error {
        if let Some(error) = bundle_failure_for_key(bundle, task_key) {
            return Err(error);
        }
    }
    let reply = match tool_result.content.first() {
        Some(tinytools::ToolContent::Json { data }) if !tool_result.is_error => data.clone(),
        // The generic dispatcher encodes non-success lifecycle snapshots
        // (awaiting input, incomplete, cancelled) as a JSON text error so the
        // calling model sees the stop. For this host adapter, accept only that
        // exact typed snapshot when the driver also produced the corresponding
        // host outcome. Arbitrary text errors and persistence failures remain
        // errors; in particular, a host executor outcome alone is not enough
        // to override a failed driver call.
        Some(tinytools::ToolContent::Text { text }) if tool_result.is_error => {
            let reply: serde_json::Value = serde_json::from_str(text).map_err(|_| {
                SubagentRunError::Provider(anyhow::anyhow!(format!(
                    "driver-backed async spawn failed: {}",
                    tool_result.output()
                )))
            })?;
            let expected_status = bundle
                .host_outcome
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .get(task_key)
                .and_then(|outcome| match &outcome.status {
                    SubagentRunStatus::AwaitingUser { .. } => Some("awaiting_input"),
                    SubagentRunStatus::Incomplete { .. } => Some("incomplete"),
                    SubagentRunStatus::Cancelled => Some("cancelled"),
                    SubagentRunStatus::Completed => None,
                });
            if !expected_status.is_some_and(|status| {
                reply.get("status").and_then(serde_json::Value::as_str) == Some(status)
            }) {
                return Err(SubagentRunError::Provider(anyhow::anyhow!(format!(
                    "driver-backed async spawn failed: {}",
                    tool_result.output()
                ))));
            }
            reply
        }
        _ => {
            return Err(SubagentRunError::Provider(anyhow::anyhow!(format!(
                "driver-backed async spawn returned no structured job reference: {}",
                tool_result.output()
            ))));
        }
    };
    reply
        .get("task_id")
        .and_then(serde_json::Value::as_str)
        .filter(|task_id| !task_id.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| {
            SubagentRunError::Provider(anyhow::anyhow!(
                "driver-backed async spawn omitted its registered job id"
            ))
        })
}

fn bundle_failure_for_key(
    bundle: &OpenHumanDriverBundle,
    task_key: &SubagentTaskKey,
) -> Option<SubagentRunError> {
    bundle
        .failures
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(task_key)
}

/// Recover the original durable key and complete host pause payload from the
/// safe task-id index written at the pause persistence boundary.
pub(crate) fn load_subagent_checkpoint(
    checkpoint_dir: &std::path::Path,
    task_id: &str,
) -> Result<super::SubagentCheckpointData, SubagentRunError> {
    let persistence = OpenHumanPersistence::new(checkpoint_dir.to_path_buf());
    // This reads under the full-key lifecycle lock and filters terminal state,
    // so an index left behind by terminal cleanup can never reopen a closed
    // subagent as an old pause.
    persistence.checkpoint_for_task(task_id)
}

/// Per-invocation OpenHuman adapter state.  A driver is intentionally created
/// for one caller-owned lifecycle; process-wide persistence remains the source
/// of truth for cross-process continuation and duplicate terminal effects.
pub struct OpenHumanSubagentHost {
    definition: crate::agent_registry::host_definition::AgentDefinition,
    options: SubagentRunOptions,
}

/// Process-local coalescing extends TinyAgents' per-driver reservation across
/// independently constructed host adapters. Durable persistence remains the
/// cross-process authority; this prevents duplicate local child graphs.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct HostLifecycleKey {
    checkpoint_dir: PathBuf,
    task_key: SubagentTaskKey,
}

pub(super) struct HostInFlight {
    /// `Some(Some(outcome))` is a committed leader result; `Some(None)` means
    /// the leader failed before a durable result and followers must reserve a
    /// new attempt rather than receiving a flattened host error.
    result: AsyncMutex<Option<Option<SubagentRunOutcome>>>,
    notify: Notify,
}

impl HostInFlight {
    pub(super) fn new() -> Self {
        Self {
            result: AsyncMutex::new(None),
            notify: Notify::new(),
        }
    }

    pub(super) async fn wait(
        &self,
        cancellation: &tinyagents_harness::CancellationToken,
        task_key: &SubagentTaskKey,
        agent_id: &str,
    ) -> Result<Option<SubagentRunOutcome>, SubagentRunError> {
        loop {
            let notified = self.notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if let Some(result) = self.result.lock().await.clone() {
                return Ok(result.map(host_observer_outcome));
            }
            tokio::select! {
                biased;
                _ = cancellation.cancelled() => {
                    return Ok(Some(observer_cancelled_outcome(task_key, agent_id)));
                }
                _ = &mut notified => {}
            }
        }
    }

    pub(super) async fn complete(&self, result: Option<SubagentRunOutcome>) {
        *self.result.lock().await = Some(result);
        self.notify.notify_waiters();
    }
}

fn observer_cancelled_outcome(task_key: &SubagentTaskKey, agent_id: &str) -> SubagentRunOutcome {
    SubagentRunOutcome {
        task_id: task_key.task_id.clone(),
        agent_id: agent_id.to_owned(),
        output: String::new(),
        iterations: 0,
        elapsed: std::time::Duration::default(),
        mode: super::SubagentMode::Typed,
        status: SubagentRunStatus::Cancelled,
        final_history: Vec::new(),
        usage: SubagentUsage::default(),
        artifact_paths: Vec::new(),
        persistence_disposition:
            tinyagents_orchestration::subagent::SubagentPersistenceDisposition::ObserverCancelled,
    }
}

fn host_observer_outcome(mut outcome: SubagentRunOutcome) -> SubagentRunOutcome {
    use tinyagents_orchestration::subagent::SubagentPersistenceDisposition as Disposition;

    outcome.persistence_disposition = match outcome.persistence_disposition {
        Disposition::PauseCommitted | Disposition::PauseReplaced | Disposition::PauseExisting => {
            Disposition::PauseExisting
        }
        Disposition::TerminalInserted | Disposition::TerminalExisting => {
            Disposition::TerminalExisting
        }
        Disposition::ObserverCancelled => Disposition::ObserverCancelled,
    };
    outcome
}

pub(super) fn host_in_flight(ctx: &crate::core::runtime::DispatchCtx) -> Arc<HostInFlightMap> {
    crate::core::runtime::slot_for(ctx)
}

impl OpenHumanSubagentHost {
    pub fn new(
        definition: crate::agent_registry::host_definition::AgentDefinition,
        options: SubagentRunOptions,
    ) -> Self {
        Self {
            definition,
            options,
        }
    }

    pub async fn run(
        self,
        parent: &RunContext<crate::agent::tinyagents::host::OpenHumanRunContext>,
        input: String,
    ) -> Result<SubagentRunOutcome, SubagentRunError> {
        let task_id = self
            .options
            .task_id
            .clone()
            .unwrap_or_else(|| format!("sub-{}", uuid::Uuid::new_v4()));
        let child_config = RunConfig::new(format!("subagent-{}", uuid::Uuid::new_v4()));
        let (task_key, child) = crate::agent::tinyagents::host::direct_subagent_child(
            parent,
            task_id.clone(),
            child_config,
        )
        .map_err(|error| SubagentRunError::Provider(anyhow::anyhow!(error.to_string())))?;
        Box::pin(self.run_with_request(parent, task_key, child, input, false)).await
    }

    async fn continue_with_key(
        self,
        parent: &RunContext<crate::agent::tinyagents::host::OpenHumanRunContext>,
        task_key: SubagentTaskKey,
        input: String,
    ) -> Result<SubagentRunOutcome, SubagentRunError> {
        let child_config = RunConfig::new(format!("subagent-{}", uuid::Uuid::new_v4()));
        let (_, child) = crate::agent::tinyagents::host::direct_subagent_child(
            parent,
            task_key.task_id.clone(),
            child_config,
        )
        .map_err(|error| SubagentRunError::Provider(anyhow::anyhow!(error.to_string())))?;
        Box::pin(self.run_with_request(parent, task_key, child, input, true)).await
    }

    async fn run_with_request(
        self,
        parent: &RunContext<crate::agent::tinyagents::host::OpenHumanRunContext>,
        task_key: SubagentTaskKey,
        child: RunContext<crate::agent::tinyagents::host::OpenHumanRunContext>,
        input: String,
        continuation: bool,
    ) -> Result<SubagentRunOutcome, SubagentRunError> {
        let checkpoint_dir =
            absolute_checkpoint_dir(self.options.checkpoint_dir.clone().unwrap_or_else(|| {
                parent
                    .data
                    .parent
                    .as_ref()
                    .map(|context| {
                        context
                            .workspace_dir
                            .join(".openhuman/subagent_checkpoints")
                    })
                    .unwrap_or_else(|| std::path::PathBuf::from(".openhuman/subagent_checkpoints"))
            }));
        let lifecycle_key = HostLifecycleKey {
            checkpoint_dir: checkpoint_dir.clone(),
            task_key: task_key.clone(),
        };
        let dispatch_ctx = parent.data.dispatch_ctx();
        let (entry, is_leader) = {
            let mut entries = host_in_flight(&dispatch_ctx).lock_owned().await;
            match entries.get(&lifecycle_key) {
                Some(entry) => (entry.clone(), false),
                None => {
                    let entry = Arc::new(HostInFlight::new());
                    entries.insert(lifecycle_key.clone(), entry.clone());
                    (entry, true)
                }
            }
        };
        if !is_leader {
            if let Some(outcome) = entry
                .wait(&parent.cancellation, &task_key, &self.definition.id)
                .await?
            {
                return Ok(outcome);
            }
            // Do not collapse the leader's typed error into an untyped string.
            // A follower retries the reservation and, if it becomes leader,
            // reports its own native planning/provider/policy classification.
            // The retry is boxed because the host keeps the typed leader error
            // out of the shared result. In practice it retries only after the
            // prior entry has been removed; boxing keeps that recovery path
            // from making the public host future infinitely sized.
            return Box::pin(self.run_with_request(parent, task_key, child, input, continuation))
                .await;
        }

        let result = Box::pin(self.run_leader(
            parent,
            task_key.clone(),
            child,
            input,
            continuation,
            checkpoint_dir,
        ))
        .await;
        entry.complete(result.as_ref().ok().cloned()).await;
        let mut entries = host_in_flight(&dispatch_ctx).lock_owned().await;
        if entries
            .get(&lifecycle_key)
            .is_some_and(|current| Arc::ptr_eq(current, &entry))
        {
            entries.remove(&lifecycle_key);
        }
        result
    }

    async fn run_leader(
        self,
        parent: &RunContext<crate::agent::tinyagents::host::OpenHumanRunContext>,
        task_key: SubagentTaskKey,
        child: RunContext<crate::agent::tinyagents::host::OpenHumanRunContext>,
        input: String,
        continuation: bool,
        checkpoint_dir: PathBuf,
    ) -> Result<SubagentRunOutcome, SubagentRunError> {
        let fallback_agent_id = self.definition.id.clone();
        let request = HostRequest {
            invocation: HostInvocation::Agent(self.definition),
            options: self.options,
        };
        let driver = OpenHumanDriverBundle::new(checkpoint_dir)?;
        let request = if continuation {
            SubagentRequest::continue_with_key(task_key.clone(), child, request, input, None, None)
                .await
        } else {
            // `direct_subagent_child` derives the same key for the host's
            // durable lookup; this constructor independently proves that the
            // execution context is the supplied parent's direct child.
            SubagentRequest::fresh_from_parent(
                parent,
                child,
                task_key.task_id.clone(),
                request,
                input,
                None,
            )
        }
        .map_err(map_lifecycle_error)?;
        let cancellation = parent.cancellation.clone();
        match Box::pin(driver.driver.run(request, cancellation)).await {
            Ok(result) => Ok(outcome_to_host(
                result,
                driver
                    .host_outcome
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .remove(&task_key),
                &driver.persistence,
                &task_key,
                &fallback_agent_id,
            )),
            Err(error) => {
                if let Some(host_error) = driver
                    .failures
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .remove(&task_key)
                {
                    Err(host_error)
                } else {
                    Err(map_lifecycle_error(error))
                }
            }
        }
    }
}

#[path = "lifecycle/driver_adapter.rs"]
mod driver_adapter;
/// One configured OpenHuman lifecycle driver and its request-keyed host
/// projections. Keeping this factory separate lets registered TinyAgents
/// dispatchers reuse the same planner, executor and persistence adapters
/// without routing through `OpenHumanSubagentHost::run` (which would create a
/// second generic driver).
#[path = "lifecycle/persistence.rs"]
mod persistence;

pub(super) use driver_adapter::{HostRequest, OpenHumanDriverBundle};

fn map_lifecycle_error(error: SubagentError) -> SubagentRunError {
    SubagentRunError::Provider(anyhow::anyhow!(error.to_string()))
}

/// A child run's cost as it comes back across the orchestration boundary.
/// That `Usage` can only say whether a charge is present, so a present one is
/// read as charged and an absent one as unknown (it was sent as absent).
fn cost_source_of(charged: Option<&ChargedAmount>) -> tinyagents_harness::cost::CostSource {
    match charged {
        Some(_) => tinyagents_harness::cost::CostSource::Charged,
        None => tinyagents_harness::cost::CostSource::Unknown,
    }
}
