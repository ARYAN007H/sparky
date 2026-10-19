//! TinyBus `CapsHost` service for one request-scoped flow binding.

use std::sync::Arc;

use serde_json::{Value, json};
use tinyflows_bus::runs;

use crate::security::credentials::HttpCredentialsStore;

use super::binding::CapsHostBinding;
use super::{
    OpenHumanAgentRunner, OpenHumanCode, OpenHumanHttp, OpenHumanLlm, OpenHumanMemory,
    OpenHumanTools,
};

pub(crate) struct CapsHostService(Arc<CapsHostBinding>);

impl CapsHostService {
    pub(crate) fn new(binding: Arc<CapsHostBinding>) -> Self {
        Self(binding)
    }

    async fn run_agent(
        binding: Arc<CapsHostBinding>,
        request: runs::AgentRunRequest,
    ) -> Result<runs::AgentRunOutcome, String> {
        let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel(128);
        let state = Arc::new(tokio::sync::Mutex::new(super::agent::AgentRunState::default()));
        let progress_state = Arc::clone(&state);
        let progress = tokio::spawn(async move {
            while let Some(event) = progress_rx.recv().await {
                if let crate::agent::progress::AgentProgress::TurnCompleted {
                    stop: Some(stop),
                    ..
                } = &event
                {
                    progress_state.lock().await.turn_stop = Some(stop.clone());
                }
                let Some((kind, text)) = progress_event(event) else {
                    continue;
                };
                let mut state = progress_state.lock().await;
                if state.events.len() >= runs::MAX_AGENT_TRANSCRIPT_EVENTS {
                    state.error = Some("agent progress event limit exceeded".to_owned());
                    break;
                }
                let seq = u64::try_from(state.events.len() + 1).unwrap_or(u64::MAX);
                let at_ms = epoch_millis();
                state.events.push(runs::AgentProgressEvent {
                    seq,
                    at_ms,
                    kind,
                    text: bounded_event_text(text),
                });
            }
        });

        let mut config = request.config.clone();
        if let Some(object) = config.as_object_mut()
            && object.get("timeout_secs").and_then(Value::as_u64).is_none()
        {
            if let Some(timeout) = request.agent.limits.agent_timeout_secs {
                object.insert("timeout_secs".to_owned(), Value::from(timeout));
            }
        }
        if let (Some(model), Some(object)) = (
            request
                .model
                .model
                .as_ref()
                .or(request.agent.model.as_ref()),
            config.as_object_mut(),
        ) && object.get("model").and_then(Value::as_str).is_none()
        {
            object.insert("model".to_owned(), Value::String(model.clone()));
        }
        let agent_ref = request.agent.id.clone();
        let connection = request.connection_ref.clone();
        let runner = OpenHumanAgentRunner {
            config: Arc::clone(&binding.config),
            ctx: binding.ctx.clone(),
            approval: binding.approval.clone(),
            progress: Some(progress_tx),
        };
        let result = binding
            .within(async move {
                runner
                    .run_agent(&agent_ref, config, connection.as_deref())
                    .await
            })
            .await;
        let _ = progress.await;

        let mut state = state.lock().await;
        state.done = true;
        match result {
            Ok(raw) => {
                let turn_stop = state.turn_stop.clone();
                let stop = match turn_stop.as_ref() {
                    None => runs::StopReason::Finished,
                    Some(stop)
                        if stop.kind
                            == tinyagents_harness::turn_stop::TurnStopKind::IterationCap =>
                    {
                        runs::StopReason::LimitStop {
                            limit: "max_steps".to_owned(),
                        }
                    }
                    Some(stop)
                        if stop.kind == tinyagents_harness::turn_stop::TurnStopKind::WindDown =>
                    {
                        runs::StopReason::Paused {
                            token: None,
                            reason: "wall_clock".to_owned(),
                            payload: json!({
                                "kind": stop.kind.as_str(),
                                "failure_class": stop.failure_class.as_ref().map(|class| class.as_str()),
                                "operation": stop.operation.as_deref(),
                            }),
                        }
                    }
                    Some(stop) => {
                        let class = stop
                            .failure_class
                            .as_ref()
                            .map(|class| class.as_str())
                            .unwrap_or("unknown");
                        let operation = stop.operation.as_deref().unwrap_or("unknown");
                        let error = format!(
                            "agent run stopped by classified failure ({class}, {operation})"
                        );
                        state.error = Some(error.clone());
                        return Err(error);
                    }
                };
                let text = raw
                    .as_str()
                    .map(str::to_owned)
                    .or_else(|| raw.get("text").and_then(Value::as_str).map(str::to_owned));
                let json = matches!(raw, Value::Object(_) | Value::Array(_))
                    .then(|| raw.clone())
                    .unwrap_or(Value::Null);
                let transcript = state
                    .events
                    .iter()
                    .map(|event| runs::TranscriptEntry {
                        at_ms: event.at_ms,
                        kind: event.kind.clone(),
                        text: event.text.clone(),
                    })
                    .collect();
                state.outcome = Some(runs::AgentRunOutcome {
                    stop,
                    text,
                    json,
                    raw,
                    usage: None,
                    transcript,
                });
                Ok(state.outcome.clone().expect("outcome was set"))
            }
            Err(error) => {
                let error = error.to_string();
                state.error = Some(error.clone());
                Err(error)
            }
        }
    }
}

