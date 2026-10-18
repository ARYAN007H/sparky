//! Global context management for agent sessions.
//!
//! This module is the single home for everything that shapes what an LLM
//! sees during a conversation:
//!
//! 1. **System prompt composition** — OpenHuman selects product content through
//!    [`prompt::SystemPromptBuilder`], while TinyAgents owns portable section
//!    and tier assembly. Main agents, sub-agents, and channels all build their
//!    opening system prompts through this module.
//!
//! 2. **Middleware configuration** — `ContextConfig` configures the TinyAgents
//!    turn middleware stack.
//!
//! Session hosts hold their prompt builder and context settings directly;
//! TinyAgents owns live transcript reduction and middleware execution.

/// Default number of most-recent tool-result envelopes the TinyAgents
/// microcompact middleware leaves intact.
pub const DEFAULT_KEEP_RECENT_TOOL_RESULTS: usize = 5;
