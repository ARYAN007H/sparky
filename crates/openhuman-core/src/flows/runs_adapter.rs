//! OpenHuman adapters for TinyFlows' module-owned run lifecycle.
//!
//! The module owns execution, durable rows, checkpoints, cancellation, and
//! dedup settlement. This boundary retains scope-bound host callbacks,
//! projects ordered PollRun events into OpenHuman events, and preserves the
//! existing synchronous run result shape.

mod projection;
mod session;

use std::sync::Arc;

use serde_json::{Value, json};
use tinyflows_bus::runs::{self, ResumeRun, StartRun};
use tinyflows_catalog::{Flow, types::FlowRunTrigger};

use crate::config::Config;
use crate::core::runtime::DispatchCtx;
use crate::flows::catalog_storage::{self, CatalogStorageRegistration};
use crate::flows::caps_host::{self, CapsHostRegistration};
use crate::modules::tinyflows as module;

const RUN_TIMEOUT_MS: u64 = 600_000;

pub(crate) use session::{RunKey, RunSession, run_bindings};

/// Callback authorities retained for one owner-scoped flow run.
pub(crate) struct RunBindings {
    catalog: Arc<CatalogStorageRegistration>,
    caps: Arc<CapsHostRegistration>,
    pub(crate) flow_id: String,
    pub(crate) run_id: String,
    pub(crate) require_approval: bool,
}

impl RunBindings {
    /// Stable owner scope used by every Catalog and Runs request.
    pub(crate) fn storage(&self) -> tinyflows_bus::catalog::CatalogStorage {
        self.catalog.storage()
    }

    /// Fresh callback bound to this run's flow, owner, and approval policy.
    pub(crate) fn caps_host(&self) -> runs::CapsHost {
        self.caps.address()
    }

    pub(crate) fn catalog_registration(&self) -> Arc<CatalogStorageRegistration> {
        Arc::clone(&self.catalog)
    }
}

/// A settled or parked engine segment, in the legacy controller vocabulary.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RunOutcome {
    pub(crate) run_id: String,
    pub(crate) output: Value,
    pub(crate) pending_approvals: Vec<String>,
    pub(crate) status: String,
}

/// Typed result returned to the existing `flows_resume` controller.
pub(crate) type ResumeOutcome = RunOutcome;

/// Retain one run's owner callback and its per-run capability callback.
pub(crate) async fn register_bindings(
    ctx: &DispatchCtx,
    config: &Config,
    flow_id: &str,
    run_id: &str,
    require_approval: bool,
) -> Result<Arc<RunBindings>, String> {
    let catalog = catalog_storage::register(ctx, config).await?;
    register_with_catalog(
        ctx,
        config,
        flow_id,
        run_id,
        require_approval,
        catalog,
    )
    .await
}

async fn register_with_catalog(
    ctx: &DispatchCtx,
    config: &Config,
    flow_id: &str,
    run_id: &str,
    require_approval: bool,
    catalog: Arc<CatalogStorageRegistration>,
) -> Result<Arc<RunBindings>, String> {
    let caps = caps_host::serve(
        ctx.clone(),
        Arc::new(config.clone()),
        flow_id.to_owned(),
        run_id.to_owned(),
        require_approval,
        Arc::clone(&catalog),
    )
    .await?;
    Ok(Arc::new(RunBindings {
        catalog,
        caps: Arc::new(caps),
        flow_id: flow_id.to_owned(),
        run_id: run_id.to_owned(),
        require_approval,
    }))
}