#[tinybus::interface(name = "ai.tinyhumans.tinyflows.CapsHost")]
impl CapsHostService {
    async fn llm_complete(&self, request: Value, conn: Option<String>) -> tinybus::Result<Value> {
        let binding = Arc::clone(&self.0);
        let scope = Arc::clone(&binding);
        scope
            .within(async move {
                OpenHumanLlm {
                    ctx: binding.ctx.clone(),
                    config: Arc::clone(&binding.config),
                }
                .complete(request, conn.as_deref())
                .await
            })
            .await
            .map_err(bus_error)
    }

    async fn tool_invoke(
        &self,
        slug: String,
        args: Value,
        conn: Option<String>,
    ) -> tinybus::Result<Value> {
        let binding = Arc::clone(&self.0);
        let scope = Arc::clone(&binding);
        scope
            .within(async move {
                OpenHumanTools {
                    ctx: binding.ctx.clone(),
                    config: Arc::clone(&binding.config),
                    security: Arc::clone(&binding.security),
                    approval: binding.approval.clone(),
                }
                .invoke(&slug, args, conn.as_deref())
                .await
            })
            .await
            .map_err(bus_error)
    }

    async fn http_request(&self, request: Value, conn: Option<String>) -> tinybus::Result<Value> {
        let binding = Arc::clone(&self.0);
        let scope = Arc::clone(&binding);
        scope
            .within(async move {
                OpenHumanHttp {
                    ctx: binding.ctx.clone(),
                    config: Arc::clone(&binding.config),
                    security: Arc::clone(&binding.security),
                    http_config: binding.config.http_request.clone(),
                    http_creds: Arc::new(HttpCredentialsStore::from_config(&binding.config)),
                    approval: binding.approval.clone(),
                }
                .request(request, conn.as_deref())
                .await
            })
            .await
            .map_err(bus_error)
    }

    async fn code_run(&self, request: runs::CodeRunRequest) -> tinybus::Result<Value> {
        let binding = Arc::clone(&self.0);
        let scope = Arc::clone(&binding);
        scope
            .within(async move {
                OpenHumanCode {
                    config: Arc::clone(&binding.config),
                    ctx: binding.ctx.clone(),
                    security: Arc::clone(&binding.security),
                    approval: binding.approval.clone(),
                }
                .run(request.language, &request.source, request.input)
                .await
            })
            .await
            .map_err(bus_error)
    }

    async fn shell_run(&self, _request: runs::ShellRunRequest) -> tinybus::Result<runs::ShellRunOutcome> {
        Err(tinybus::Error::failed(
            "shell execution is disabled for workflow runs",
        ))
    }

    async fn memory_recall(
        &self,
        scope: String,
        query: String,
        opts: Value,
    ) -> tinybus::Result<Value> {
        let binding = Arc::clone(&self.0);
        let scope = Arc::clone(&binding);
        scope
            .within(async move { memory(&binding).recall(&scope, &query, opts).await })
            .await
            .map_err(bus_error)
    }

    async fn memory_flavour(&self, slug: String) -> tinybus::Result<Value> {
        let binding = Arc::clone(&self.0);
        let scope = Arc::clone(&binding);
        scope
            .within(async move { memory(&binding).flavour(&slug).await })
            .await
            .map_err(bus_error)
    }

    async fn memory_people(&self, query: Option<String>) -> tinybus::Result<Value> {
        let binding = Arc::clone(&self.0);
        let scope = Arc::clone(&binding);
        scope
            .within(async move { memory(&binding).people(query.as_deref()).await })
            .await
            .map_err(bus_error)
    }

