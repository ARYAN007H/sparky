//! Host adapters over the runtime's generic tool registration
//! ([`tinyagents_runtime::turn_driver::ToolSetBuilder`]): which typed dispatch a
//! tool needs, the ambient turn-origin scope every tool execution runs in, and
//! the capability-registry projection behind the fail-closed diagnostics gate.

use std::collections::HashSet;
use std::sync::Arc;

use tinyagents_graph::goals::GoalToolKind;
use tinyagents_harness::context::RunContext;
use tinyagents_harness::tool::packs::USE_SKILL;
use tinyagents_harness::tool::{
    CanonicalSharedToolAdapter, ToolDispatch, ToolExecScope, ToolExecution, ToolExecutionContext,
};
use tinyagents_registry::{CapabilityRegistry, ComponentKind, RegistryDiagnostic};
use tinyagents_runtime::turn_driver::{SharedToolSet, ToolSetBuilder};
use tinyagents_runtime::{project_descriptors, ToolAdmission};
use tinyinference_llm::model::ChatModel;
use tinytools::ToolRunContext;

use crate::agent::orchestration::tools::{
    CloseSubagentDispatch, ContinueSubagentDispatch, DelegateGraphDispatch, DelegationDispatch,
    ListSubagentsDispatch, SpawnAsyncSubagentDispatch, SpawnParallelAgentsDispatch,
    SpawnSubagentDispatch, SteerSubagentDispatch, WaitSubagentDispatch,
};
use crate::agent::tinyagents::host::run_context::HostOperationContext;
use crate::agent::tinyagents::host::OpenHumanRunContext;
use crate::agent::tinyagents::turn_policy::is_subagent_spawn_or_delegate_tool;
use crate::agent::tinyagents::use_skill_dispatch::UseSkillDispatch;
use crate::agent::tools::TodoToolDispatch;
use crate::core::runtime::CoreContext;
use crate::tools::delegate::DelegateToolDispatch;
use crate::tools::implementations::threads::SpawnWorkerThreadDispatch;

type RegisteredToolDispatch = Arc<dyn ToolDispatch<(), OpenHumanRunContext>>;
type ToolDispatchSelectorFn =
    dyn Fn(&str, Arc<dyn tinytools::Tool>) -> Option<RegisteredToolDispatch> + Send + Sync;
type ToolDispatchSelector = Arc<ToolDispatchSelectorFn>;

#[derive(Clone, Copy)]
enum SubagentResultProjection {
    List,
    Wait,
}

struct ProjectedSubagentDispatch {
    inner: RegisteredToolDispatch,
    tool: Arc<dyn tinytools::Tool>,
    projection: SubagentResultProjection,
}

#[async_trait::async_trait]
impl ToolDispatch<(), OpenHumanRunContext> for ProjectedSubagentDispatch {
    fn tool(&self) -> Arc<dyn tinytools::Tool> {
        self.tool.clone()
    }

    fn output_origin(&self) -> tinyagents_harness::host::ContentOrigin {
        self.inner.output_origin()
    }

    async fn execute(
        &self,
        state: &(),
        call_id: tinyagents_harness::CallId,
        arguments: serde_json::Value,
        options: tinytools::ToolCallOptions,
        parent: &RunContext<OpenHumanRunContext>,
    ) -> anyhow::Result<tinytools::ToolResult> {
        let result = self
            .inner
            .execute(state, call_id, arguments.clone(), options, parent)
            .await?;
        Ok(project_subagent_result(
            self.projection,
            result,
            &arguments,
            parent,
        ))
    }
}

fn project_subagent_result(
    projection: SubagentResultProjection,
    result: tinytools::ToolResult,
    arguments: &serde_json::Value,
    parent: &RunContext<OpenHumanRunContext>,
) -> tinytools::ToolResult {
    match projection {
        SubagentResultProjection::List => project_list_subagents(result, parent),
        SubagentResultProjection::Wait => project_wait_subagent(result, arguments, parent),
    }
}

