//! Registry of in-flight asynchronous sub-agents and their cancellation lifecycle.
//!
//! `spawn_subagent` runs a child as a detached `tokio` task. On its own
//! that task is opaque: the parent gets a `task_id` back but has no channel into
//! the running loop and no way to collect the result inline. This registry
//! closes both gaps.
//!
//! The registry tracks task metadata, status and abort handles for cancellation.
//!
//! TinyAgents owns the process-local watch/cancel/abort/steering mechanics.
//! OpenHuman retains product metadata and durable task-store projection.
//!
//! ## Typed lifecycle ledger (issue #4249)
//!
//! Alongside the executor plumbing (abort handle + steering queue + watch
//! status), every detached sub-agent is also recorded in a process-wide
//! [`tinyagents` orchestration `TaskStore`](tinyagents_tasks)
//! as an `OrchestrationTaskKind::SubAgent` task. `register` inserts it
//! (`Pending` → `Running`) and spawns a watcher that mirrors the child's
//! terminal status into the store (`Completed`/`Failed`/`Awaiting`); the cancel
//! paths record `Cancelled`. This gives a typed, queryable lifecycle
//! (`task_records`) alongside the crate-owned runtime registry.
//!
//! ## Module layout
//!
//! - [`registry`] — the in-process table itself: [`DetachedSubagentStatus`],
//!   registration, and status channels.
//! - [`task_ledger`] — the durable per-workspace task store and the typed
//!   lifecycle records mirrored into it.
//! - [`roster`] — read-only snapshots for prompt injection and control tools.
//! - [`resolve`] — mapping a durable session id (or bare task id) to the live
//!   registry entry or a durable fallback record.
//! - [`cancel`] — aborting sub-agents by task, session, thread, or all at once.

mod cancel;
mod registry;
mod resolve;
mod roster;
mod task_ledger;

#[cfg(test)]
#[path = "running_subagents_tests.rs"]
mod tests;

#[cfg(test)]
pub(crate) use cancel::cancel_by_task;
#[cfg(test)]
pub(crate) use cancel::prune;
#[cfg(test)]
pub(crate) use cancel::{caller_workspace_in, CallerWorkspace};
pub(crate) use cancel::{
    cancel_all, cancel_by_session_in_workspace, cancel_by_task_with_workspace_reason,
    cancel_for_thread, stop_for_thread, CancelledSubagent,
};
pub(crate) use registry::admit_spawn;
#[cfg(test)]
pub(crate) use registry::{
    register, register_with_parent_epoch, status_channel, RegistrationError,
};
pub(crate) use roster::active_subagents_context_block;
#[cfg(test)]
pub(crate) use task_ledger::task_records;
pub(crate) use task_ledger::{
    reconcile_orphaned_tasks_on_boot, task_store_for_workspace,
};
