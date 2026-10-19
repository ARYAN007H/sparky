use super::*;

// ─────────────────────────────────────────────────────────────────────────────
// Tool-contract enforcement gate (systemic tool-contract fix, Part 2)
// ─────────────────────────────────────────────────────────────────────────────
//
// `validate_binding_resolvability` (above) statically proves a binding's
// SHAPE is sound (envelope dereference, agent output schema). It has no
// opinion on whether a `tool_call` node's `slug` is a REAL Composio action,
// or whether the args it wires cover that action's REAL required set — a
// builder could pass a hallucinated slug (`SLACK_POST_MESSAGE_TO_CHANNEL`,
// which 404s at runtime) or omit a genuinely required arg, and
// `validate_binding_resolvability` would have nothing to say about either.
// [`validate_tool_contracts`] is that missing HARD gate, grounded in
// [`crate::integrations::composio::catalog::fetch_live_toolkit_catalog`] — the
// FULL LIVE Composio catalog, not the static curated subset.

/// Statically proves every `tool_call` node's `config.slug` is a REAL action
/// in the LIVE Composio catalog for its toolkit, and that every one of that
/// action's REAL required args is present (non-null) in `config.args` —
/// rejecting the graph (a non-empty `Vec` = reject; empty = pass) when
/// either check fails. Wired into `propose_workflow` / `revise_workflow` /
/// `save_workflow` alongside [`validate_binding_resolvability`].
///
/// Skipped for a `slug` that is `=`-derived (resolved from upstream/trigger
/// data at runtime — nothing to check statically) or a native `oh:` tool (no
/// Composio contract at all).
///
/// **Best-effort on catalog availability, not on catalog CONTENT**: when the
/// live-catalog fetch itself fails (no backend session, network error) the
/// node is SKIPPED with a debug log — never rejected — because a
/// hallucinated slug can only be confirmed hallucinated once the real
/// catalog was actually reachable; `graph_wiring_warnings`'s
/// `composio_required_args` checks share this exact contract. Once the
/// catalog IS reachable, though, both checks below are HARD: an unreal slug
/// or a missing required arg rejects the graph outright, unlike the
/// advisory output-field/`split_out.path` WARNs in `graph_wiring_warnings`
/// (Part 2c/2d) — those degrade gracefully because a binding to an unknown
/// field can't be proven wrong, whereas a nonexistent slug or a missing
/// required arg are both provably broken.
/// Whether OpenHuman ships a STATIC curated catalog for `toolkit`. This is the
/// exact condition both [`validate_tool_contracts`]'s curation gate and
/// `tinyflows::caps::flow_tool_allowed`'s runtime Path A use to decide a toolkit
/// is a hard curated-only allowlist: for such a toolkit a real-but-uncurated
/// action is rejected on EVERY real run, so the author-time gate and the early
/// builder-tool warnings (`get_tool_contract` / `search_tool_catalog`) must all
/// agree on it — one home for the check so they cannot drift.
pub(crate) fn toolkit_has_curated_catalog(toolkit: &str) -> bool {
    // The curated catalogs moved to `tinymemory-bus` (OpenHuman#5560) and are
    // reachable directly via `catalog_for_toolkit`, so this no longer needs
    // the engine-backed provider-registry hop the comment here used to
    // explain: `get_provider(toolkit).curated_tools()` was verified identical
    // to `catalog_for_toolkit(toolkit)` for every toolkit that had one, and
    // tinymemory v1.13.4 deleted the registry outright, so the hop is gone
    // rather than merely unnecessary.
    crate::integrations::composio::catalog::has_curated_catalog(toolkit)
}

pub(crate) async fn validate_tool_contracts(config: &Config, graph: &WorkflowGraph) -> Vec<String> {
    use crate::integrations::composio::contract::validation::{
        ToolContractNode, validate_tool_contract_nodes,
    };

    let nodes: Vec<_> = graph
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::ToolCall)
        .filter_map(|node| {
            Some(ToolContractNode {
                id: node.id.clone(),
                slug: node.config.get("slug")?.as_str()?.to_string(),
                args: node.config.get("args").cloned().unwrap_or(Value::Null),
            })
        })
        .collect();
    validate_tool_contract_nodes(config, &nodes).await
}