fn project_list_subagents(
    result: tinytools::ToolResult,
    parent: &RunContext<OpenHumanRunContext>,
) -> tinytools::ToolResult {
    use tinytools::{ToolContent, ToolResult};

    if result.is_error {
        return result;
    }
    let Some(ToolContent::Json { data }) = result.content.first() else {
        return result;
    };
    let Some(rows) = data.as_array() else {
        return result;
    };
    let Some(parent_ctx) = parent.data.parent.as_ref() else {
        return result;
    };
    let mut summaries = Vec::new();
    for row in rows
        .iter()
        .filter(|row| row.get("kind").and_then(serde_json::Value::as_str) == Some("session"))
    {
        let mut row = row.clone();
        if let Some(object) = row.as_object_mut() {
            object.remove("kind");
        }
        let Ok(mut summary) = serde_json::from_value::<
            crate::agent::orchestration::subagent_sessions::DurableSubagentSessionSummary,
        >(row) else {
            continue;
        };
        crate::agent::orchestration::tools::overlay_task_store_status(
            &mut summary,
            &parent_ctx.workspace_dir,
            &parent_ctx.session_id,
        );
        summaries.push(summary);
    }
    ToolResult::success(format!(
        "[subagent_sessions]\n{}\n[/subagent_sessions]",
        serde_json::to_string_pretty(&summaries).unwrap_or_else(|_| "[]".into())
    ))
}

fn project_wait_subagent(
    result: tinytools::ToolResult,
    arguments: &serde_json::Value,
    parent: &RunContext<OpenHumanRunContext>,
) -> tinytools::ToolResult {
    use tinyagents_orchestration::subagent::SubagentResumeRef;
    use tinytools::{ToolContent, ToolResult};

    if result.is_error {
        return result;
    }
    let Some(ToolContent::Json { data }) = result.content.first() else {
        return result;
    };
    let Some(object) = data.as_object() else {
        return result;
    };
    let Some(parent_ctx) = parent.data.parent.as_ref() else {
        return result;
    };
    let extensions = object
        .get("extensions")
        .and_then(serde_json::Value::as_object);
    let task_id = extensions
        .and_then(|values| values.get("task_id"))
        .and_then(serde_json::Value::as_str)
        .or_else(|| object.get("task_id").and_then(serde_json::Value::as_str))
        .or_else(|| arguments.get("task_id").and_then(serde_json::Value::as_str))
        .unwrap_or("")
        .to_owned();
    let agent_id = extensions
        .and_then(|values| values.get("agent_id"))
        .and_then(serde_json::Value::as_str)
        .or_else(|| object.get("agent").and_then(serde_json::Value::as_str))
        .unwrap_or("unknown")
        .to_owned();
    if task_id.is_empty() {
        return result;
    }
    let parent_thread = parent.data.thread_id.as_deref();
    let store = crate::agent::orchestration::subagent_sessions::SubagentSessionStore::new(
        parent_ctx.workspace_dir.clone(),
    );
    let session_id = crate::agent::orchestration::subagent_sessions::list_for_parent(
        &store,
        &parent_ctx.session_id,
        parent_thread,
    )
    .ok()
    .and_then(|sessions| {
        sessions
            .into_iter()
            .find(|session| session.current_task_id.as_deref() == Some(&task_id))
            .map(|session| session.subagent_session_id)
    });
    let resume = SubagentResumeRef {
        task_id: task_id.clone(),
        agent_id: agent_id.clone(),
        subagent_session_id: session_id,
    };
    let status = object
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("running");
    let iterations = extensions
        .and_then(|values| values.get("iterations"))
        .and_then(serde_json::Value::as_u64)
        .map(|value| value as usize);
    let output = object
        .get("output")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let error = object
        .get("error")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("subagent execution failed");

    match status {
        "completed" => {
            crate::agent::orchestration::background_completions::mark_collected(
                &parent_ctx.workspace_dir,
                &task_id,
            );
            let payload = crate::agent::orchestration::tools::wait_status_payload(
                Some(&resume),
                &task_id,
                "completed",
                iterations,
                None,
                "synthesize the sub-agent output into the parent response",
            );
            ToolResult::success(format!(
                "Sub-agent `{agent_id}` completed in {} iteration(s).\n\n[subagent_wait_result]\n{}\n[/subagent_wait_result]\n\n{output}",
                iterations.unwrap_or_default(),
                serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into())
            ))
        }
        "awaiting_input" | "awaiting_user" => {
            let question = output;
            let payload = crate::agent::orchestration::tools::wait_status_payload(
                Some(&resume),
                &task_id,
                "awaiting_user",
                None,
                Some(question),
                "ask the user for the missing information, then call continue_subagent",
            );
            let mut message = format!(
                "Sub-agent `{agent_id}` paused for clarification and did not finish: {question}\n\nIt cannot proceed unattended. Resume it with continue_subagent once you have an answer.\n\n[subagent_wait_result]\n{}\n[/subagent_wait_result]",
                serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into())
            );
            if let Some(session_id) = resume.subagent_session_id {
                message.push_str("\n\n[subagent_resume_ref]\n");
                message.push_str(
                    &serde_json::to_string(&serde_json::json!({
                        "task_id": task_id,
                        "agent_id": agent_id,
                        "subagent_session_id": session_id,
                        "tool": "continue_subagent"
                    }))
                    .unwrap_or_else(|_| "{}".into()),
                );
                message.push_str("\n[/subagent_resume_ref]");
            }
            ToolResult::success(message)
        }
        "failed" | "incomplete" | "cancelled" => {
            let payload = crate::agent::orchestration::tools::wait_status_payload(
                Some(&resume),
                &task_id,
                "failed",
                None,
                Some(error),
                "report the failure or retry with a corrected instruction",
            );
            ToolResult::error(format!(
                "Sub-agent `{agent_id}` failed: {error}\n\n[subagent_wait_result]\n{}\n[/subagent_wait_result]",
                serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into())
            ))
        }
        _ => ToolResult::success(
            crate::agent::orchestration::tools::format_running_wait_message(
                Some(&resume),
                &task_id,
                crate::agent::orchestration::tools::requested_timeout_secs(arguments),
            ),
        ),
    }
}

