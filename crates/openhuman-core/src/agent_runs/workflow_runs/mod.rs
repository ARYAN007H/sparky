//! Durable dynamic workflow run RPC surface.

#[path = "../../agent/orchestration/workflow_runs/host.rs"]
mod host;
#[path = "../../agent/orchestration/workflow_runs/engine/cancel.rs"]
mod cancel;
#[path = "../../agent/orchestration/workflow_runs/engine/lifecycle.rs"]
mod lifecycle;
const LOG_TARGET: &str = "workflow_run_engine";
mod ops;
mod schemas;

pub(crate) use lifecycle::{resume_workflow_run, start_workflow_run, stop_workflow_run};
pub(crate) use ops::{
    PARALLEL_RESEARCH_ID, builtin_definitions, definition_by_id, get_run, list_definitions,
    list_runs,
};
pub(crate) use schemas::{
    all_controller_schemas as all_workflow_run_controller_schemas,
    all_registered_controllers as all_workflow_run_registered_controllers,
};
