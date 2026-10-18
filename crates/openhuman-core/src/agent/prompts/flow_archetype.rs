//! Scoped TinyFlows archetype prompt loading for the two flow builtins.

use anyhow::{anyhow, ensure, Result};
use tinyagents_definition::{AgentDefinition as GenericAgentDefinition, DefinitionSource, PromptSource};

use crate::{
    config::Config,
    agent::harness::host_definition::AgentDefinition,
};

/// Whether `definition` is one of the bus-backed builtin flow prompt owners.
///
/// User definitions and static/session prompt overrides keep their own source
/// and never cause a TinyFlows module lookup, even when they reuse a builtin id.
pub(crate) fn is_bus_backed_builtin(definition: &AgentDefinition) -> bool {
    matches!(&definition.source, DefinitionSource::Builtin)
        && matches!(&definition.system_prompt, PromptSource::Dynamic(()))
        && matches!(definition.id.as_str(), "workflow_builder" | "flow_discovery")
}

/// Fetch the archetype for a resolved definition through the module bus.
///
/// The caller supplies the exact config snapshot that selected the module
/// generation. No ambient config or fallback prompt is used.
pub(crate) async fn load_for_definition(
    config: Option<&Config>,
    definition: &AgentDefinition,
) -> Result<Option<String>> {
    if !is_bus_backed_builtin(definition) {
        return Ok(None);
    }

    let config = config.ok_or_else(|| {
        anyhow!(
            "cannot render TinyFlows builtin `{}` without its scoped runtime config",
            definition.id
        )
    })?;
    let proxy = crate::modules::tinyflows::copilot_proxy(config)
        .await
        .map_err(anyhow::Error::msg)
        .map_err(|error| error.context(format!("loading TinyFlows prompt `{}`", definition.id)))?;
    let response: tinyflows_bus::copilot::TextResponse = proxy
        .call("GetPrompt", (definition.id.clone(),))
        .await
        .map_err(|error| anyhow!(error.wire_message()))
        .map_err(|error| error.context(format!("fetching TinyFlows prompt `{}`", definition.id)))?;
    ensure!(
        !response.text.trim().is_empty(),
        "TinyFlows returned an empty prompt for builtin `{}`",
        definition.id
    );
    Ok(Some(response.text))
}

#[cfg(test)]
#[path = "flow_archetype_tests.rs"]
mod tests;