/// Typed-dispatch selection shared by the direct per-turn registration below
/// and by [`UseSkillDispatch`], which must resolve the SAME live-parent
/// dispatch for a packed archetype delegation (`create_image`, `do_crypto`,
/// `make_presentation`, …) reached through `use_skill` instead of natively
/// advertised (regression R3: `use_skill` used to hand every packed tool to
/// plain `Tool::execute_with_context`, which has no live parent, so a packed
/// delegation always failed with "delegation requires a live harness run
/// context.").
///
/// `adapter` is expected to be the same `CanonicalSharedToolAdapter` seam
/// used at registration: dispatch selection keys off `name` and the tool's
/// own schema (via [`DelegationDispatch::for_tool`]'s fallback), not object
/// identity, so a freshly built adapter over the resolved tool's registry
/// slot is equivalent to the one the harness itself would have registered.
pub(crate) fn typed_dispatch_for(
    name: &str,
    adapter: Arc<dyn tinytools::Tool>,
) -> Option<Arc<dyn ToolDispatch<(), OpenHumanRunContext>>> {
    let dispatch: Arc<dyn ToolDispatch<(), OpenHumanRunContext>> = match name {
        "spawn_parallel_agents" => Arc::new(SpawnParallelAgentsDispatch::new(adapter)),
        "spawn_async_subagent" => Arc::new(SpawnAsyncSubagentDispatch::new(adapter)),
        "spawn_worker_thread" => Arc::new(SpawnWorkerThreadDispatch::new(adapter)),
        "spawn_subagent" => Arc::new(SpawnSubagentDispatch::new(adapter)),
        "continue_subagent" => Arc::new(ContinueSubagentDispatch::new(adapter)),
        "wait_subagent" => Arc::new(WaitSubagentDispatch::new(adapter)),
        "steer_subagent" => Arc::new(SteerSubagentDispatch::new(adapter)),
        "close_subagent" => Arc::new(CloseSubagentDispatch::new(adapter)),
        "list_subagents" => Arc::new(ListSubagentsDispatch::new(adapter)),
        "delegate_graph" => Arc::new(DelegateGraphDispatch::new(adapter)),
        "delegate" | "memory" => Arc::new(DelegateToolDispatch::new(adapter)),
        "todo" => Arc::new(TodoToolDispatch::new(adapter)),
        _ => {
            return DelegationDispatch::for_tool(adapter).map(|dispatch| {
                Arc::new(dispatch) as Arc<dyn ToolDispatch<(), OpenHumanRunContext>>
            });
        }
    };
    Some(dispatch)
}

