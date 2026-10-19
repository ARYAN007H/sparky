use super::*;
use crate::core::runtime::DispatchCtx;
use tinyflows_bus::{GraphValidation, ImportGraphResult, ValidateGraphRequest};

// The refusal code remains stable on OpenHuman's test surface.
#[cfg(test)]
pub(crate) use tinyflows::compat::UNSUPPORTED_NESTED_CONDITIONAL_FAN_IN;

async fn validate_over_bus(
    ctx: &DispatchCtx,
    config: &Config,
    graph: Value,
) -> Result<GraphValidation, String> {
    let registration = crate::flows::authoring_host::register(Arc::new(config.clone()), ctx.clone()).await?;
    let proxy = crate::modules::tinyflows::authoring_proxy(config).await?;
    proxy
        .call(
            tinyflows_bus::methods::VALIDATE_GRAPH,
            &(ValidateGraphRequest {
                graph,
                host: Some(registration.authoring.clone()),
            },),
        )
        .await
        .map_err(|error| error.wire_message())
}

/// Validates and migrates a graph through TinyFlows, resolving saved children
/// through the host callback bound to the current invocation scope.
pub(crate) async fn validate_and_migrate_graph(
    ctx: &DispatchCtx,
    config: &Config,
    graph: Value,
) -> Result<WorkflowGraph, String> {
    let report = validate_over_bus(ctx, config, graph).await?;
    if let Some(error) = report.errors.first() {
        return Err(error.message.clone());
    }
    report
        .graph
        .ok_or_else(|| "TinyFlows validation returned no graph".to_string())
        .and_then(|graph| serde_json::from_value(graph).map_err(|error| error.to_string()))
}

/// Host-aware compatibility errors for graphs that can reference saved flows.
pub(crate) async fn config_aware_engine_compatibility_errors(
    ctx: &DispatchCtx,
    config: &Config,
    graph: Value,
) -> Result<Vec<String>, String> {
    let report = validate_over_bus(ctx, config, graph).await?;
    Ok(report
        .errors
        .into_iter()
        .filter(|error| {
            error.code.starts_with("unsupported_")
                || error.code.starts_with("incompatible_referenced_workflow")
        })
        .map(|error| format!("{}: {}", error.code, error.message))
        .collect())
}

pub(super) async fn ensure_config_aware_engine_compatible(
    ctx: &DispatchCtx,
    config: &Config,
    graph: &WorkflowGraph,
) -> Result<(), String> {
    let graph = serde_json::to_value(graph).map_err(|error| error.to_string())?;
    match config_aware_engine_compatibility_errors(ctx, config, graph).await?
        .into_iter()
        .next()
    {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// Validates a candidate graph without persisting it and reports all errors
/// with host trigger warnings added after successful engine validation.
pub async fn flows_validate(
    ctx: &DispatchCtx,
    config: &Config,
    graph_json: Value,
) -> Outcome<tinyflows_catalog::FlowValidation> {
    use tinyflows_catalog::{FlowValidation, FlowValidationError};
    let report = match validate_over_bus(ctx, config, graph_json).await {
        Ok(report) => report,
        Err(error) => {
            return Outcome::single_log(
                FlowValidation {
                    valid: false,
                    errors: vec![error.clone()],
                    error_details: vec![FlowValidationError {
                        code: "authoring_unavailable".into(),
                        message: error,
                        node_id: None,
                        field: None,
                    }],
                    warnings: Vec::new(),
                },
                "flow validation failed",
            );
        }
    };
    let errors = report
        .errors
        .into_iter()
        .map(|error| FlowValidationError {
            code: error.code,
            message: error.message,
            node_id: error.node_id,
            field: error.field,
        })
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        let messages = errors.iter().map(|error| error.message.clone()).collect();
        return Outcome::single_log(
            FlowValidation {
                valid: false,
                errors: messages,
                error_details: errors,
                warnings: Vec::new(),
            },
            "flow validation failed",
        );
    }
    let graph: WorkflowGraph = match report.graph.ok_or_else(|| "TinyFlows validation returned no graph".to_string())
        .and_then(|graph| serde_json::from_value(graph).map_err(|error| error.to_string()))
    {
        Ok(graph) => graph,
        Err(error) => {
            return Outcome::single_log(
                FlowValidation {
                    valid: false,
                    errors: vec![error.clone()],
                    error_details: vec![FlowValidationError {
                        code: "unparseable_graph".into(),
                        message: error,
                        node_id: None,
                        field: None,
                    }],
                    warnings: Vec::new(),
                },
                "flow validation failed",
            );
        }
    };
    let warnings = graph_trigger_warnings(&graph);
    Outcome::single_log(
        FlowValidation {
            valid: true,
            errors: Vec::new(),
            error_details: Vec::new(),
            warnings,
        },
        "flow validated",
    )
}

/// Imports and validates native or n8n graph JSON through TinyFlows without
/// persisting it.
pub async fn flows_import(
    _ctx: &DispatchCtx,
    config: &Config,
    graph_json: Value,
    format: Option<String>,
) -> Result<Outcome<tinyflows_catalog::FlowImport>, String> {
    use tinyflows_catalog::FlowImport;
    let requested = format.as_deref().unwrap_or("auto").trim().to_ascii_lowercase();
    if !matches!(requested.as_str(), "auto" | "" | "native" | "tinyflows" | "n8n") {
        return Err(format!(
            "unknown import format '{requested}' (expected 'native' or 'n8n')"
        ));
    }
    let proxy = crate::modules::tinyflows::authoring_proxy(config).await?;
    let imported: ImportGraphResult = proxy
        .call(
            tinyflows_bus::methods::IMPORT_GRAPH,
            &(tinyflows_bus::ImportGraphRequest {
                graph: graph_json,
                format: Some(requested),
            },),
        )
        .await
        .map_err(|error| error.wire_message())?;
    if let Some(error) = imported.errors.first() {
        return Err(error.message.clone());
    }
    let graph: WorkflowGraph = imported
        .graph
        .ok_or_else(|| "TinyFlows import returned no graph".to_string())
        .and_then(|graph| serde_json::from_value(graph).map_err(|error| error.to_string()))?;
    let mut warnings = imported.warnings;
    warnings.extend(graph_trigger_warnings(&graph));
    Ok(Outcome::single_log(FlowImport { graph, warnings }, "flow imported"))
}

/// Compatibility diagnostics for a portable Copilot graph payload.
pub(crate) async fn engine_compatibility_errors_json(
    ctx: &DispatchCtx,
    config: &Config,
    graph: Value,
) -> Result<Vec<String>, String> {
    config_aware_engine_compatibility_errors(ctx, config, graph).await
}
