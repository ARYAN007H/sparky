//! OpenHuman's MCP endpoint for TinyAgents' native Claude subprocess builder.

#[derive(Debug)]
pub(super) struct OpenHumanMcpEndpoint;

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