/// Names among the candidate tools that spawn or delegate to a sub-agent.
fn candidate_spawn_tools(tool_sets: &[SharedToolSet]) -> HashSet<String> {
    tool_sets
        .iter()
        .flat_map(|set| set.iter())
        .map(|tool| tool.name().to_string())
        .filter(|name| is_subagent_spawn_or_delegate_tool(name))
        .collect()
}

/// Whether `name` is one of the model-facing per-thread goal tools
/// (`goal_get` / `goal_set` / `goal_complete`), named by their owner.
pub(crate) fn is_thread_goal_tool(name: &str) -> bool {
    GoalToolKind::MODEL_FACING
        .iter()
        .any(|kind| kind.name() == name)
}

/// Runs every tool execution inside the turn's [`AgentTurnOrigin`] scope.
///
/// The origin rides the run's typed host operation context; the process-wide
/// `CoreContext` is re-scoped with it so a tool that asks "who is acting"
/// answers for this turn and not for whichever turn last set a global.
///
/// [`AgentTurnOrigin`]: crate::security::turn_origin::AgentTurnOrigin
pub(crate) struct TurnOriginScope;

impl ToolExecScope for TurnOriginScope {
    fn scope<'a>(
        &'a self,
        context: Option<&'a dyn ToolRunContext>,
        execution: ToolExecution<'a>,
    ) -> ToolExecution<'a> {
        let origin = context
            .and_then(ToolRunContext::host_extension)
            .and_then(|extension| extension.downcast_ref::<ToolExecutionContext>())
            .and_then(|context| context.state::<HostOperationContext>())
            .and_then(|context| context.origin.clone());
        match (
            origin,
            super::host::run_context::dispatch_ctx_for_tool(context).into_core(),
        ) {
            (Some(origin), Some(core_context)) => Box::pin(CoreContext::scope_with_turn_origin(
                core_context,
                Some(origin),
                execution,
            )),
            _ => execution,
        }
    }
}

/// The adapter for tool `name` over `sets`, scoped to the turn origin.
///
/// This is the same adapter the registration builds, so a dispatch that
/// resolves a tool later (`use_skill`) executes it exactly as a natively
/// registered one.
pub(crate) fn shared_tool_adapter(
    sets: Vec<SharedToolSet>,
    name: &str,
) -> Option<CanonicalSharedToolAdapter> {
    Some(
        CanonicalSharedToolAdapter::for_name(sets, name)?
            .with_exec_scope(Arc::new(TurnOriginScope)),
    )
}

type DispatchSelector = Arc<
    dyn Fn(&str, Arc<dyn tinytools::Tool>) -> Option<Arc<dyn ToolDispatch<(), OpenHumanRunContext>>>
        + Send
        + Sync,
>;

