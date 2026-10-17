//! System-prompt helpers for sub-agent typed mode.
//!
//! Includes the artifact-offload contract and the tool-spec deduplication
//! helper used before sending specs to the provider.

use crate::agent::harness::artifact_offload::OFFLOAD_WRITE_TOOL;
use std::collections::HashSet;
use tinyagents_harness::artifacts::{
    render_artifact_offload_contract, should_render_offload_contract, ARTIFACT_OFFLOAD_HEADING,
};
use tinytools::ToolSpec;

/// Compose the leaf role framing with the host's artifact-tool prompt policy.
pub(crate) fn compose_subagent_system_prompt(
    base_prompt: String,
    agent_id: &str,
    visible_tool_names: &HashSet<String>,
    task_prompt: &str,
    spawn_depth: usize,
) -> String {
    let framed = format!(
        "{}\n\n{}",
        base_prompt.trim_end(),
        tinyagents_orchestration::subagent::subagent_framing(
            tinyagents_orchestration::subagent::SubagentRole::Leaf,
            spawn_depth,
            task_prompt,
        )
    );
    append_artifact_offload_contract(framed, agent_id, visible_tool_names)
}

/// Append the filesystem-offload contract (#3883), but only for a sub-agent
/// that actually holds a file-write tool.
///
/// A sub-agent's system prompt may only name tools it can really
/// call — a skill-filtered specialist sees just its own toolkit. Telling it to write a file yields
/// hallucinated calls that fail, and two guards enforce it
/// (`ops_tests::typed_mode_filters_tools_by_skill_filter`).
///
/// Agents skipped here are still covered: `offload_oversized_result` runs on
/// every sub-agent outcome and needs no cooperation from the model.
///
/// The contract text is rendered from the artifact module so the directory
/// names the model is told about can never drift from the ones
/// `resolve_artifact_path` will actually accept. The heading check makes it
/// idempotent, and lets an archetype prompt spell the convention out itself.
pub(crate) fn append_artifact_offload_contract(
    base_prompt: String,
    agent_id: &str,
    visible_tool_names: &HashSet<String>,
) -> String {
    if !should_render_offload_contract(visible_tool_names, OFFLOAD_WRITE_TOOL) {
        tracing::debug!(
            agent_id = %agent_id,
            "[artifact] sub-agent holds no file-write tool — skipping offload contract (harness-side offload still applies)"
        );
        return base_prompt;
    }
    if base_prompt.contains(ARTIFACT_OFFLOAD_HEADING) {
        return base_prompt;
    }

    let mut prompt = base_prompt;
    if !prompt.ends_with('\n') {
        prompt.push('\n');
    }
    prompt.push('\n');
    prompt.push_str(&render_artifact_offload_contract(OFFLOAD_WRITE_TOOL));

    tracing::debug!(
        agent_id = %agent_id,
        final_chars = prompt.chars().count(),
        "[artifact] appended offload contract to sub-agent system prompt"
    );

    prompt
}

// ─────────────────────────────────────────────────────────────────────────────
// Tool-spec deduplication
// ─────────────────────────────────────────────────────────────────────────────

/// Deduplicate assembled tool specs by name, keeping the first occurrence.
///
/// The sub-agent's `filtered_specs` is a `Vec` assembled from
/// `parent.all_tool_specs` indices plus dynamic tools, so a delegation tool can
/// shadow a same-named skill/integration tool (common for the wide-set
/// `tools_agent`), leaving two specs with the same name. Strict providers reject
/// such a request with `400 "Tool names must be unique."` The main-agent path
/// dedups via [`session::builder::dedup_visible_tool_specs`]; this separate
/// sub-agent assembly must do the same.
///
/// First occurrence wins so registration-order semantics are preserved (tool
/// dispatch still resolves by name). Dropped duplicates are logged at `debug`
/// (diagnostic instrumentation, per the repo Rust logging guideline).
///
/// Extracted as a free function so the regression suite can exercise the dedup
/// without standing up the full `run_typed_mode` plumbing.
pub(super) fn dedup_tool_specs_by_name(agent_id: &str, specs: Vec<ToolSpec>) -> Vec<ToolSpec> {
    let mut seen: HashSet<String> = HashSet::with_capacity(specs.len());
    let mut deduped: Vec<ToolSpec> = Vec::with_capacity(specs.len());
    let mut dropped: Vec<String> = Vec::new();
    for spec in specs {
        if seen.insert(spec.name.clone()) {
            deduped.push(spec);
        } else {
            dropped.push(spec.name);
        }
    }
    if !dropped.is_empty() {
        tracing::debug!(
            agent_id = %agent_id,
            "[subagent_host] dropped {} duplicate tool spec(s) before sending to provider: {:?}",
            dropped.len(),
            dropped
        );
    }
    deduped
}
