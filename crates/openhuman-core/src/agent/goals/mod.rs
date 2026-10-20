//! OpenHuman adapters for TinyAgents' thread-goal lifecycle.
//!
//! TinyAgents owns persistence, model-facing controls, prompt rendering, and
//! accounting. OpenHuman resolves a context-scoped KV store and publishes the
//! existing thread-goal domain events consumed by RPC and the web channel.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tinyagents_harness_tools::goals::{
    GoalObserver, GoalStore, GoalTool, GoalToolKind, ThreadGoal,
};
use tinytools::{PermissionLevel, Tool, ToolCallOptions, ToolPolicy, ToolResult, ToolRunContext};

/// Serialize a [`ThreadGoal`] for the existing goal event and RPC payload.
#[must_use]
pub fn goal_to_value(goal: &ThreadGoal) -> serde_json::Value {
    serde_json::to_value(goal).unwrap_or(serde_json::Value::Null)
}

#[derive(Default)]
struct ScopedGoalStores(Mutex<HashMap<PathBuf, Arc<GoalStore>>>);

/// Returns the goal store owned by this dispatch context and workspace.
///
/// The scoped slot keeps the owner's update throttle alive across turns while
/// ensuring separate tenants and embedded agents never share goal state.
pub fn goal_store(
    workspace_dir: &Path,
    context: &crate::core::runtime::DispatchCtx,
) -> Arc<GoalStore> {
    let stores = crate::core::runtime::slot_for::<ScopedGoalStores>(context);
    let mut stores = stores
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(store) = stores.get(workspace_dir) {
        return Arc::clone(store);
    }

    let owner_stores = match context.tenant() {
        Ok(tenant) => crate::storage::sessions::provider_for_workspace(workspace_dir)
            .for_agent(&crate::core::runtime::session_key(&tenant)),
        Err(no_tenant) => {
            log::warn!("[thread_goals] {no_tenant}; using throwaway in-memory store");
            tinyagents_session::port::InMemorySessionStores::new().for_agent("ephemeral")
        }
    };
    let kv = owner_stores.kv;
    let store = Arc::new(GoalStore::with_observer(
        kv,
        Arc::new(OpenHumanGoalObserver),
    ));
    stores.insert(workspace_dir.to_path_buf(), Arc::clone(&store));
    store
}

struct OpenHumanGoalObserver;

impl GoalObserver for OpenHumanGoalObserver {
    fn on_updated(&self, goal: &ThreadGoal) {
        crate::core::bus::BUS.publish(crate::core::events::DomainEvent::ThreadGoalUpdated {
            thread_id: goal.thread_id.clone(),
            goal_id: goal.goal_id.clone(),
            status: goal.status.as_str().to_string(),
            goal: Some(goal_to_value(goal)),
        });
    }

    fn on_cleared(&self, thread_id: &str) {
        crate::core::bus::BUS.publish(crate::core::events::DomainEvent::ThreadGoalCleared {
            thread_id: thread_id.to_string(),
        });
    }
}

/// The model-facing goal controls, backed by the context-scoped observer.
pub fn goal_tools(workspace_dir: &Path) -> Vec<Box<dyn Tool>> {
    GoalToolKind::MODEL_FACING
        .into_iter()
        .map(|kind| Box::new(OpenHumanGoalTool::new(kind, workspace_dir)) as Box<dyn Tool>)
        .collect()
}

struct OpenHumanGoalTool {
    kind: GoalToolKind,
    workspace_dir: PathBuf,
    metadata: GoalTool,
}

impl OpenHumanGoalTool {
    fn new(kind: GoalToolKind, workspace_dir: &Path) -> Self {
        let metadata_store = tinyagents_session::port::InMemorySessionStores::new().kv;
        Self {
            kind,
            workspace_dir: workspace_dir.to_path_buf(),
            metadata: GoalTool::new(kind, Arc::new(GoalStore::new(metadata_store))),
        }
    }

    fn for_context(&self, context: Option<&dyn ToolRunContext>) -> GoalTool {
        let dispatch = crate::agent::tinyagents::host::run_context::dispatch_ctx_for_tool(context);
        GoalTool::new(self.kind, goal_store(&self.workspace_dir, &dispatch))
    }
}

#[async_trait::async_trait]
impl Tool for OpenHumanGoalTool {
    fn name(&self) -> &str {
        self.metadata.name()
    }

    fn description(&self) -> &str {
        self.metadata.description()
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.metadata.parameters_schema()
    }

    fn supports_markdown(&self) -> bool {
        self.metadata.supports_markdown()
    }

    fn permission_level(&self) -> PermissionLevel {
        self.metadata.permission_level()
    }

    fn policy(&self) -> ToolPolicy {
        self.metadata.policy()
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        self.for_context(None).execute(args).await
    }

    async fn execute_with_context(
        &self,
        args: serde_json::Value,
        options: ToolCallOptions,
        context: Option<&dyn ToolRunContext>,
    ) -> anyhow::Result<ToolResult> {
        self.for_context(context)
            .execute_with_context(args, options, context)
            .await
    }
}

/// Best-effort account of receipt-backed usage against a thread's goal.
pub async fn account_turn(
    workspace_dir: &Path,
    context: &crate::core::runtime::DispatchCtx,
    thread_id: Option<&str>,
    input_tokens: u64,
    output_tokens: u64,
    elapsed: std::time::Duration,
) {
    let Some(thread_id) = thread_id.map(str::trim).filter(|id| !id.is_empty()) else {
        return;
    };
    if let Err(error) = goal_store(workspace_dir, context)
        .account_turn(thread_id, input_tokens, output_tokens, elapsed)
        .await
    {
        tracing::debug!(thread_id, %error, "[thread_goals] account_turn failed");
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
