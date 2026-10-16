//! Tool sources: id-keyed factories an outside crate registers so its tools
//! join the agent catalogue.
//!
//! [`all_tools_with_runtime`](super::ops::all_tools_with_runtime) builds the
//! built-in catalogue, then appends every registered source in registration
//! order, then applies the capability filters (`DomainSet`, memory capability,
//! `ToolGroups`). A registered tool is therefore filtered exactly like a
//! built-in one. With nothing registered the catalogue is unchanged.
//!
//! The registry is generic: it knows nothing about what a source builds.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use tinytools::Tool;

use crate::agent::host_runtime::RuntimeAdapter;
use crate::config::{BrowserConfig, Config, DelegateAgentConfig, HttpRequestConfig};
use crate::core::slots::HookSlot;
use crate::security::{AuditLogger, SecurityPolicy};

/// What a tool source can see while the catalogue is assembled: the same
/// handles `all_tools_with_runtime` was given.
pub struct ToolSourceCtx<'a> {
    /// The session's configuration snapshot.
    pub config: &'a Arc<Config>,
    /// The root configuration the catalogue is assembled under.
    pub root_config: &'a Config,
    pub security: &'a Arc<SecurityPolicy>,
    pub runtime: &'a Arc<dyn RuntimeAdapter>,
    pub audit: &'a Arc<AuditLogger>,
    pub browser_config: &'a BrowserConfig,
    pub http_config: &'a HttpRequestConfig,
    /// The agent's permitted read/write root.
    pub action_dir: &'a Path,
    pub agents: &'a HashMap<String, DelegateAgentConfig>,
    pub approval_workspace_root: Option<&'a Path>,
}

/// A registered tool source.
pub type ToolSourceFactory =
    Arc<dyn for<'a> Fn(&ToolSourceCtx<'a>) -> Vec<Box<dyn Tool>> + Send + Sync>;

static TOOL_SOURCES: HookSlot<ToolSourceFactory> = HookSlot::new("tools::sources");

/// Registers `factory` as the tool source `id` for the whole process.
/// Re-registering an id replaces its factory and keeps its position.
pub fn register_tool_source<F>(id: impl Into<String>, factory: F)
where
    F: for<'a> Fn(&ToolSourceCtx<'a>) -> Vec<Box<dyn Tool>> + Send + Sync + 'static,
{
    TOOL_SOURCES.register(id, Arc::new(factory));
}

/// Registers a tool source for the context built from `extensions`.
pub fn register_tool_source_in<F>(
    extensions: &crate::core::slots::Extensions,
    id: impl Into<String>,
    factory: F,
) where
    F: for<'a> Fn(&ToolSourceCtx<'a>) -> Vec<Box<dyn Tool>> + Send + Sync + 'static,
{
    TOOL_SOURCES.register_in(extensions, id, Arc::new(factory));
}

/// Removes the process-wide tool source `id`.
pub fn unregister_tool_source(id: &str) -> bool {
    TOOL_SOURCES.unregister(id)
}

/// Builds every registered source's tools, in registration order.
pub(crate) fn build_registered_tool_sources(ctx: &ToolSourceCtx<'_>) -> Vec<Box<dyn Tool>> {
    let mut tools = Vec::new();
    for (id, factory) in TOOL_SOURCES.entries() {
        let built = factory(ctx);
        tracing::debug!(
            source = %id,
            count = built.len(),
            "[tools::sources] registered tool source built"
        );
        tools.extend(built);
    }
    tools
}

#[cfg(test)]
#[path = "sources_tests.rs"]
mod tests;
