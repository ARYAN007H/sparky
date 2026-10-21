mod ops;
mod storage_schema;
mod store;
pub use storage_schema::storage_schema;
mod store_documents;
mod tiny_port;
mod types;

pub use ops::{
    action_root_key, cancel_driver_job_by_task, cancel_driver_job_for_observed_session,
    cancel_driver_jobs_for_thread, close_if_current, find_reusable, list_for_parent,
    mark_driver_pause_cancelled, mark_failed, mark_finished, normalize_task_key,
    resolve_lifecycle_key, reuse_decision, upsert_running, DriverTaskCancellation,
};
pub(crate) use tiny_port::OpenHumanTinySubagentSessionStore;
pub use types::{
    DurableSubagentStatus, SubagentSessionSelector, SubagentSessionStore, SubagentSessionUpsert,
};
// Test-only: the host keeps the raw durable row private to its persistence
// adapter and exposes scoped operations to production callers.
#[cfg(test)]
pub use types::DurableSubagentSession;
