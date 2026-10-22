//! OpenHuman's context-bound source adapter for TinyJuice's shared REPL tools.

use crate::core::runtime::DispatchCtx;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tinyjuice_bus::repl::ReplLimits;
use tinyjuice_bus::wire::{QueryRequest, QueryResponse};
use tinytools::{Tool, ToolRunContext, ToolSpec};

/// Whether a name is one of TinyJuice's query tools.
pub fn is_repl_tool(name: &str) -> bool {
    tinyjuice_bus::tools::REPL_TOOLS.contains(&name)
}

/// The host-specific query seam. TinyJuice owns the tool implementation;
/// OpenHuman resolves the active turn's DispatchCtx and module configuration.
#[async_trait]
pub(crate) trait QuerySource: Send + Sync {
    async fn query(
        &self,
        ctx: &DispatchCtx,
        request: QueryRequest,
    ) -> Result<QueryResponse, String>;
}

struct ModuleSource {
    config: Option<Arc<crate::config::Config>>,
}

#[async_trait]
impl QuerySource for ModuleSource {
    async fn query(
        &self,
        ctx: &DispatchCtx,
        request: QueryRequest,
    ) -> Result<QueryResponse, String> {
        match &self.config {
            Some(config) => super::query_for_config(config, request).await,
            None => super::query(ctx, request).await,
        }
    }
}

struct QueryBridge(Arc<dyn QuerySource>);

#[async_trait]
impl tinyjuice_host::repl_tools::QuerySource for QueryBridge {
    async fn query(
        &self,
        context: Option<&dyn ToolRunContext>,
        request: QueryRequest,
    ) -> Result<QueryResponse, String> {
        let ctx = crate::agent::tinyagents::host::run_context::dispatch_ctx_for_tool(context);
        self.0.query(&ctx, request).await
    }
}

const MAX_ARTIFACT_BYTES: u64 = tinytools_std::filesystem::FileReadTool::MAX_FILE_SIZE_BYTES;

fn host_tools(
    source: Arc<dyn QuerySource>,
    limits: ReplLimits,
    artifacts_dir: Option<PathBuf>,
) -> Vec<Box<dyn Tool>> {
    tinyjuice_host::repl_tools::repl_tools(
        Arc::new(QueryBridge(source)),
        limits,
        artifacts_dir,
        MAX_ARTIFACT_BYTES,
        None,
    )
}

/// The current TinyJuice REPL declarations for ordinary turns.
pub fn repl_tools() -> Vec<Box<dyn Tool>> {
    host_tools(Arc::new(ModuleSource { config: None }), ReplLimits::default(), None)
}

/// Offer the REPL only while enabled handle storage can produce queryable handles.
pub fn repl_tools_for(config: &crate::config::Config) -> Vec<Box<dyn Tool>> {
    if !super::repl_handle_active(config) {
        return Vec::new();
    }
    host_tools(
        Arc::new(ModuleSource {
            config: Some(Arc::new(config.clone())),
        }),
        ReplLimits::default(),
        Some(crate::security::policy::tool_result_artifacts_dir(
            &config.workspace_dir,
        )),
    )
}

/// Rebuild the recorded query declarations exactly while keeping current host policy.
pub(crate) fn repl_tools_for_recorded(
    config: Option<&crate::config::Config>,
    workspace_dir: &Path,
    recorded: &[ToolSpec],
) -> Vec<Box<dyn Tool>> {
    let current = tinyjuice_bus::tools::repl_tool_declarations();
    let declarations = recorded
        .iter()
        .filter(|spec| is_repl_tool(&spec.name))
        .filter_map(|spec| {
            let declaration = current.iter().find(|item| item.name == spec.name)?;
            Some(tinyjuice_bus::tools::ReplToolDeclaration {
                name: spec.name.clone(),
                op: declaration.op.clone(),
                description: spec.description.clone(),
                parameters: spec.parameters.clone(),
            })
        })
        .collect();
    tinyjuice_host::repl_tools::repl_tools_with_declarations(
        declarations,
        Arc::new(QueryBridge(Arc::new(ModuleSource {
            config: config.cloned().map(Arc::new),
        }))),
        ReplLimits::default(),
        Some(crate::security::policy::tool_result_artifacts_dir(
            workspace_dir,
        )),
        MAX_ARTIFACT_BYTES,
        None,
    )
}

#[cfg(test)]
#[path = "repl_tools_tests.rs"]
mod tests;

