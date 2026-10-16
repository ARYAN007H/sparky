//! Global context management for agent sessions.
//!
//! This module is the single home for everything that shapes what an LLM
//! sees during a conversation:
//!
//! 1. **System prompt assembly** — [`prompt::SystemPromptBuilder`] and its
//!    composable [`prompt::PromptSection`] trait. Main agents, sub-agents,
//!    and channels all build their opening system prompts through this
//!    module; there is no parallel implementation elsewhere in the crate.
//!
//! 2. **Mechanical history bookkeeping** — [`stats::ContextStatsState`] records
//!    provider usage. Live reduction runs in the TinyAgents middleware stack.
//!
//! Agents hold a single [`ContextManager`] per session. The manager owns
//! per-conversation state (budget, utilisation) while prompt assembly remains
//! centralized here.
//!
//! Submodules are added incrementally as the `agent/` → `context/`
//! migration lands (see plan `misty-bubbling-bunny.md`).

pub mod channels_prompt;
pub mod manager;
pub mod stats;

pub use manager::{ContextManager, ContextStats};
/// Default number of most-recent tool-result envelopes the TinyAgents
/// microcompact middleware leaves intact.
pub const DEFAULT_KEEP_RECENT_TOOL_RESULTS: usize = 5;