    async fn memory_remember(
        &self,
        scope: String,
        key: String,
        value: Value,
    ) -> tinybus::Result<()> {
        let binding = Arc::clone(&self.0);
        let scope = Arc::clone(&binding);
        scope
            .within(async move { memory(&binding).remember(&scope, &key, value).await })
            .await
            .map_err(bus_error)
    }

    async fn memory_forget(&self, scope: String, key: String) -> tinybus::Result<()> {
        let binding = Arc::clone(&self.0);
        let scope = Arc::clone(&binding);
        scope
            .within(async move { memory(&binding).forget(&scope, &key).await })
            .await
            .map_err(bus_error)
    }

    async fn agent_run(&self, request: runs::AgentRunRequest) -> tinybus::Result<runs::AgentRunHandle> {
        let handle = uuid::Uuid::new_v4().simple().to_string();
        let state = Arc::new(tokio::sync::Mutex::new(super::agent::AgentRunState::default()));
        self.0
            .agent_runs
            .lock()
            .await
            .insert(handle.clone(), state);
        let binding = Arc::clone(&self.0);
        tokio::spawn(async move {
            if let Err(error) = Self::run_agent(binding.clone(), request).await {
                if let Some(state) = binding.agent_runs.lock().await.get(&handle).cloned() {
                    let mut state = state.lock().await;
                    state.error = Some(error);
                    state.done = true;
                }
            }
        });
        Ok(runs::AgentRunHandle { handle })
    }

    async fn agent_events(
        &self,
        request: runs::AgentEventsRequest,
    ) -> tinybus::Result<runs::AgentEventsResponse> {
        let state = self
            .0
            .agent_runs
            .lock()
            .await
            .get(&request.handle)
            .cloned()
            .ok_or_else(|| tinybus::Error::failed("unknown agent run handle"))?;
        let state = state.lock().await;
        if let Some(error) = &state.error {
            return Err(tinybus::Error::failed(error));
        }
        Ok(runs::AgentEventsResponse {
            events: state
                .events
                .iter()
                .filter(|event| event.seq > request.cursor)
                .take(runs::MAX_AGENT_EVENTS_PAGE)
                .cloned()
                .collect(),
            done: state.done,
            outcome: state.done.then(|| state.outcome.clone()).flatten(),
        })
    }

    async fn resolve_agent(&self, id: String) -> tinybus::Result<Option<runs::AgentDefinition>> {
        let binding = Arc::clone(&self.0);
        let scope = Arc::clone(&binding);
        scope
            .within(async move {
                crate::agent::harness::host_definition::AgentDefinitionRegistry::init_global(
                    &binding.config.workspace_dir,
                )
                .map_err(|error| error.to_string())?;
                let registry = crate::agent::harness::host_definition::AgentDefinitionRegistry::current();
                registry
                    .and_then(|registry| registry.get(&id).cloned())
                    .map(to_bus_agent_definition)
                    .transpose()
            })
            .await
            .map_err(bus_error)
    }

    async fn list_agents(&self) -> tinybus::Result<Vec<runs::AgentDefinition>> {
        let binding = Arc::clone(&self.0);
        let scope = Arc::clone(&binding);
        scope
            .within(async move {
                crate::agent::harness::host_definition::AgentDefinitionRegistry::init_global(
                    &binding.config.workspace_dir,
                )
                .map_err(|error| error.to_string())?;
                let registry = crate::agent::harness::host_definition::AgentDefinitionRegistry::current();
                registry
                    .map(|registry| registry.list().into_iter().cloned().map(to_bus_agent_definition).collect())
                    .unwrap_or_else(|| Ok(Vec::new()))
            })
            .await
            .map_err(bus_error)
    }

    async fn resolve_context(
        &self,
        _request: runs::ResolveContextRequest,
    ) -> tinybus::Result<Option<runs::ContextBlock>> {
        Ok(None)
    }

    async fn resolve_tools(
        &self,
        _request: runs::ResolveToolsRequest,
    ) -> tinybus::Result<Vec<runs::ToolDescriptor>> {
        Ok(Vec::new())
    }

    async fn resolve_workdir(
        &self,
        _request: runs::ResolveWorkdirRequest,
    ) -> tinybus::Result<runs::WorkdirCheck> {
        Ok(runs::WorkdirCheck::Unmanaged)
    }

