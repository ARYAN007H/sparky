//! The `workflow_builder` built-in agent: this host's wiring of the tinyflows
//! authoring copilot.
//!
//! `prompt` assembles the workflow-builder archetype with this host's runtime
//! sections, while the asynchronous `flows.build` operation asks the compiled
//! TinyFlows module to render per-turn builder briefs.

pub mod prompt;
