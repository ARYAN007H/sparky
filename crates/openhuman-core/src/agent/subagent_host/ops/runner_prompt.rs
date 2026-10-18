//! Scoped async preparation for typed sub-agent prompt rendering.

use super::{AgentDefinition, PromptContext, SubagentRunError};
use crate::agent::subagent_host::tool_prep::load_prompt_source;

/// One spawn's snapshot of the host config, loaded once by [`super::run_subagent`].
///
/// Threading it in as a parameter keeps all prompt, model and context decisions
/// on the same profile snapshot without giving the generic runtime file access.
pub(super) type LoadedConfig = Result<std::sync::Arc<crate::config::Config>, String>;

/// Prefetch a module-owned archetype before entering the large typed-runner future.
pub(super) async fn load_flow_prompt(
    definition: &AgentDefinition,
    config: &LoadedConfig,
) -> Result<Option<String>, SubagentRunError> {
    #[cfg(feature = "flows")]
    {
        return Ok(crate::agent::prompts::flow_archetype::load_for_definition(
            config.as_ref().ok().map(|config| config.as_ref()),
            definition,
        )
        .await?);
    }
    #[cfg(not(feature = "flows"))]
    {
        let _ = (definition, config);
        Ok(None)
    }
}

/// Synchronously render a typed prompt with the already scoped module text.
pub(super) fn load_typed_prompt_source(
    definition: &AgentDefinition,
    mut prompt_context: PromptContext<'_>,
    flow_archetype_text: Option<&str>,
) -> Result<String, SubagentRunError> {
    prompt_context.flow_archetype_text = flow_archetype_text;
    load_prompt_source(definition, &prompt_context)
}