/// Chooses the typed dispatch for a registered tool: `use_skill` gets its own
/// (it is the proxy every packed archetype delegation is reached through and
/// must resolve the SAME live parent a natively advertised delegate tool does,
/// regression R3), everything else goes through [`typed_dispatch_for`].
fn dispatch_selector(tool_sets: Vec<SharedToolSet>) -> DispatchSelector {
    Arc::new(move |name, adapter| {
        if name != USE_SKILL {
            return typed_dispatch_for(name, adapter);
        }
        // The pack-registry handle comes off the raw registered tool, not the
        // adapter.
        let handle = tool_sets
            .iter()
            .flat_map(|set| set.iter())
            .find(|tool| tool.name() == name)
            .and_then(|tool| crate::tools::host_extensions::pack_registry_handle(tool.as_ref()))
            .cloned()?;
        Some(Arc::new(UseSkillDispatch::new(adapter, handle)))
    })
}

fn subagent_control_dispatches(
    tool_sets: &[SharedToolSet],
    run_context: &OpenHumanRunContext,
) -> std::collections::HashMap<String, RegisteredToolDispatch> {
    use tinyagents_orchestration::subagent::{
        SubAgentJobAction, SubAgentJobRegistry, SubAgentJobsTool, SubAgentMessageTool,
    };

    let Some(parent) = run_context.parent.as_ref() else {
        return std::collections::HashMap::new();
    };
    let registry =
        crate::core::runtime::slot_for::<SubAgentJobRegistry>(&run_context.dispatch_ctx());
    let jobs = (*registry).clone();
    let store: Arc<dyn tinyagents_orchestration::subagent::SubagentSessionStore> = Arc::new(
        crate::agent::orchestration::subagent_sessions::OpenHumanTinySubagentSessionStore::new(
            parent.workspace_dir.clone(),
            parent.session_id.clone(),
            run_context.thread_id.clone(),
        ),
    );
    let owner = format!(
        "{}:{}",
        parent.session_id,
        run_context.thread_id.as_deref().unwrap_or("no-thread")
    );
    let parent_session = parent.session_id.clone();
    let owner_scope: Arc<dyn Fn(u64) -> String + Send + Sync> = Arc::new(move |_| owner.clone());

    let find_tool = |name: &str| -> Option<Arc<dyn tinytools::Tool>> {
        shared_tool_adapter(tool_sets.to_vec(), name)
            .map(|adapter| Arc::new(adapter) as Arc<dyn tinytools::Tool>)
    };
    let make_jobs_alias =
        |name: &str, action: SubAgentJobAction| -> Option<RegisteredToolDispatch> {
            let adapter = find_tool(name)?;
            let parent_session = parent_session.clone();
            let dispatch = SubAgentJobsTool::new(jobs.clone())
                .with_tool_name(name)
                .with_description(adapter.description())
                .with_parameters(adapter.parameters_schema())
                .with_fixed_action(action)
                .with_metadata(
                    adapter.permission_level(),
                    adapter.scope(),
                    adapter.category(),
                    tinytools::ToolTimeout::Inherit,
                )
                .with_owner_scope(owner_scope.clone())
                .with_sessions(store.clone(), move |_| parent_session.clone());
            Some(Arc::new(dispatch))
        };
    let mut dispatches = std::collections::HashMap::new();
    if let Some(dispatch) = make_jobs_alias("list_subagents", SubAgentJobAction::Query) {
        dispatches.insert(
            "list_subagents".into(),
            Arc::new(ProjectedSubagentDispatch {
                tool: dispatch.tool(),
                inner: dispatch,
                projection: SubagentResultProjection::List,
            }) as RegisteredToolDispatch,
        );
    }
    if let Some(adapter) = find_tool("wait_subagent") {
        let parent_session = parent_session.clone();
        let wait_dispatch: RegisteredToolDispatch = Arc::new(
            SubAgentJobsTool::new(jobs.clone())
                .with_tool_name("wait_subagent")
                .with_description(adapter.description())
                .with_parameters(adapter.parameters_schema())
                .with_fixed_action(SubAgentJobAction::Await)
                .with_identifier_fields("task_id", "subagent_session_id")
                .with_await_timeout_field("timeout_secs", 120, 3600)
                .with_required_identifier()
                .with_metadata(
                    adapter.permission_level(),
                    adapter.scope(),
                    adapter.category(),
                    tinytools::ToolTimeout::Inherit,
                )
                .with_owner_scope(owner_scope.clone())
                .with_sessions(store.clone(), move |_| parent_session.clone()),
        );
        dispatches.insert(
            "wait_subagent".into(),
            Arc::new(ProjectedSubagentDispatch {
                tool: wait_dispatch.tool(),
                inner: wait_dispatch,
                projection: SubagentResultProjection::Wait,
            }),
        );
    }
    if let Some(adapter) = find_tool("close_subagent") {
        let parent_session = parent_session.clone();
        dispatches.insert(
            "close_subagent".into(),
            Arc::new(
                SubAgentJobsTool::new(jobs.clone())
                    .with_tool_name("close_subagent")
                    .with_description(adapter.description())
                    .with_parameters(adapter.parameters_schema())
                    .with_fixed_action(SubAgentJobAction::Close)
                    .with_identifier_field("subagent_session_id")
                    .with_required_identifier()
                    .with_metadata(
                        adapter.permission_level(),
                        adapter.scope(),
                        adapter.category(),
                        adapter.timeout_policy(&serde_json::Value::Null),
                    )
                    .with_owner_scope(owner_scope.clone())
                    .with_sessions(store.clone(), move |_| parent_session.clone()),
            ),
        );
    }
    if let Some(adapter) = find_tool("steer_subagent") {
        let parent_session = parent_session.clone();
        dispatches.insert(
            "steer_subagent".into(),
            Arc::new(
                SubAgentMessageTool::new(jobs)
                    .with_tool_name("steer_subagent")
                    .with_description(adapter.description())
                    .with_parameters(adapter.parameters_schema())
                    .with_argument_fields(
                        "task_id",
                        "subagent_session_id",
                        "message",
                        "__request_id",
                    )
                    .with_lane_field("mode")
                    .with_metadata(
                        adapter.permission_level(),
                        adapter.scope(),
                        adapter.category(),
                        adapter.timeout_policy(&serde_json::Value::Null),
                    )
                    .with_owner_scope(owner_scope)
                    .with_sessions(store, move |_| parent_session.clone()),
            ),
        );
    }
    dispatches
}

