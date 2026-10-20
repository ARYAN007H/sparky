//! OpenHuman adapters for TinyAgents' thread-goal lifecycle.
//!
//! TinyAgents owns persistence, model-facing controls, prompt rendering, and
//! accounting. OpenHuman resolves a context-scoped KV store and publishes the
//! existing thread-goal domain events consumed by RPC and the web channel.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};

use tinyagents_harness_tools::goals::{GoalObserver, GoalStore, ThreadGoal};
use tinytools::Tool;

/// Serialize a [`ThreadGoal`] for the existing goal event and RPC payload.
#[must_use]
pub fn goal_to_value(goal: &ThreadGoal) -> serde_json::Value {
    serde_json::to_value(goal).unwrap_or(serde_json::Value::Null)
}

#[derive(Default)]
struct ScopedGoalStores(Mutex<HashMap<PathBuf, Weak<GoalStore>>>);

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
    if let Some(store) = stores.get(workspace_dir).and_then(Weak::upgrade) {
        return store;
    }

    let kv = crate::storage::sessions::stores_or_ephemeral(workspace_dir).kv;
    let store = Arc::new(GoalStore::with_observer(kv, Arc::new(OpenHumanGoalObserver)));
    stores.insert(workspace_dir.to_path_buf(), Arc::downgrade(&store));
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
    let context = crate::core::runtime::DispatchCtx::ambient();
    let store = goal_store(workspace_dir, &context);
    tinyagents_harness_tools::goals::GoalToolKind::MODEL_FACING
        .into_iter()
        .map(|kind| {
            Box::new(tinyagents_harness_tools::goals::GoalTool::new(
                kind,
                Arc::clone(&store),
            )) as Box<dyn Tool>
        })
        .collect()
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