/// Start a module-owned run and subscribe before its first PollRun projection.
pub(crate) async fn start_run(
    ctx: &DispatchCtx,
    config: &Config,
    flow: &Flow,
    run_id: &str,
    input: Value,
    inputs: serde_json::Map<String, Value>,
    trigger: FlowRunTrigger,
) -> Result<(Arc<RunSession>, tokio::sync::mpsc::UnboundedReceiver<runs::RunEvent>), String> {
    let bindings = register_bindings(
        ctx,
        config,
        &flow.id,
        run_id,
        flow.require_approval,
    )
    .await?;
    let session = RunSession::new(
        Arc::clone(&bindings),
        Arc::new(config.clone()),
        flow.clone(),
        trigger,
        0,
    );
    let receiver = session.subscribe();
    session::insert(Arc::clone(&session))?;
    let proxy = match module::runs_proxy(config).await {
        Ok(proxy) => proxy,
        Err(error) => {
            session::remove(&session);
            return Err(error);
        }
    };
    let request = StartRun {
        run_id: run_id.to_owned(),
        context: runs::RunContext {
            flow_id: flow.id.clone(),
            origin: Some(json!({
                "kind": "workflow",
                "flow_id": flow.id.clone(),
                "require_approval": flow.require_approval,
            })),
        },
        caps_host: bindings.caps_host(),
        storage: bindings.storage(),
        graph: serde_json::to_value(&flow.graph)
            .map_err(|error| format!("encode workflow graph: {error}"))?,
        trigger_kind: bus_trigger(trigger),
        trigger_data: input,
        inputs,
        timeout_ms: RUN_TIMEOUT_MS,
    };
    let started: runs::StartRunResponse = match proxy
        .call(runs::methods::START_RUN, (request,))
        .await
    {
        Ok(started) => started,
        Err(error) => {
            session::remove(&session);
            return Err(error.wire_message());
        }
    };
    if started.already_running {
        session::remove(&session);
        return Err(format!("flow run '{run_id}' is already running"));
    }
    session.start_polling(proxy);
    Ok((session, receiver))
}

/// Wait for the first parked or terminal outcome after a run is admitted.
pub(crate) async fn wait_for_outcome(
    receiver: &mut tokio::sync::mpsc::UnboundedReceiver<runs::RunEvent>,
    run_id: &str,
    after_seq: u64,
) -> Result<RunOutcome, String> {
    loop {
        let event = receiver
            .recv()
            .await
            .ok_or_else(|| "flow run event stream closed before settlement".to_string())?;
        if event.seq <= after_seq {
            continue;
        }
        match event.kind.as_str() {
            "parked" => {
                let parked: runs::RunParked = serde_json::from_value(event.data)
                    .map_err(|error| format!("decode parked run outcome: {error}"))?;
                return Ok(RunOutcome {
                    run_id: run_id.to_owned(),
                    output: parked.output.ok_or_else(|| {
                        "TinyFlows omitted the parked engine outcome".to_string()
                    })?,
                    pending_approvals: parked.pending_approvals,
                    status: "pending_approval".into(),
                });
            }
            "finished" => {
                let finished: runs::RunFinished = serde_json::from_value(event.data)
                    .map_err(|error| format!("decode finished run outcome: {error}"))?;
                return finished_outcome(finished, run_id);
            }
            _ => {}
        }
    }
}

fn finished_outcome(
    finished: runs::RunFinished,
    run_id: &str,
) -> Result<RunOutcome, String> {
    if finished.timed_out {
        return Err(finished
            .error
            .unwrap_or_else(|| "flow run timed out".to_string()));
    }
    if finished.status == "cancelled" {
        return Ok(RunOutcome {
            run_id: run_id.to_owned(),
            output: Value::Null,
            pending_approvals: Vec::new(),
            status: finished.status,
        });
    }
    let output = finished.output.ok_or_else(|| {
        finished
            .error
            .clone()
            .unwrap_or_else(|| format!("flow run ended as {} without an engine outcome", finished.status))
    })?;
    Ok(RunOutcome {
        run_id: run_id.to_owned(),
        output,
        pending_approvals: Vec::new(),
        status: finished.status,
    })
}

fn bus_trigger(trigger: FlowRunTrigger) -> runs::FlowRunTrigger {
    match trigger {
        FlowRunTrigger::Rpc => runs::FlowRunTrigger::Rpc,
        FlowRunTrigger::Schedule => runs::FlowRunTrigger::Schedule,
        FlowRunTrigger::AppEvent => runs::FlowRunTrigger::AppEvent,
        FlowRunTrigger::Resume => runs::FlowRunTrigger::Resume,
    }
}
