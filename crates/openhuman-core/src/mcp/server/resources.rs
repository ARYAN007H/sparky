//! MCP resource catalog for bundled and module-owned prompt assets.
//!
//! Static prompt assets are embedded at compile time via `include_str!`.
//! The two TinyFlows archetypes are read through Copilot's `GetPrompt` bus
//! method using the current dispatch config.
//!
//! ## URI scheme
//!
//! | Resource            | URI                                      |
//! |---------------------|------------------------------------------|
//! | `IDENTITY.md`       | `openhuman://prompts/identity`           |
//! | `SOUL.md`           | `openhuman://prompts/soul`               |
//! | `USER.md`           | `openhuman://prompts/user`               |
//! | `<id>/prompt.md`    | `openhuman://prompts/agents/<id>`        |
//!
//! ## Catalog parity
//!
//! The unit test `catalog_mirrors_builtins` cross-references the exposed
//! resources against `BUILTINS` in `loader.rs`.

use serde_json::{json, Value};
use tinymcp_bus::{ResourceSpec, ToolCallError};

/// Every bundled prompt is markdown.
const MIME_TYPE: &str = "text/markdown";

struct PromptResource {
    uri: &'static str,
    name: &'static str,
    description: &'static str,
    content: &'static str,
}

const RESOURCE_CATALOG: &[PromptResource] = &[
    // ── Core prompts ──────────────────────────────────────────────────────
    PromptResource {
        uri: "openhuman://prompts/identity",
        name: "Agent Identity",
        description: "Core agent identity definition (IDENTITY.md).",
        content: include_str!("../../agent/prompts/IDENTITY.md"),
    },
    PromptResource {
        uri: "openhuman://prompts/soul",
        name: "Agent Soul",
        description: "Core agent personality and values (SOUL.md).",
        content: include_str!("../../agent/prompts/SOUL.md"),
    },
    PromptResource {
        uri: "openhuman://prompts/user",
        name: "User Context",
        description: "Core user-profile context injected into every session (USER.md).",
        content: include_str!("../../agent/prompts/USER.md"),
    },
    // ── Subagent prompt templates ─────────────────────────────────────────
    PromptResource {
        uri: "openhuman://prompts/agents/orchestrator",
        name: "orchestrator",
        description: "Chat-tier orchestrator that routes tasks to specialist subagents.",
        content: include_str!("../../agent/registry/agents/orchestrator/prompt.md"),
    },
    PromptResource {
        uri: "openhuman://prompts/agents/planner",
        name: "planner",
        description: "Read-only reasoning worker that decomposes and researches a question for workflow runs.",
        content: include_str!("../../agent/registry/agents/planner/prompt.md"),
    },
    PromptResource {
        uri: "openhuman://prompts/agents/critic",
        name: "critic",
        description: "Read-only worker that cross-checks claims and reviews changes for workflow runs.",
        content: include_str!("../../agent/registry/agents/critic/prompt.md"),
    },
    PromptResource {
        uri: "openhuman://prompts/agents/vision_agent",
        name: "vision_agent",
        description: "Multimodal worker that analyses attached images for the vision tier.",
        content: include_str!("../../agent/registry/agents/vision_agent/prompt.md"),
    },
    PromptResource {
        uri: "openhuman://prompts/agents/image_agent",
        name: "image_agent",
        description: "Worker that generates or edits images via GMI and saves them to the configured Files folder.",
        content: include_str!("../../agent/registry/agents/image_agent/prompt.md"),
    },
    PromptResource {
        uri: "openhuman://prompts/agents/video_agent",
        name: "video_agent",
        description: "Worker that generates short videos via GMI and saves them to the configured Files folder.",
        content: include_str!("../../agent/registry/agents/video_agent/prompt.md"),
    },
    PromptResource {
        uri: "openhuman://prompts/agents/trigger_triage",
        name: "trigger_triage",
        description: "Read-only worker that classifies incoming automation triggers.",
        content: include_str!("../../agent/registry/agents/trigger_triage/prompt.md"),
    },
    PromptResource {
        uri: "openhuman://prompts/agents/trigger_reactor",
        name: "trigger_reactor",
        description: "Worker that executes actions in response to classified triggers.",
        content: include_str!("../../agent/registry/agents/trigger_reactor/prompt.md"),
    },
    PromptResource {
        uri: "openhuman://prompts/agents/morning_briefing",
        name: "morning_briefing",
        description: "Read-only worker that assembles a personalised morning briefing.",
        content: include_str!("../../agent/registry/agents/morning_briefing/prompt.md"),
    },
    PromptResource {
        uri: "openhuman://prompts/agents/summarizer",
        name: "summarizer",
        description: "The extraction contract oversized tool results are summarized against.",
        content: crate::agent::registry::agents::summarizer::prompt::ARCHETYPE,
    },
    PromptResource {
        uri: "openhuman://prompts/agents/presentation_agent",
        name: "presentation_agent",
        description: "Specialist worker for evidence-grounded presentation generation.",
        content: include_str!("../../agent/registry/agents/presentation_agent/prompt.md"),
    },
    PromptResource {
        uri: "openhuman://prompts/agents/task_manager_agent",
        name: "task_manager_agent",
        description: "Specialist worker for task-source feeds, workflow bundles, and artifacts.",
        content: include_str!("../../agent/registry/agents/task_manager_agent/prompt.md"),
    },
    #[cfg(feature = "skills")]
    PromptResource {
        uri: "openhuman://prompts/agents/skill_setup",
        name: "skill_setup",
        description: "Worker that guides skill installation and backend configuration.",
        content: include_str!("../../skills/catalog/agent/skill_setup/prompt.md"),
    },
];