fn turn_dispatch_selector(
    tool_sets: Vec<SharedToolSet>,
    run_context: &OpenHumanRunContext,
) -> DispatchSelector {
    let aliases = subagent_control_dispatches(&tool_sets, run_context);
    let fallback = dispatch_selector(tool_sets);
    Arc::new(move |name, adapter| {
        aliases
            .get(name)
            .cloned()
            .or_else(|| fallback(name, adapter))
    })
}

/// The admission rules for one turn.
///
/// Allowlist semantics are **fail-closed** (issue #4452): `None` admits every
/// visible tool; `Some(set)` admits *exactly* the named tools, so `Some(empty)`
/// denies all. That keeps a deliberately tool-less sub-agent from silently
/// inheriting the parent's full tool surface. `has_thread == false` drops the
/// per-thread goal tools, which cannot run without a chat thread (#6956).
pub(crate) fn turn_admission(
    tool_sets: &[SharedToolSet],
    allowed: &Option<HashSet<String>>,
    has_thread: bool,
) -> ToolAdmission {
    ToolAdmission {
        allowed: allowed.clone(),
        // A sub-agent scope on the turn forces this on in the driver.
        is_subagent_run: false,
        has_thread,
        thread_scoped: GoalToolKind::MODEL_FACING
            .iter()
            .map(|kind| kind.name().to_string())
            .collect(),
        spawn_tools: candidate_spawn_tools(tool_sets),
    }
}

/// The host's tool registration: shared sets, fail-closed admission, session
/// deferral, typed dispatches and the turn-origin execution scope.
pub(crate) fn host_tool_set_builder(
    tool_sets: &[SharedToolSet],
    allowed: &Option<HashSet<String>>,
    has_thread: bool,
    session_deferred: &HashSet<String>,
) -> ToolSetBuilder<(), OpenHumanRunContext> {
    ToolSetBuilder::new()
        .sets(tool_sets.iter().cloned())
        .admission(turn_admission(tool_sets, allowed, has_thread))
        .deferred(session_deferred.iter().cloned())
        .dispatch(dispatch_selector(tool_sets.to_vec()))
        .exec_scope(Arc::new(TurnOriginScope))
}