    async fn approval_decide(
        &self,
        request: runs::ApprovalRequest,
    ) -> tinybus::Result<runs::ApprovalOutcome> {
        if request
            .run_id
            .as_deref()
            .is_some_and(|run_id| run_id != self.0.run_id.as_str())
        {
            return Err(tinybus::Error::failed(
                "workflow approval request does not match its bound run",
            ));
        }
        // The module owns graph approval rows and projects them through the
        // durable run record. Repeated calls intentionally return Pending;
        // the module retries this request after `Runs.ResumeRun` carries the
        // human's node decision. Acting-tool approvals remain on ApprovalGate.
        Ok(runs::ApprovalOutcome::Pending)
    }

    async fn approval_cancel(
        &self,
        _request: runs::ApprovalCancelRequest,
    ) -> tinybus::Result<()> {
        // Graph approval rows are module-owned; cancellation is represented
        // by the module's run lifecycle and does not cancel tool approvals.
        Ok(())
    }
}

fn memory(binding: &CapsHostBinding) -> OpenHumanMemory {
    OpenHumanMemory {
        ctx: binding.ctx.clone(),
        config: Arc::clone(&binding.config),
        security: Arc::clone(&binding.security),
        approval: binding.approval.clone(),
    }
}

fn bus_error(error: impl std::fmt::Display) -> tinybus::Error {
    tinybus::Error::failed(error.to_string())
}

fn to_bus_agent_definition(
    definition: crate::agent::harness::host_definition::AgentDefinition,
) -> Result<runs::AgentDefinition, String> {
    use tinyagents_definition::{ModelSpec, PromptSource, ToolScope};

    let instructions = match &definition.system_prompt {
        PromptSource::Inline(prompt) | PromptSource::Verbatim(prompt) => {
            Some(prompt.clone())
        }
        PromptSource::File { .. } | PromptSource::Dynamic(()) => None,
    };
    let (model, provider) = match &definition.model {
        ModelSpec::Inherit => (None, None),
        ModelSpec::Exact(model) => (Some(model.clone()), None),
        ModelSpec::Hint(hint) => (Some(format!("hint:{hint}")), None),
    };
    let tools = match &definition.tools {
        ToolScope::Wildcard => Vec::new(),
        ToolScope::Named(names) => names
            .iter()
            .map(|slug| runs::ToolGrant {
                slug: slug.clone(),
                connection_ref: None,
            })
            .collect(),
    };
    Ok(runs::AgentDefinition {
        id: definition.id.clone(),
        name: definition
            .display_name
            .clone()
            .unwrap_or_else(|| definition.id.clone()),
        description: definition.when_to_use.clone(),
        instructions,
        model,
        provider,
        working_dir: None,
        limits: runs::AgentLimits {
            max_steps: Some(u32::try_from(definition.max_iterations).unwrap_or(u32::MAX)),
            max_tool_calls: None,
            agent_timeout_secs: definition.timeout_secs.map(|secs| u32::try_from(secs).unwrap_or(u32::MAX)),
            tool_timeout_secs: None,
        },
        tools,
        context: Vec::new(),
        metadata: Default::default(),
    })
}

fn progress_event(
    event: crate::agent::progress::AgentProgress,
) -> Option<(String, String)> {
    use crate::agent::progress::AgentProgress;
    match event {
        AgentProgress::TurnStarted => Some(("turn_started".into(), "Agent turn started".into())),
        AgentProgress::IterationStarted { iteration, .. } => Some((
            "iteration_started".into(),
            format!("Iteration {iteration}"),
        )),
        AgentProgress::ToolCallStarted { tool_name, .. } => Some((
            "tool_call_started".into(),
            format!("Calling {tool_name}"),
        )),
        AgentProgress::ToolCallCompleted {
            tool_name, success, ..
        } => Some((
            "tool_call_completed".into(),
            format!("{tool_name} {}", if success { "completed" } else { "failed" }),
        )),
        AgentProgress::TextDelta { delta, .. } if !delta.is_empty() => {
            Some(("text_delta".into(), delta))
        }
        AgentProgress::TurnCompleted { stop, iterations } => Some((
            "turn_completed".into(),
            stop.map_or_else(
                || format!("Turn completed after {iterations} iterations"),
                |stop| format!("Turn stopped: {}", stop.kind.as_str()),
            ),
        )),
        _ => None,
    }
}

fn epoch_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or_default()
}

fn bounded_event_text(text: String) -> String {
    const MAX_CHARS: usize = 4_096;
    if text.chars().count() <= MAX_CHARS {
        return text;
    }
    let mut bounded: String = text.chars().take(MAX_CHARS).collect();
    bounded.push_str("…(truncated)");
    bounded
}