/// The catalog as `resources/list` advertises it.
pub fn resource_specs() -> Vec<ResourceSpec> {
    let mut resources = RESOURCE_CATALOG
        .iter()
        .map(|r| {
            ResourceSpec::new(r.uri, r.name)
                .with_description(r.description)
                .with_mime_type(MIME_TYPE)
        })
        .collect::<Vec<_>>();
    #[cfg(feature = "flows")]
    resources.extend([
        ResourceSpec::new(
            "openhuman://prompts/agents/flow_discovery",
            "flow_discovery",
        )
        .with_description(
            "Flow Scout — read-only workflow discovery agent that suggests automations from memory, threads, and integrations.",
        )
        .with_mime_type(MIME_TYPE),
        ResourceSpec::new(
            "openhuman://prompts/agents/workflow_builder",
            "workflow_builder",
        )
        .with_description(
            "Workflow authoring specialist that builds tinyflows automation graphs and returns proposals for review.",
        )
        .with_mime_type(MIME_TYPE),
    ]);
    log::debug!("[mcp_server] resources/list count={}", resources.len());
    resources
}

/// The `resources/read` result for `uri`, or
/// [`ToolCallError::ResourceNotFound`] (`-32002`) when the catalog has no such
/// entry. TinyMCP has already rejected a missing or blank `uri`.
pub fn read_resource(uri: &str) -> Result<Value, ToolCallError> {
    let resource = RESOURCE_CATALOG
        .iter()
        .find(|r| r.uri == uri)
        .ok_or_else(|| {
            log::debug!("[mcp_server] resources/read unknown uri={uri}");
            ToolCallError::ResourceNotFound(format!("no resource with uri `{uri}`"))
        })?;

    log::debug!(
        "[mcp_server] resources/read uri={uri} bytes={}",
        resource.content.len()
    );

    Ok(json!({
        "contents": [{
            "uri": resource.uri,
            "mimeType": MIME_TYPE,
            "text": resource.content
        }]
    }))
}

#[cfg(test)]
#[path = "resources_tests.rs"]
mod tests;
