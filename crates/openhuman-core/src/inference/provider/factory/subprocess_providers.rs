//! Subprocess-backed providers: the Claude Agent SDK and the Claude Code CLI.

use super::*;
use tinyinference_providers::route::{ProviderRoute, RouteKind};

/// Build the Claude Agent SDK subprocess directly as a crate model. This is a
/// prompt-guided model: TinyAgents owns its text-tool protocol, while the
/// provider owns only subprocess transport and NDJSON decoding.
pub(super) fn claude_agent_sdk_model_from_string(
    provider: &str,
    config: &Config,
) -> Option<String> {
    let provider = provider.trim();
    if provider == CLAUDE_AGENT_SDK_PROVIDER {
        return config.claude_agent_sdk.default_model.clone();
    }
    let route = ProviderRoute::parse(provider)?;
    matches!(route.kind(), RouteKind::ClaudeAgentSdk).then_some(route.model)
}

pub(super) fn build_claude_code_model(
    role: &str,
    provider: &str,
    config: &Config,
    model_override: Option<&str>,
    egress: &EgressLedger,
) -> Option<anyhow::Result<(Arc<dyn ChatModel<()>>, String)>> {
    let route = ProviderRoute::parse(provider)?;
    if !matches!(route.kind(), RouteKind::ClaudeCode) {
        return None;
    }
    Some((|| {
        if route.temperature.is_some() {
            log::warn!(
                "[providers][chat-factory] claude-code provider: per-model temperature override \
                 is accepted but not wired through to the CLI — the @<temp> suffix is ignored"
            );
        }
        if route.model.is_empty() {
            anyhow::bail!(
                "[chat-factory] provider string '{}' for role '{}' has an empty model — \
                 use 'claude-code:<model-id>'",
                provider,
                role
            );
        }
        let effective_model = model_override.unwrap_or(&route.model).to_owned();
        emit_inference_egress(
            role,
            &format!(
                "{}{effective_model}",
                tinyagents_harness::providers::claude_code::PROVIDER_PREFIX
            ),
            egress,
        );
        let workspace = config
            .config_path
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| config.workspace_dir.clone());
        let model = tinyagents_harness::providers::claude_code::ClaudeCodeProvider::from_env(
            effective_model,
            workspace,
            config.action_dir.clone(),
        )?
        .with_mcp_provider(Arc::new(OpenHumanMcpEndpoint));
        Ok((Arc::new(model) as Arc<dyn ChatModel<()>>, route.model))
    })())
}

#[derive(Debug)]
struct OpenHumanMcpEndpoint;

#[async_trait::async_trait]
impl tinyagents_harness::providers::claude_code::driver::McpEndpointProvider
    for OpenHumanMcpEndpoint
{
    async fn endpoint(
        &self,
    ) -> Result<tinyagents_harness::providers::claude_code::driver::McpEndpoint, String> {
        let endpoint = crate::mcp::server::ensure_local_http()
            .await
            .map_err(|error| error.to_string())?;
        Ok(
            tinyagents_harness::providers::claude_code::driver::McpEndpoint {
                addr: endpoint.addr,
                token: endpoint.token,
            },
        )
    }
}
