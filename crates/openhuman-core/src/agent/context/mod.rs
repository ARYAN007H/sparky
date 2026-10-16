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
//! 2. **Middleware configuration** — `ContextConfig` configures the TinyAgents
//!    turn middleware stack.
//!
//! Session hosts hold their prompt builder and context settings directly;
//! TinyAgents owns live transcript reduction and middleware execution.

/// Default number of most-recent tool-result envelopes the TinyAgents
/// microcompact middleware leaves intact.
pub const DEFAULT_KEEP_RECENT_TOOL_RESULTS: usize = 5;
