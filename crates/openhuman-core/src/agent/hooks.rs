//! Process-wide hooks supplied by an embedding host.
//!
//! Hook contracts and their per-agent scope are owned by `tinyagents-hooks`;
//! this module only keeps the embedder registries and the host identity bridge.

use std::sync::Arc;

pub use tinyagents_harness::host::{LearningSink as PostTurnHook, TurnSummary as TurnContext};
pub use tinyagents_hooks::tool::{
    HookIdentity, ToolHook, ToolHookContext, ToolHookDecision, ToolHookEvent,
};

use crate::agent::tinyagents::host::OpenHumanRunContext;
pub type HookScope = tinyagents_hooks::scope::HookScope<
    OpenHumanRunContext,
    tinyagents_harness::cost::TurnCost,
>;

static EMBEDDER_POST_TURN_HOOKS:
    std::sync::LazyLock<std::sync::Mutex<Vec<(Option<String>, Arc<dyn PostTurnHook>)>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));
static EMBEDDER_TOOL_HOOKS:
    std::sync::LazyLock<
        std::sync::Mutex<Vec<(Option<String>, Arc<dyn ToolHook<OpenHumanRunContext>>) >>,
    > =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

impl HookIdentity for OpenHumanRunContext {
    fn session_id(&self) -> Option<String> {
        self.thread_id.clone()
    }

    fn agent_id(&self) -> Option<String> {
        self.dispatch_ctx()
            .core()
            .and_then(|context| context.session_agent().map(str::to_owned))
    }

    fn cwd(&self) -> Option<std::path::PathBuf> {
        self.workspace
            .as_ref()
            .map(|workspace| workspace.root.clone())
            .or_else(|| {
                self.dispatch_ctx()
                    .core()
                    .and_then(|context| context.workspace_dir().ok())
            })
    }
}

pub fn register_embedder_post_turn_hook(hook: Arc<dyn PostTurnHook>) {
    EMBEDDER_POST_TURN_HOOKS
        .lock()
        .expect("embedder learning sinks poisoned")
        .push((None, hook));
}

pub fn replace_embedder_post_turn_hook(name: &str, hook: Option<Arc<dyn PostTurnHook>>) {
    let mut hooks = EMBEDDER_POST_TURN_HOOKS
        .lock()
        .expect("embedder learning sinks poisoned");
    hooks.retain(|(registered, _)| registered.as_deref() != Some(name));
    if let Some(hook) = hook {
        hooks.push((Some(name.to_owned()), hook));
    }
}

pub fn embedder_post_turn_hooks() -> Vec<Arc<dyn PostTurnHook>> {
    EMBEDDER_POST_TURN_HOOKS
        .lock()
        .expect("embedder learning sinks poisoned")
        .iter()
        .map(|(_, hook)| Arc::clone(hook))
        .collect()
}

pub fn embedder_post_turn_hooks_in(
    ctx: &crate::core::runtime::DispatchCtx,
) -> Vec<Arc<dyn PostTurnHook>> {
    let mut hooks = embedder_post_turn_hooks();
    if let Some(local) = ctx.host_overrides() {
        hooks.extend(local.post_turn_hooks());
    }
    hooks
}

pub fn register_embedder_tool_hook(hook: Arc<dyn ToolHook<OpenHumanRunContext>>) {
    EMBEDDER_TOOL_HOOKS
        .lock()
        .expect("embedder tool hooks poisoned")
        .push((None, hook));
}

pub fn replace_embedder_tool_hook(
    name: &str,
    hook: Option<Arc<dyn ToolHook<OpenHumanRunContext>>>,
) {
    let mut hooks = EMBEDDER_TOOL_HOOKS
        .lock()
        .expect("embedder tool hooks poisoned");
    hooks.retain(|(registered, _)| registered.as_deref() != Some(name));
    if let Some(hook) = hook {
        hooks.push((Some(name.to_owned()), hook));
    }
}

pub fn embedder_tool_hooks() -> Vec<Arc<dyn ToolHook<OpenHumanRunContext>>> {
    EMBEDDER_TOOL_HOOKS
        .lock()
        .expect("embedder tool hooks poisoned")
        .iter()
        .map(|(_, hook)| Arc::clone(hook))
        .collect()
}

pub fn embedder_tool_hooks_in(
    ctx: &crate::core::runtime::DispatchCtx,
) -> Vec<Arc<dyn ToolHook<OpenHumanRunContext>>> {
    let mut hooks = embedder_tool_hooks();
    if let Some(local) = ctx.host_overrides() {
        hooks.extend(local.tool_hooks());
    }
    hooks
}
