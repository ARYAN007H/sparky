//! Map effective OpenHuman runtime reasoning settings to TinyInference config.
//!
//! Per-model settings take precedence over the global setting, which takes
//! precedence over `reasoning_enabled = false`. A thread's explicit choice is
//! applied by the TinyAgents host adapter before this mapping is consulted.

use tinyinference_llm::model::{ReasoningConfig, ReasoningEffort};

use crate::config::Config;

/// The config-level reasoning choice for the selected model, or `None` to
/// leave reasoning to the provider.
pub(crate) fn reasoning_for_config(config: &Config) -> Option<ReasoningConfig> {
    let per_model = config
        .default_model
        .as_deref()
        .and_then(|model| config.runtime.reasoning_effort_by_model.get(model.trim()));
    if let Some(raw) = per_model
        .or(config.runtime.reasoning_effort.as_ref())
        .map(|raw| raw.trim())
        .filter(|raw| !raw.is_empty())
    {
        return match ReasoningEffort::parse(raw) {
            Some(effort) => Some(ReasoningConfig::effort(effort)),
            None => {
                log::warn!(
                    "[agent][reasoning] ignoring unknown runtime.reasoning_effort {raw:?}; \
                     using the provider default"
                );
                None
            }
        };
    }
    if config.runtime.reasoning_enabled == Some(false) {
        return Some(ReasoningConfig::effort(ReasoningEffort::None));
    }
    None
}

#[cfg(test)]
#[path = "reasoning_tests.rs"]
mod tests;
