//! Bundled prompt assets and system-prompt rendering pipeline. See
//! [`README.md`](README.md) for the section/builder model and the
//! `agent::prompts`.

pub mod types;
pub use types::*;
mod connected_identities;
pub use connected_identities::render_connected_identities;

pub mod agents_md;
pub use agents_md::{AGENTS_MD_FILENAME, AgentsMdContent, load_agents_md, load_agents_md_layers};

pub mod builder;
pub use builder::{GLOBAL_STYLE_SUFFIX, SystemPromptBuilder, TieredPrompt};

pub mod sections;
pub use sections::*;

#[path = "render_helpers/section_renderers.rs"]
mod section_renderers;
#[path = "render_helpers/subagent.rs"]
mod subagent;
#[path = "render_helpers/workspace_files.rs"]
mod workspace_files;

pub use section_renderers::{
    current_datetime_line, render_ambient_environment, render_datetime, render_identity,
    render_runtime, render_safety, render_tools, render_user_identity, render_workspace,
    turn_preamble,
};
pub(crate) use subagent::harness_json_tool_prompt;
pub use subagent::{render_subagent_system_prompt, render_subagent_system_prompt_with_format};
pub(crate) use workspace_files::write_agents_md_blocks;
pub use workspace_files::{
    default_workspace_file_content, inject_inline_content, inject_workspace_file,
    inject_workspace_file_capped, sync_workspace_file,
};

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
