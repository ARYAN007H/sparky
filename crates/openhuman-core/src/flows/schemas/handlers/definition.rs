use crate::core::runtime::DispatchCtx;
use super::{read_required, to_json};
use super::super::*;

pub(super) fn handle_create(ctx: DispatchCtx, params: Map<String, Value>) -> ControllerFuture {
    Box::pin(async move {
        let config = ctx.load_config().await?;
        let name = read_required::<String>(&params, "name")?;
        let graph = read_required::<Value>(&params, "graph")?;
        let require_approval = params
            .get("require_approval")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        // Opt-in strict mode (F3): run the same author hard-gates an agent save
        // must pass, before persisting. Default off — the human canvas save
        // path stays permissive.
        if params
            .get("strict")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            ops::strict_gate(&ctx, &config, &graph).await?;
        }
        to_json(ops::flows_create(&config, name, graph, require_approval).await?)
    })
}

pub(super) fn handle_validate(_ctx: DispatchCtx, params: Map<String, Value>) -> ControllerFuture {
    Box::pin(async move {
        // No config load: validation is pure (no persistence, no workspace).
        let graph = read_required::<Value>(&params, "graph")?;
        to_json(ops::flows_validate(graph))
    })
}

pub(super) fn handle_import(_ctx: DispatchCtx, params: Map<String, Value>) -> ControllerFuture {
    Box::pin(async move {
        // No config load: import is pure (no persistence, no workspace).
        let graph = read_required::<Value>(&params, "graph")?;
        let format = params
            .get("format")
            .filter(|v| !v.is_null())
            .map(|v| serde_json::from_value::<String>(v.clone()))
            .transpose()
            .map_err(|e| format!("invalid 'format': {e}"))?;
        to_json(ops::flows_import(graph, format)?)
    })
}

pub(super) fn handle_duplicate(ctx: DispatchCtx, params: Map<String, Value>) -> ControllerFuture {
    Box::pin(async move {
        let config = ctx.load_config().await?;
        let id = read_required::<String>(&params, "id")?;
        to_json(ops::flows_duplicate(&config, id.trim()).await?)
    })
}

pub(super) fn handle_get(ctx: DispatchCtx, params: Map<String, Value>) -> ControllerFuture {
    Box::pin(async move {
        let config = ctx.load_config().await?;
        let id = read_required::<String>(&params, "id")?;
        to_json(ops::flows_get(&config, id.trim()).await?)
    })
}

pub(super) fn handle_list(ctx: DispatchCtx, _params: Map<String, Value>) -> ControllerFuture {
    Box::pin(async move {
        let config = ctx.load_config().await?;
        to_json(ops::flows_list(&config).await?)
    })
}

pub(super) fn handle_list_connections(
    ctx: DispatchCtx,
    _params: Map<String, Value>,
) -> ControllerFuture {
    Box::pin(async move {
        let config = ctx.load_config().await?;
        to_json(ops::flows_list_connections(&ctx, &config).await?)
    })
}

pub(super) fn handle_update(ctx: DispatchCtx, params: Map<String, Value>) -> ControllerFuture {
    Box::pin(async move {
        let config = ctx.load_config().await?;
        let id = read_required::<String>(&params, "id")?;
        let name = params
            .get("name")
            .filter(|v| !v.is_null())
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()
            .map_err(|e| format!("invalid 'name': {e}"))?;
        let graph = params.get("graph").filter(|v| !v.is_null()).cloned();
        let require_approval = params.get("require_approval").and_then(Value::as_bool);
        let expected_version = params
            .get("expected_version")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        // Opt-in strict mode (F3): when a new graph is supplied, run the same
        // author hard-gates an agent save must pass, before persisting.
        if params
            .get("strict")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            if let Some(graph_json) = graph.as_ref() {
                ops::strict_gate(&ctx, &config, graph_json).await?;
            }
        }
        to_json(
            ops::flows_update(
                &config,
                id.trim(),
                name,
                graph,
                require_approval,
                expected_version,
            )
            .await?,
        )
    })
}

pub(super) fn handle_delete(ctx: DispatchCtx, params: Map<String, Value>) -> ControllerFuture {
    Box::pin(async move {
        let config = ctx.load_config().await?;
        let id = read_required::<String>(&params, "id")?;
        to_json(ops::flows_delete(&config, id.trim()).await?)
    })
}

pub(super) fn handle_set_enabled(ctx: DispatchCtx, params: Map<String, Value>) -> ControllerFuture {
    Box::pin(async move {
        let config = ctx.load_config().await?;
        let id = read_required::<String>(&params, "id")?;
        let enabled = params
            .get("enabled")
            .and_then(Value::as_bool)
            .ok_or_else(|| "missing required param 'enabled'".to_string())?;
        to_json(ops::flows_set_enabled(&config, id.trim(), enabled).await?)
    })
}
