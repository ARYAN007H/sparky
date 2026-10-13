//! Scope gating of the Composio agent tools, as a thin adapter over the
//! connector module.
//!
//! The curated catalogs and the stored per-toolkit preference belong to
//! `tinyconnectors` (`ListCuratedCatalog` / `GetVocabulary`, `GetUserScopes`);
//! this file only asks them and turns the answer into the decisions the
//! tools need: [`resolve_action_scope`] / [`action_mutates_external_state`]
//! (sandbox and approval gates), [`evaluate_tool_visibility`] and
//! [`filter_list_tools_response`] (`composio_list_tools`), and the
//! user-facing scope error.

use crate::config::Config;

use super::super::contract::{
    catalog_for_toolkit, classify_unknown, find_curated, toolkit_from_slug, ToolScope,
    UserScopePref,
};
use super::super::ops::load_user_scope_pref;

/// Decision returned by [`evaluate_tool_visibility`].
pub(super) enum ToolDecision {
    /// Action is curated for this toolkit and user scope allows it.
    Allow,
    /// Action exists in the curated list but the user's scope blocks
    /// it. `scope` is the curated classification.
    BlockedByScope { scope: ToolScope },
    /// Action is not in the toolkit's curated whitelist (and the
    /// toolkit has one). Hidden / rejected.
    NotCurated,
    /// Toolkit has no curated catalog — pass through, but still gate by
    /// the user scope using the [`classify_unknown`] heuristic.
    PassthroughCheckScope { scope: ToolScope },
}

/// Resolve a Composio action slug to its [`ToolScope`] classification.
///
/// Prefers the toolkit's curated catalog when available (most accurate
/// — curated entries are hand-classified) and falls back to the
/// [`classify_unknown`] heuristic for un-curated toolkits. Unparseable
/// slugs default to `Write` so the sandbox gate errs on the side of
/// blocking rather than letting a potentially-mutating action slip
/// through uncategorised.
pub(crate) async fn resolve_action_scope(slug: &str) -> ToolScope {
    resolve_action_scope_sync(slug)
}

/// Synchronous core used by policy hooks that cannot await.
fn resolve_action_scope_sync(slug: &str) -> ToolScope {
    let Some(toolkit) = toolkit_from_slug(slug) else {
        return ToolScope::Write;
    };
    let catalog = catalog_for_toolkit(&toolkit);
    if let Some(cat) = catalog {
        if let Some(entry) = find_curated(&cat, slug) {
            return entry.scope;
        }
    }
    classify_unknown(slug)
}

/// Whether an action must pass through the human approval gate.
pub(crate) fn action_mutates_external_state(slug: &str) -> bool {
    matches!(
        resolve_action_scope_sync(slug),
        ToolScope::Write | ToolScope::Admin
    )
}

/// Decide whether a Composio action slug should be visible / executable
/// for the current user, given the registered provider's curated list
/// (if any) and the user's stored scope preference.
pub(super) async fn evaluate_tool_visibility(config: &Config, slug: &str) -> ToolDecision {
    let Some(toolkit) = toolkit_from_slug(slug) else {
        // Unparseable slug — let the backend return its own error.
        return ToolDecision::Allow;
    };
    let pref = load_user_scope_pref(config, &toolkit).await;
    // The catalog covers every catalogued toolkit directly now — the
    // engine's `get_provider(toolkit).curated_tools()` hop this used to
    // prefer was pure indirection, verified identical to `catalog_for_toolkit`
    // for every toolkit that had a native provider.
    let catalog = catalog_for_toolkit(&toolkit);
    match catalog {
        Some(catalog) => match find_curated(&catalog, slug) {
            Some(curated) if pref.allows(curated.scope) => ToolDecision::Allow,
            Some(curated) => ToolDecision::BlockedByScope {
                scope: curated.scope,
            },
            None => ToolDecision::NotCurated,
        },
        None => {
            let scope = classify_unknown(slug);
            if pref.allows(scope) {
                ToolDecision::PassthroughCheckScope { scope }
            } else {
                ToolDecision::BlockedByScope { scope }
            }
        }
    }
}

/// Filter a freshly-fetched [`super::super::types::ComposioToolsResponse`] in
/// place: drop tools that aren't curated for their toolkit and tools
/// whose scope is disabled in the user's pref.
pub(super) async fn filter_list_tools_response(
    config: &Config,
    resp: &mut super::super::types::ComposioToolsResponse,
) {
    let before = resp.tools.len();
    // Compute keep/drop decisions sequentially (the await means we
    // can't fold this into a single sync `retain` closure). Then zip
    // each tool with its decision and collect the survivors — clearer
    // than juggling a parallel index alongside `Vec::retain`.
    let mut keep: Vec<bool> = Vec::with_capacity(before);
    for t in &resp.tools {
        let decision = evaluate_tool_visibility(config, &t.function.name).await;
        keep.push(matches!(
            decision,
            ToolDecision::Allow | ToolDecision::PassthroughCheckScope { .. }
        ));
    }
    let drained = std::mem::take(&mut resp.tools);
    resp.tools = drained
        .into_iter()
        .zip(keep)
        .filter_map(|(tool, keep_it)| if keep_it { Some(tool) } else { None })
        .collect();
    let after = resp.tools.len();
    if after != before {
        tracing::debug!(
            before,
            after,
            dropped = before - after,
            "[composio][scopes] composio_list_tools filtered"
        );
    }
}

/// Format a user-facing error message for a scope-blocked execution.
///
/// Embeds the unlock path in the error itself so the agent reads the
/// instruction straight off the tool response — same policy-in-data
/// approach as the `gated_tools` surface. Only ONE path: the user
/// toggles the scope in the Connections UI. The agent has no tool to
/// flip scopes (see the note above the removed `ComposioEnableScopeTool`
/// for why) — it can only describe the gate and point at the UI.
pub(super) fn scope_error_message(slug: &str, scope: ToolScope, pref: UserScopePref) -> String {
    let toolkit = toolkit_from_slug(slug).unwrap_or_default();
    let scope_str = scope.as_str();
    format!(
        "composio_execute: action `{slug}` is classified `{scope_str}` and is \
         disabled in the user's current scope preferences for `{toolkit}` \
         (read={}, write={}, admin={}). Tell the user this action requires the \
         `{scope_str}` scope and they can enable it themselves in \
         **Connections → {toolkit} → {scope_str}**. Do not claim you can flip \
         it — you cannot.",
        pref.read, pref.write, pref.admin,
    )
}
