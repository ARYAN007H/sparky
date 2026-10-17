use super::*;
use crate::flows::tinyflows::caps::{fetch_live_toolkit_catalog, ToolContract};

pub(crate) const RUNTIME_GATE_WARNING: &str = "This action will be REJECTED on every real run — the runtime tool gate only allows curated actions for this toolkit. Pick a `featured: true` result from search_tool_catalog instead.";

pub(crate) enum ContractLookupError {
    InvalidSlug { slug: String },
    CatalogUnavailable { toolkit: String },
    ActionNotFound { slug: String, toolkit: String },
}

/// Resolves one live action and the runtime-curation warning used by both
/// builder surfaces. The runtime allowlist remains authoritative; this only
/// classifies an action so authoring can explain an inevitable rejection.
pub(crate) async fn lookup_contract(
    config: &Config,
    slug: &str,
) -> Result<(ToolContract, Option<&'static str>), ContractLookupError> {
    let trimmed = slug.trim();
    let Some(toolkit) = toolkit_for_contract_slug(trimmed) else {
        return Err(ContractLookupError::InvalidSlug {
            slug: slug.to_string(),
        });
    };
    let Some(catalog) = fetch_live_toolkit_catalog(config, &toolkit).await else {
        return Err(ContractLookupError::CatalogUnavailable { toolkit });
    };
    let Some(contract) = catalog.iter().find(|contract| {
        crate::integrations::composio::contract::action_slug_matches(&contract.slug, trimmed)
    }) else {
        return Err(ContractLookupError::ActionNotFound {
            slug: trimmed.to_string(),
            toolkit,
        });
    };
    let warning = (!contract.is_curated && toolkit_has_curated_catalog(&toolkit))
        .then_some(RUNTIME_GATE_WARNING);
    Ok((contract.clone(), warning))
}

// ─────────────────────────────────────────────────────────────────────────────
// Catalog RPCs for the UI (Phase 5, item 16) — one implementation, two consumers
// ─────────────────────────────────────────────────────────────────────────────

/// Searches the live Composio tool catalog (secret-free) — the RPC the in-canvas
/// tool browser calls, reusing the exact same core as the agent's
/// `search_tool_catalog` tool so the two can't drift.
pub async fn flows_search_tool_catalog(
    config: &Config,
    query: &str,
    toolkit: Option<&str>,
    limit: usize,
) -> Result<Outcome<Value>, String> {
    tracing::debug!(target: "flows", %query, toolkit = toolkit.unwrap_or("<all>"), "[flows] flows_search_tool_catalog: searching live catalog");
    let tools =
        crate::flows::builder_tools::search_live_catalog(config, query, toolkit, limit).await;
    Ok(Outcome::single_log(
        json!({ "tools": tools }),
        "tool catalog searched",
    ))
}

/// Resolves the toolkit for a *single action* slug, rejecting anything that is
/// not shaped `<TOOLKIT>_<ACTION>`.
///
/// [`crate::integrations::composio::contract::toolkit_from_slug`] falls back to the whole
/// string when there is no `_`, so it answers `Some` for every non-empty input
/// — `toolkit_from_slug("nodashhere") == Some("nodashhere")`. That permissive
/// fall-back is load-bearing for `compute_required_connections`, which maps
/// toolkit-ish tokens and legitimately wants the whole string back, so the
/// stricter rule belongs to this caller rather than to the shared helper.
///
/// A contract fetch returns *one action*, so a slug with no action segment
/// cannot name anything it could return; rejecting it here also spares the
/// caller a pointless catalog round trip (which takes a per-toolkit fetch lock)
/// before failing with an unrelated "could not fetch the catalog" message.
///
/// The multi-segment toolkit prefixes (`MICROSOFT_TEAMS_`, `ONE_DRIVE_`,
/// `ZOHO_MAIL_`) all end in `_`, so a real action under one of them always has
/// non-empty segments either side of its first `_` and passes unchanged.
pub(crate) fn toolkit_for_contract_slug(slug: &str) -> Option<String> {
    let trimmed = slug.trim();
    let (toolkit_segment, action_segment) = trimmed.split_once('_')?;
    if toolkit_segment.is_empty() || action_segment.is_empty() {
        return None;
    }
    crate::integrations::composio::contract::toolkit_from_slug(trimmed)
}

/// Fetches one Composio action's full contract (secret-free) — the RPC the
/// canvas tool browser calls to fill in an action's arg schema, reusing the same
/// core as the agent's `get_tool_contract` tool.
pub async fn flows_get_tool_contract(
    config: &Config,
    slug: &str,
) -> Result<Outcome<Value>, String> {
    let trimmed = slug.trim();
    let (contract, runtime_gate) =
        lookup_contract(config, slug)
            .await
            .map_err(|error| match error {
                ContractLookupError::InvalidSlug { slug } => format!(
                    "Could not extract a toolkit from slug '{slug}' — it must look like \
             '<TOOLKIT>_<ACTION>' (e.g. 'GMAIL_SEND_EMAIL')."
                ),
                ContractLookupError::CatalogUnavailable { toolkit } => {
                    format!("Could not fetch the live Composio catalog for toolkit '{toolkit}'.")
                }
                ContractLookupError::ActionNotFound { slug, toolkit } => format!(
                    "'{slug}' is not a real action in the '{toolkit}' toolkit's live catalog."
                ),
            })?;
    let value = serde_json::to_value(&contract).map_err(|e| e.to_string())?;
    tracing::debug!(target: "flows", slug = %trimmed, toolkit = %contract.toolkit, "[flows] flows_get_tool_contract: fetching contract");
    let mut response = json!({ "contract": value });
    if let Some(warning) = runtime_gate {
        response["runtime_gate"] = json!(warning);
    }
    Ok(Outcome::single_log(response, "tool contract fetched"))
}
