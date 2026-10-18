//! Built-in agent archetypes.
//!
//! Each submodule below is one shipped agent. Most own a TOML definition and
//! a prompt builder that includes a sibling Markdown archetype; the flow
//! builders receive their archetype text from TinyFlows Copilot over the bus.
//!
//! * `agent.toml`  — id, `when_to_use`, model, tool scope, sandbox mode,
//!   iteration cap, tier, and `omit_*` flags. Parsed directly into
//!   [`crate::agent::harness::host_definition::AgentDefinition`]; ships without
//!   a `system_prompt`.
//! * `prompt.md`   — the static archetype body used by builders that embed it.
//! * `prompt.rs`   — exposes `pub fn build(&PromptContext) ->
//!   anyhow::Result<String>`, which appends runtime-dependent sections
//!   (rendered tool list, workspace) to the archetype body.
//!   [`BUILTINS`] installs it as `PromptSource::Dynamic` on the parsed
//!   definition. Most archetypes keep a `prompt_tests.rs` beside it.
//!
//! `loader.rs` holds [`BUILTINS`], [`load_builtins`] and
//! [`validate_tier_hierarchy`]. The slice also registers archetypes that
//! live with other domains (`skills/*/agent/`), so this directory is not the
//! full built-in set. The
//! package `README.md` one level up describes what each archetype does.

mod loader;

#[cfg(feature = "flows")]
mod flow_prompts;

#[cfg(test)]
#[path = "fleet_prompt_tests.rs"]
mod fleet_prompt_tests;

pub mod critic;
pub mod image_agent;
pub mod morning_briefing;
pub mod orchestrator;
pub mod planner;
pub mod presentation_agent;
pub mod summarizer;
pub mod task_manager_agent;
pub mod trigger_reactor;
pub mod trigger_triage;
pub mod video_agent;
pub mod vision_agent;
#[cfg(feature = "flows")]
pub mod flow_discovery;
#[cfg(feature = "flows")]
pub mod workflow_builder;

pub use loader::{load_builtins, validate_tier_hierarchy, BuiltinAgent, BUILTINS};
