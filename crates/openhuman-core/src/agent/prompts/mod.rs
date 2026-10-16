//! Bundled prompt assets and system-prompt rendering pipeline. See
//! [`README.md`](README.md) for the section/builder model and the
//! `agent::prompts`.

pub mod types;
pub use types::*;

pub mod agents_md;
pub use agents_md::{AGENTS_MD_FILENAME, AgentsMdContent, load_agents_md, load_agents_md_layers};

pub mod builder;
pub use builder::{GLOBAL_STYLE_SUFFIX, SystemPromptBuilder, TieredPrompt};

pub(crate) use subagent::harness_json_tool_prompt;
pub use subagent::{render_subagent_system_prompt, render_subagent_system_prompt_with_format};

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
