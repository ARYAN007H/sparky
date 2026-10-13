//! Tool listing ops.

use crate::config::Config;
use crate::core::Outcome;

use super::super::module_client::{self as connectors};
use super::super::types::{ComposioListToolsRequest, ComposioToolsResponse};
use super::error_utils::{report_composio_op_error, should_forward_tags, OpResult};
use tinyconnectors_bus::names::methods;

/// List the actions the agent may pick from.
///
/// # What moved
///
/// The direct route used to be a second implementation here: prefetch the
/// connected toolkits, then fetch v3 schemas for them. That is the module's
/// now, and this function no longer branches on which route is live.
///
/// # Why the scope filter is asked for `false`
///
/// The module can filter a listing by the user's scope preference, and
/// eventually it should — it is the half that also refuses an `Execute`, and
/// one component deciding both is the only arrangement that cannot disagree
/// with itself.
///
/// The preference now lives in the module (`GetUserScopes`), so the two
/// would agree; the flag stays `false` because the host's scope gate still
/// applies the preference to what the agent sees (and refuses the execution),
/// and filtering twice would only hide the host's "blocked by scope" message.
pub async fn composio_list_tools(
    config: &Config,
    toolkits: Option<Vec<String>>,
    tags: Option<Vec<String>>,
) -> OpResult<Outcome<ComposioToolsResponse>> {
    let effective_tags = if should_forward_tags(toolkits.as_deref()) {
        tags
    } else {
        None
    };
    tracing::debug!(?toolkits, ?effective_tags, "[composio] rpc list_tools");

    let request = ComposioListToolsRequest {
        toolkits: toolkits.unwrap_or_default(),
        tags: effective_tags.unwrap_or_default(),
        apply_user_scopes: false,
    };
    let named = request.toolkits.len();

    match connectors::call::<_, ComposioToolsResponse>(config, methods::LIST_TOOLS, request).await {
        Ok(resp) => {
            let count = resp.tools.len();
            let line = if named == 0 {
                format!("composio: {count} tool(s) listed")
            } else {
                format!("composio: {count} tool(s) listed across {named} toolkit(s)")
            };
            Ok(Outcome::new(resp, vec![line]))
        }
        Err(error) => {
            report_composio_op_error("list_tools", &anyhow::anyhow!("{error}"));
            Err(format!("[composio] list_tools failed: {error}"))
        }
    }
}
