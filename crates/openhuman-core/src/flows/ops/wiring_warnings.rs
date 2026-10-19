use super::*;

/// Ask TinyFlows for portable wiring diagnostics, with Composio contracts
/// supplied by the invocation-scoped OpenHuman callback.
pub(crate) async fn graph_wiring_warnings(
    ctx: &crate::core::runtime::DispatchCtx,
    config: &Config,
    graph: &WorkflowGraph,
) -> Result<Vec<String>, String> {
    let registration = crate::flows::authoring_host::register(
        std::sync::Arc::new(config.clone()),
        ctx.clone(),
    )
    .await?;
    let proxy = crate::modules::tinyflows::authoring_proxy(config).await?;
    proxy
        .call(
            tinyflows_bus::methods::WIRING_WARNINGS,
            &(tinyflows_bus::WiringWarningsRequest {
                graph: serde_json::to_value(graph).map_err(|error| error.to_string())?,
                host: registration.authoring.clone(),
            },),
        )
        .await
        .map_err(|error| error.wire_message())
}

/// Copilot-facing form of the same warning operation.
pub(crate) async fn wiring_warnings_json(
    ctx: &crate::core::runtime::DispatchCtx,
    config: &Config,
    graph: Value,
) -> Result<Vec<String>, String> {
    let graph: WorkflowGraph = serde_json::from_value(graph).map_err(|error| error.to_string())?;
    graph_wiring_warnings(ctx, config, &graph).await
}
