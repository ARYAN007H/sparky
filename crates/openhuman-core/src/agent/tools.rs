//! Agent-owned dialogue and control tools.
//!
//! These tools act on the agent loop or its todo list rather than on files,
//! memory, or the network. Wire names are
//! given in parentheses where they differ from the type name:
//!
//! - `AskClarificationTool` (`ask_user_clarification`, from `tinyagents_harness::tools`) — returns the
//!   question as its output; the turn actually pauses only because callers
//!   list this name in the harness seam's `early_exit_tools`.
//! - [`PlanExitTool`] — ends a plan-mode pass by returning the plan plus
//!   [`PLAN_EXIT_MARKER`], which TinyAgents consumes to switch to build mode.
//! - `RunWorkflowTool` / `AwaitWorkflowTool` — spawn a
//!   `crate::skills::runtime` workflow run and wait on its outcome. Compiled
//!   in only with the `skills` feature, so builds without it omit both tools
//!   from the catalog.
//! - [`TodoTool`] — the session's todo list (whole-list write, thread-scoped).
//!
//! `agent::registration` assembles the dialogue tools; `tools::ops` composes
//! domain registrations and applies the runtime capability filters.
pub(crate) mod plan_exit;
mod todo;

pub use plan_exit::PlanExitTool;
pub const PLAN_EXIT_MARKER: &str = tinyagents_harness_tools::PLAN_EXIT_MARKER;
pub use todo::TodoTool;
pub(crate) use todo::TodoToolDispatch;