pub(crate) fn host_tool_set_builder_for_turn(
    tool_sets: &[SharedToolSet],
    allowed: &Option<HashSet<String>>,
    has_thread: bool,
    session_deferred: &HashSet<String>,
    run_context: &OpenHumanRunContext,
) -> ToolSetBuilder<(), OpenHumanRunContext> {
    ToolSetBuilder::new()
        .sets(tool_sets.iter().cloned())
        .admission(turn_admission(tool_sets, allowed, has_thread))
        .deferred(session_deferred.iter().cloned())
        .dispatch(turn_dispatch_selector(tool_sets.to_vec(), run_context))
        .exec_scope(Arc::new(TurnOriginScope))
}

/// Projects the turn's models, the `admitted` tools, graph topologies and visible
/// agents into a [`CapabilityRegistry`] and returns its health diagnostics.
///
/// The turn path enforces them fail-closed before the first model dispatch, so
/// an ambiguous tool surface (a name shared across native/MCP/Composio/generated
/// tools, a dangling alias) aborts instead of silently resolving to an
/// unintended component mid-call. The live run keeps using the harness's own
/// registries; this projection is metadata only.
pub(crate) fn capability_diagnostics(
    models: &[(String, Arc<dyn ChatModel<()>>)],
    tool_sets: &[SharedToolSet],
    admitted: &[String],
) -> Vec<RegistryDiagnostic> {
    let mut registry: CapabilityRegistry<()> = CapabilityRegistry::new();
    for (name, model) in models {
        registry.replace_model(name.as_str(), model.clone());
    }
    for name in admitted {
        if let Some(adapter) = shared_tool_adapter(tool_sets.to_vec(), name) {
            registry.replace_tool(Arc::new(adapter));
        }
    }
    project_descriptors(
        &mut registry,
        ComponentKind::Graph,
        tinyagents_orchestration::topology::all_graph_topologies()
            .into_iter()
            .map(|report| report.name),
    );

    // The agents visible to this turn, as name-only `ComponentKind::Agent`
    // descriptors (issue #4249, Workstream 10.1): metadata only, no executable
    // agent is attached (sub-agent dispatch flows through the OpenHuman
    // sub-agent runner). Sourced from the runtime `AgentDefinitionRegistry`
    // (built-ins plus workspace/config overrides, already merged by id) first,
    // then the `agent_registry` built-in loader, deduped by id.
    let runtime_ids: Vec<String> =
        crate::agent_registry::host_definition::AgentDefinitionRegistry::current()
            .map(|runtime| runtime.list().iter().map(|def| def.id.clone()).collect())
            .unwrap_or_default();
    let runtime_agent_count = project_descriptors(&mut registry, ComponentKind::Agent, runtime_ids);
    let builtin_supplement_count = match crate::agent_registry::agents::load_builtins() {
        Ok(builtins) => project_descriptors(
            &mut registry,
            ComponentKind::Agent,
            builtins.into_iter().map(|def| def.id),
        ),
        Err(err) => {
            tracing::debug!(
                %err,
                "[registry] agent_registry builtin load failed; \
                 registered runtime-registry agents only"
            );
            0
        }
    };

    let diagnostics = registry.diagnostics();
    tracing::debug!(
        models = registry.to_model_registry().names().len(),
        tools = registry.names(ComponentKind::Tool).len(),
        graphs = registry.names(ComponentKind::Graph).len(),
        agents = runtime_agent_count + builtin_supplement_count,
        diagnostics = diagnostics.len(),
        "[registry] per-turn capability projection summary"
    );
    diagnostics
}

#[cfg(test)]
#[path = "tool_registration_tests.rs"]
mod tests;
