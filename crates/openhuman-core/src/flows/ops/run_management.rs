use super::*;

use tinyflows_catalog::run_management as rm;
use tinyflows_bus::catalog::{self as wire, FlowRun as WireFlowRun};

/// Cancel a workflow run through its owner-scoped Runs.CancelRun operation.
pub async fn flows_cancel_run(config: &Config, run_id: &str) -> Result<Outcome<Value>, String> {
    let ctx = crate::core::runtime::DispatchCtx::ambient();
    let storage = crate::flows::catalog_storage::register(&ctx, config)
        .await?
        .storage();
    let run = get_run_with_storage(config, storage.clone(), run_id)
        .await?
        .ok_or_else(|| format!("flow run '{run_id}' not found"))?;
    if rm::is_terminal_run_status(&run.status) {
        return Err(rm::already_terminal_error(run_id, &run.status));
    }

    let (signalled, was_in_flight) =
        crate::flows::runs_adapter::cancel_run(config, storage.clone(), run_id).await?;
    if !signalled {
        let current = get_run_with_storage(config, storage, run_id).await?;
        if current
            .as_ref()
            .is_some_and(|current| rm::is_terminal_run_status(&current.status))
        {
            return Err(rm::settled_concurrently_error(run_id));
        }
        return Err(format!("flow run '{run_id}' cancellation was not accepted"));
    }

    let result = json!({
        "run_id": run_id,
        "cancelled": true,
        "was_in_flight": was_in_flight,
    });
    let log = if was_in_flight {
        format!("flow run {run_id} cancellation requested")
    } else {
        format!("flow run {run_id} cancelled")
    };
    Ok(Outcome::single_log(result, log))
}

/// List the most recent runs for one workflow. Expiry remains a best-effort
/// on-read recovery operation, owned by the Runs module.
pub async fn flows_list_runs(
    config: &Config,
    flow_id: &str,
    limit: usize,
) -> Result<Outcome<Vec<FlowRun>>, String> {
    expire_parked_on_read(config).await;
    let ctx = crate::core::runtime::DispatchCtx::ambient();
    let storage = crate::flows::catalog_storage::register(&ctx, config)
        .await?
        .storage();
    let request = wire::ListRunsRequest {
        storage,
        flow_id: flow_id.to_owned(),
        limit,
    };
    let runs: Vec<WireFlowRun> = catalog_call(config, wire::members::LIST_RUNS, request).await?;
    let runs = runs
        .into_iter()
        .map(crate::flows::store::roundtrip)
        .collect::<anyhow::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    Ok(Outcome::single_log(
        runs,
        format!("flow runs listed: {flow_id}"),
    ))
}

/// List the most recent runs across the acting storage scope.
pub async fn flows_list_all_runs(
    config: &Config,
    limit: usize,
) -> Result<Outcome<Vec<FlowRun>>, String> {
    expire_parked_on_read(config).await;
    let ctx = crate::core::runtime::DispatchCtx::ambient();
    let storage = crate::flows::catalog_storage::register(&ctx, config)
        .await?
        .storage();
    let request = wire::ListAllRunsRequest { storage, limit };
    let runs: Vec<WireFlowRun> =
        catalog_call(config, wire::members::LIST_ALL_RUNS, request).await?;
    let runs = runs
        .into_iter()
        .map(crate::flows::store::roundtrip)
        .collect::<anyhow::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    let count = runs.len();
    Ok(Outcome::single_log(
        runs,
        format!("all flow runs listed: {count} run(s)"),
    ))
}

/// Prune terminal run history while retaining the module's standard cap and
/// every live or approval-parked run.
pub async fn flows_prune_runs(config: &Config, flow_id: &str) -> Result<Outcome<Value>, String> {
    let keep = tinyflows_catalog::store::MAX_FLOW_RUNS_PER_FLOW;
    let ctx = crate::core::runtime::DispatchCtx::ambient();
    let storage = crate::flows::catalog_storage::register(&ctx, config)
        .await?
        .storage();
    let request = wire::PruneRunsRequest {
        storage,
        flow_id: flow_id.to_owned(),
        keep,
    };
    let pruned: usize = catalog_call(config, wire::members::PRUNE_RUNS, request).await?;
    tracing::info!(target: "flows", flow_id, pruned, keep, "[flows] flows_prune_runs: manual retention sweep");
    Ok(Outcome::single_log(
        json!({ "flow_id": flow_id, "pruned": pruned, "kept": keep }),
        format!("flow runs pruned: {flow_id} ({pruned} removed)"),
    ))
}

/// Load one run after the module-owned lazy approval-expiry check.
pub async fn flows_get_run(config: &Config, run_id: &str) -> Result<Outcome<FlowRun>, String> {
    expire_parked_on_read(config).await;
    let ctx = crate::core::runtime::DispatchCtx::ambient();
    let storage = crate::flows::catalog_storage::register(&ctx, config)
        .await?
        .storage();
    let run = get_run_with_storage(config, storage, run_id)
        .await?
        .ok_or_else(|| format!("flow run '{run_id}' not found"))?;
    Ok(Outcome::single_log(
        run,
        format!("flow run loaded: {run_id}"),
    ))
}

async fn expire_parked_on_read(config: &Config) {
    let ctx = crate::core::runtime::DispatchCtx::ambient();
    crate::flows::recovery::expire_parked_runs(&ctx, config).await;
}

async fn get_run_with_storage(
    config: &Config,
    storage: tinyflows_bus::catalog::CatalogStorage,
    run_id: &str,
) -> Result<Option<FlowRun>, String> {
    let request = wire::RunIdRequest {
        storage,
        id: run_id.to_owned(),
    };
    let run: Option<WireFlowRun> = catalog_call(config, wire::members::GET_RUN, request).await?;
    run.map(crate::flows::store::roundtrip)
        .transpose()
        .map_err(|error| error.to_string())
}

async fn catalog_call<T, A>(
    config: &Config,
    member: &'static str,
    args: A,
) -> Result<T, String>
where
    T: serde::de::DeserializeOwned + Send + 'static,
    A: serde::Serialize + Send + 'static,
{
    let proxy = crate::modules::tinyflows::catalog_proxy(config).await?;
    proxy
        .call(member, (args,))
        .await
        .map_err(|error| error.wire_message())
}
