//! TinyBus `CapsHost` service for one request-scoped flow binding.

use std::sync::Arc;

use serde_json::Value;
use tinyflows_bus::runs;

use crate::security::credentials::HttpCredentialsStore;

#[path = "service_agent.rs"]
mod service_agent;
#[path = "service_helpers.rs"]
mod service_helpers;

use super::binding::CapsHostBinding;
use super::{OpenHumanCode, OpenHumanHttp, OpenHumanLlm, OpenHumanTools};
use service_helpers::{bus_error, memory, to_bus_agent_definition};

pub(crate) struct CapsHostService(Arc<CapsHostBinding>);

impl CapsHostService {
    pub(crate) fn new(binding: Arc<CapsHostBinding>) -> Self {
        Self(binding)
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

    async fn shell_run(
        &self,
        _request: runs::ShellRunRequest,
    ) -> tinybus::Result<runs::ShellRunOutcome> {
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

    async fn agent_run(
        &self,
        request: runs::AgentRunRequest,
    ) -> tinybus::Result<runs::AgentRunHandle> {
        let handle = uuid::Uuid::new_v4().simple().to_string();
        let state = Arc::new(tokio::sync::Mutex::new(
            super::agent::AgentRunState::default(),
        ));
        self.0.agent_runs.lock().await.insert(handle.clone(), state);
        let binding = Arc::clone(&self.0);
        tokio::spawn(async move {
            if let Err(error) = service_agent::run_agent(binding.clone(), request).await {
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
                let registry =
                    crate::agent::harness::host_definition::AgentDefinitionRegistry::current();
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
                let registry =
                    crate::agent::harness::host_definition::AgentDefinitionRegistry::current();
                registry
                    .map(|registry| {
                        registry
                            .list()
                            .into_iter()
                            .cloned()
                            .map(to_bus_agent_definition)
                            .collect()
                    })
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

    async fn approval_cancel(&self, _request: runs::ApprovalCancelRequest) -> tinybus::Result<()> {
        // Graph approval rows are module-owned; cancellation is represented
        // by the module's run lifecycle and does not cancel tool approvals.
        Ok(())
    }
}
