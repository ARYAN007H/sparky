//! Bundled prompt assets and system-prompt rendering pipeline. See
//! [`README.md`](README.md) for the section/builder model and the
//! `agent::prompts`.

pub mod types;
pub use types::*;

pub mod agents_md;
pub use agents_md::{load_agents_md, load_agents_md_layers, AgentsMdContent, AGENTS_MD_FILENAME};

pub mod builder;
pub use builder::{SystemPromptBuilder, TieredPrompt, GLOBAL_STYLE_SUFFIX};

#[path = "render_helpers/subagent.rs"]
mod subagent;
pub(crate) use subagent::harness_json_tool_prompt;
pub use subagent::{render_subagent_system_prompt, render_subagent_system_prompt_with_format};

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
