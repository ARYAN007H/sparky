use super::*;
use crate::config::Config;
use crate::integrations::composio::catalog::{seed_live_catalog_cache, ToolContract};
use serde_json::{json, Value};
use tinyflows_catalog::FlowConnection;
use tinyflows::model::WorkflowGraph;
use tempfile::TempDir;

fn test_config(tmp: &TempDir) -> Config {
    let mut config = Config {
        workspace_dir: tmp.path().join("workspace"),
        action_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("config.toml"),
        ..Config::default()
    };
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    config
}


fn graph(value: Value) -> WorkflowGraph {
    let graph = tinyflows::migrate::deserialize_graph(value).expect("test graph deserializes");
    tinyflows::validate::validate(&graph).expect("test graph is structurally valid");
    tinyflows::compat::ensure_compatible(&graph).expect("test graph is compatible");
    graph
}

fn seeded_slack_send_contract() -> ToolContract {
    ToolContract {
        slug: "SLACK_SEND_MESSAGE".to_string(),
        toolkit: "slack".to_string(),
        description: None,
        required_args: vec!["channel".to_string(), "text".to_string()],
        input_schema: None,
        output_fields: vec!["ts".to_string(), "channel".to_string()],
        output_schema: Some(json!({
            "type": "object",
            "properties": { "ts": {"type": "string"}, "channel": {"type": "string"} }
        })),
        primary_array_path: None,
        is_curated: true,
    }
}

fn seeded_slack_send_message_contract_with_schema() -> ToolContract {
    ToolContract {
        slug: "SLACKARGNAMETEST_SEND_MESSAGE".to_string(),
        toolkit: "slackargnametest".to_string(),
        description: None,
        required_args: vec![],
        input_schema: Some(json!({
            "type": "object",
            "properties": {
                "channel": { "type": "string" },
                "markdown_text": { "type": "string" }
            }
        })),
        output_fields: vec![],
        output_schema: None,
        primary_array_path: None,
        is_curated: false,
    }
}

fn ws3_flow_conn(toolkit: &str, id: &str) -> FlowConnection {
    FlowConnection {
        connection_ref: format!("composio:{toolkit}:{id}"),
        kind: "composio".to_string(),
        display: toolkit.to_string(),
        toolkit: Some(toolkit.to_string()),
        scheme: None,
        platform_user_id: None,
    }
}

fn ws3_transcript_connections() -> Vec<FlowConnection> {
    vec![
        ws3_flow_conn("twitter", "ca_JX6QU88UfSk4"),
        ws3_flow_conn("gmail", "ca_vX_WA8FsqNmE"),
        ws3_flow_conn("tiktok", "ca_LPCp3WQpaDma"),
    ]
}

fn ws3_tool_call_graph(slug: &str, connection_ref: Option<&str>) -> WorkflowGraph {
    let mut config = json!({ "slug": slug, "args": {} });
    if let Some(connection_ref) = connection_ref {
        config["connection_ref"] = json!(connection_ref);
    }
    graph(json!({
        "nodes": [
            {"id":"t","kind":"trigger","name":"Manual"},
            {"id":"act","kind":"tool_call","name":"Act","config":config}
        ],
        "edges": [{"from_node":"t","to_node":"act"}]
    }))
}

fn upload_graph(path: Value) -> WorkflowGraph {
    graph(json!({
        "nodes": [
            {"id":"t","kind":"trigger","name":"Manual"},
            {"id":"up","kind":"tool_call","name":"Upload","config":{"slug":"oh:storage_upload_file","args":{"path":path}}}
        ],
        "edges": [{"from_node":"t","to_node":"up"}]
    }))
}

#[tokio::test]
async fn validate_tool_contracts_rejects_a_hallucinated_slug() {
    seed_live_catalog_cache("slack", vec![seeded_slack_send_contract()]);
    let config = Config::default();
    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Manual" },
            { "id": "post", "kind": "tool_call", "name": "Post",
              "config": { "slug": "SLACK_POST_MESSAGE_TO_CHANNEL",
                "args": { "channel": "#general", "markdown_text": "hi" } } }
        ],
        "edges": [ { "from_node": "t", "to_node": "post" } ]
    }));
    let errors = validate_tool_contracts(&config, &g).await;
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("post"), "{}", errors[0]);
    assert!(
        errors[0].contains("SLACK_POST_MESSAGE_TO_CHANNEL"),
        "{}",
        errors[0]
    );
    assert!(errors[0].contains("search_tool_catalog"), "{}", errors[0]);
}

#[tokio::test]
async fn validate_tool_contracts_rejects_a_missing_required_arg() {
    seed_live_catalog_cache("slack", vec![seeded_slack_send_contract()]);
    let config = Config::default();
    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Manual" },
            { "id": "post", "kind": "tool_call", "name": "Post",
              "config": { "slug": "SLACK_SEND_MESSAGE",
                "args": { "channel": "#general" } } }
        ],
        "edges": [ { "from_node": "t", "to_node": "post" } ]
    }));
    let errors = validate_tool_contracts(&config, &g).await;
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("`text`"), "{}", errors[0]);
    assert!(errors[0].contains("get_tool_contract"), "{}", errors[0]);
}

#[tokio::test]
async fn validate_tool_contracts_passes_a_fully_wired_real_slug() {
    seed_live_catalog_cache("slack", vec![seeded_slack_send_contract()]);
    let config = Config::default();
    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Manual" },
            { "id": "post", "kind": "tool_call", "name": "Post",
              "config": { "slug": "SLACK_SEND_MESSAGE",
                "args": { "channel": "#general", "text": "hi" } } }
        ],
        "edges": [ { "from_node": "t", "to_node": "post" } ]
    }));
    let errors = validate_tool_contracts(&config, &g).await;
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn connection_refs_reject_the_transcript_wrong_id_naming_the_right_ref() {
    // Twitter node carrying the TIKTOK connection id: toolkit segment matches
    // (twitter == twitter) but the id belongs to no Twitter account.
    let g = ws3_tool_call_graph(
        "TWITTER_CREATION_OF_A_POST",
        Some("composio:twitter:ca_LPCp3WQpaDma"),
    );
    let conns = ws3_transcript_connections();
    let errors = validate_connection_refs_against(&g, Some(&conns));
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("act"), "{}", errors[0]);
    assert!(
        errors[0].contains("composio:twitter:ca_JX6QU88UfSk4"),
        "must name the correct ref verbatim: {}",
        errors[0]
    );
    assert!(errors[0].contains("did you mean"), "{}", errors[0]);
}

#[test]
fn connection_refs_reject_a_toolkit_mismatch_naming_the_right_ref() {
    // A literal `composio:tiktok:...` ref stamped onto a Twitter node.
    let g = ws3_tool_call_graph(
        "TWITTER_CREATION_OF_A_POST",
        Some("composio:tiktok:ca_LPCp3WQpaDma"),
    );
    let conns = ws3_transcript_connections();
    let errors = validate_connection_refs_against(&g, Some(&conns));
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("tiktok"), "{}", errors[0]);
    assert!(
        errors[0].contains("composio:twitter:ca_JX6QU88UfSk4"),
        "{}",
        errors[0]
    );
}

#[test]
fn connection_refs_reject_an_unknown_id_when_the_toolkit_has_no_connection() {
    // Gmail slug, but no gmail account connected at all → point at composio_connect.
    let g = ws3_tool_call_graph("GMAIL_SEND_EMAIL", Some("composio:gmail:ca_missing"));
    let conns = vec![ws3_flow_conn("twitter", "ca_JX6QU88UfSk4")];
    let errors = validate_connection_refs_against(&g, Some(&conns));
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("composio_connect"), "{}", errors[0]);
    assert!(!errors[0].contains("did you mean"), "{}", errors[0]);
}

#[test]
fn connection_refs_pass_the_correct_ref() {
    let g = ws3_tool_call_graph(
        "TWITTER_CREATION_OF_A_POST",
        Some("composio:twitter:ca_JX6QU88UfSk4"),
    );
    let conns = ws3_transcript_connections();
    let errors = validate_connection_refs_against(&g, Some(&conns));
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn connection_refs_reject_a_malformed_ref() {
    let g = ws3_tool_call_graph("GMAIL_SEND_EMAIL", Some("gmail-ca_vX_WA8FsqNmE"));
    let conns = ws3_transcript_connections();
    let errors = validate_connection_refs_against(&g, Some(&conns));
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("malformed"), "{}", errors[0]);
}

#[test]
fn connection_refs_skip_oh_and_refless_and_expression_nodes() {
    // Native oh: tool with a ref → skipped.
    let g_oh = ws3_tool_call_graph("oh:memory_search", Some("composio:twitter:whatever"));
    assert!(
        validate_connection_refs_against(&g_oh, Some(&ws3_transcript_connections())).is_empty()
    );
    // Composio tool_call with NO connection_ref stays allowed (prompts at run).
    let g_refless = ws3_tool_call_graph("TWITTER_CREATION_OF_A_POST", None);
    assert!(
        validate_connection_refs_against(&g_refless, Some(&ws3_transcript_connections()))
            .is_empty()
    );
    // `=`-derived slug → skipped.
    let g_expr = ws3_tool_call_graph("=item.slug", Some("composio:twitter:ca_LPCp3WQpaDma"));
    assert!(
        validate_connection_refs_against(&g_expr, Some(&ws3_transcript_connections())).is_empty()
    );
}

#[test]
fn connection_refs_fail_open_on_unavailable_connections_but_keep_mismatch() {
    // Connections unavailable (None): the id-existence check is SKIPPED — a
    // toolkit-matched ref with an unknown id passes rather than false-reject.
    let g_ok = ws3_tool_call_graph(
        "TWITTER_CREATION_OF_A_POST",
        Some("composio:twitter:ca_anything"),
    );
    assert!(
        validate_connection_refs_against(&g_ok, None).is_empty(),
        "unknown id must be skipped when connections are unavailable"
    );
    // ...but the toolkit-mismatch check needs no I/O and still fires.
    let g_mismatch = ws3_tool_call_graph(
        "TWITTER_CREATION_OF_A_POST",
        Some("composio:tiktok:ca_LPCp3WQpaDma"),
    );
    let errors = validate_connection_refs_against(&g_mismatch, None);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("tiktok"), "{}", errors[0]);
}

/// The mock-run argument preflight as `run_builder_gates` invokes it: the
/// analysis is `tinyflows::preflight`'s, this pins the host's native slug prefix.
async fn validate_required_arg_resolvability(graph: &WorkflowGraph) -> Vec<String> {
    tinyflows::preflight::unresolvable_tool_args(
        graph,
        &[crate::flows::tinyflows::caps::NATIVE_TOOL_PREFIX],
    )
    .await
}

// ── validate_required_arg_resolvability (issue B18) ─────────────────────────
//
// `validate_tool_contracts`'s `missing_required_args` only proves an arg is
// PRESENT (absent/literal-null) — it says nothing about whether an arg wired
// to a real-looking `=`-expression actually RESOLVES to a value at runtime,
// nor about an arg the schema doesn't individually mark `required` even
// though the provider enforces it as a business rule (the real B18 bug:
// `GMAIL_SEND_EMAIL.subject`/`.body` are each optional in the schema, but
// Gmail rejects a send where both are empty). These tests sandbox-run the
// graph the same way `dry_run_workflow` does and prove ANY tool_call arg
// that resolves `null` (because it's bound to a field that doesn't exist
// upstream) is a hard reject, while a fully-resolved graph passes clean. No
// live-catalog seeding needed — this check doesn't consult the Composio
// schema at all, only the sandbox's own traced diagnostics.

#[tokio::test]
async fn validate_required_arg_resolvability_rejects_a_null_resolved_arg() {
    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Manual" },
            { "id": "prep", "kind": "code", "name": "Prep",
              "config": { "language": "javascript", "source": "return {};" } },
            { "id": "post", "kind": "tool_call", "name": "Post",
              "config": { "slug": "GMAIL_SEND_EMAIL",
                "args": { "recipient_email": "a@b.com", "subject": "=item.nonexistent_field" } } }
        ],
        "edges": [
            { "from_node": "t", "to_node": "prep" },
            { "from_node": "prep", "to_node": "post" }
        ]
    }));
    let errors = validate_required_arg_resolvability(&g).await;
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("post"), "{}", errors[0]);
    assert!(errors[0].contains("`subject`"), "{}", errors[0]);
    assert!(errors[0].contains("GMAIL_SEND_EMAIL"), "{}", errors[0]);
}

#[tokio::test]
async fn validate_required_arg_resolvability_accepts_a_fully_resolved_graph() {
    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Manual" },
            { "id": "post", "kind": "tool_call", "name": "Post",
              "config": { "slug": "GMAIL_SEND_EMAIL",
                "args": { "recipient_email": "a@b.com", "subject": "hello", "body": "hi there" } } }
        ],
        "edges": [ { "from_node": "t", "to_node": "post" } ]
    }));
    let errors = validate_required_arg_resolvability(&g).await;
    assert!(errors.is_empty(), "{errors:?}");
}

#[tokio::test]
async fn validate_required_arg_resolvability_ignores_native_and_dynamic_slugs() {
    // `oh:` native tools and `=`-derived slugs have no external-provider
    // rejection mode this gate should be checking.
    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Manual" },
            { "id": "prep", "kind": "code", "name": "Prep",
              "config": { "language": "javascript", "source": "return {};" } },
            { "id": "native", "kind": "tool_call", "name": "Native",
              "config": { "slug": "oh:web_search",
                "args": { "query": "=item.nonexistent_field" } } },
            { "id": "dynamic", "kind": "tool_call", "name": "Dynamic",
              "config": { "slug": "=item.nonexistent_field",
                "args": { "x": "=item.nonexistent_field" } } }
        ],
        "edges": [
            { "from_node": "t", "to_node": "prep" },
            { "from_node": "prep", "to_node": "native" },
            { "from_node": "native", "to_node": "dynamic" }
        ]
    }));
    let errors = validate_required_arg_resolvability(&g).await;
    assert!(errors.is_empty(), "{errors:?}");
}

#[tokio::test]
async fn mock_opaque_tool_call_upstream_ref_matches_native_and_composio_upstreams() {
    // Both a Composio curated action and a native `oh:` tool are opaque-echoed
    // by the mock sandbox, so a null bound to EITHER is unverifiable (Some).
    // An `agent` / `code` upstream's real output IS produced by the sandbox, and
    // a `=`-dynamic slug is unknowable, so a null bound to those is genuine (None).
    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Manual" },
            { "id": "code_up", "kind": "code", "name": "Code",
              "config": { "language": "javascript", "source": "return {};" } },
            { "id": "agent_up", "kind": "agent", "name": "Agent",
              "config": { "agent_ref": "planner", "prompt": "x" } },
            { "id": "native_up", "kind": "tool_call", "name": "Link",
              "config": { "slug": "oh:storage_get_link", "args": { "file_id": "f" } } },
            { "id": "composio_up", "kind": "tool_call", "name": "Profile",
              "config": { "slug": "GMAIL_GET_PROFILE", "args": {} } },
            { "id": "dyn_up", "kind": "tool_call", "name": "Dyn",
              "config": { "slug": "=item.slug", "args": {} } },
            { "id": "sink", "kind": "tool_call", "name": "Sink",
              "config": { "slug": "GMAIL_SEND_EMAIL", "args": {} } }
        ],
        "edges": []
    }));
    let up = |expr: &str| {
        tinyflows::preflight::mock_opaque_tool_call_upstream_ref(expr, &g, "sink")
            .map(str::to_string)
    };
    assert_eq!(
        up("=nodes.native_up.item.json.url").as_deref(),
        Some("native_up")
    );
    assert_eq!(
        up("=nodes.composio_up.item.json.data.emailAddress").as_deref(),
        Some("composio_up")
    );
    assert_eq!(up("=nodes.agent_up.item.json.field"), None);
    assert_eq!(up("=nodes.code_up.item.json.field"), None);
    assert_eq!(up("=nodes.dyn_up.item.json.x"), None);
}

#[tokio::test]
async fn validate_required_arg_resolvability_downgrades_null_from_native_tool_call_upstream() {
    // #5148's chain: a Composio `send` binds its `attachment` to a native
    // `oh:storage_get_link` node's `url`. That `url` is null in the echo sandbox
    // (native tools are opaque-echoed), but the wiring is correct, so the gate
    // must NOT reject it. Before the native-upstream carve-out it did — the loop
    // that halted the live "fix with agent" self-repair.
    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Manual" },
            { "id": "prep", "kind": "code", "name": "Prep",
              "config": { "language": "javascript", "source": "return {};" } },
            { "id": "get_link", "kind": "tool_call", "name": "Link",
              "config": { "slug": "oh:storage_get_link", "args": { "file_id": "f_1" } } },
            { "id": "send", "kind": "tool_call", "name": "Send",
              "config": { "slug": "GMAIL_SEND_EMAIL",
                "args": { "recipient_email": "a@b.com", "subject": "hi", "body": "there",
                          "attachment": "=nodes.get_link.item.json.url" } } }
        ],
        "edges": [
            { "from_node": "t", "to_node": "prep" },
            { "from_node": "prep", "to_node": "get_link" },
            { "from_node": "get_link", "to_node": "send" }
        ]
    }));
    let errors = validate_required_arg_resolvability(&g).await;
    assert!(
        errors.is_empty(),
        "a native-upstream attachment null must be downgraded, got: {errors:?}"
    );
}

#[tokio::test]
async fn native_file_attachment_chain_passes_required_arg_resolvability() {
    // Drift check that was missing pre-merge: author #5148's OWN documented
    // `produce -> oh:storage_upload_file -> oh:storage_get_link -> send` chain
    // and assert the null-arg gate (the exact gate that rejected it in the live
    // "fix with agent" loop) now passes it. Targets `validate_required_arg_
    // resolvability` directly (deterministic, no live catalog) rather than
    // `run_builder_gates`, whose connection/contract gates need live Composio.
    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Manual" },
            { "id": "make_page", "kind": "code", "name": "Write",
              "config": { "language": "javascript", "source": "return {};" } },
            { "id": "upload", "kind": "tool_call", "name": "Upload",
              "config": { "slug": "oh:storage_upload_file", "args": { "path": "report.html" } } },
            { "id": "get_link", "kind": "tool_call", "name": "Link",
              "config": { "slug": "oh:storage_get_link",
                "args": { "file_id": "=nodes.upload.item.json.file_id", "expires_in_seconds": 900 } } },
            { "id": "send", "kind": "tool_call", "name": "Send",
              "config": { "slug": "GMAIL_SEND_EMAIL",
                "args": { "recipient_email": "a@b.com", "subject": "AI trends", "body": "attached",
                          "attachment": "=nodes.get_link.item.json.url" } } }
        ],
        "edges": [
            { "from_node": "t", "to_node": "make_page" },
            { "from_node": "make_page", "to_node": "upload" },
            { "from_node": "upload", "to_node": "get_link" },
            { "from_node": "get_link", "to_node": "send" }
        ]
    }));
    let errors = validate_required_arg_resolvability(&g).await;
    assert!(
        errors.is_empty(),
        "the documented native attachment chain must pass the null-arg gate, got: {errors:?}"
    );
}

#[test]
fn validate_upload_paths_rejects_an_absolute_path() {
    // The live-observed bug: the model copies `/tmp/openhuman-flow/report.html`
    // from a prior flow, which the runtime rejects (uploads are confined to the
    // workspace). Catch it at author time with an actionable message.
    let errors = validate_upload_paths(&upload_graph(json!("/tmp/openhuman-flow/report.html")));
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("'up'"), "{}", errors[0]);
    assert!(errors[0].contains("workspace-relative"), "{}", errors[0]);
}

#[test]
fn validate_upload_paths_accepts_a_workspace_relative_path() {
    assert!(validate_upload_paths(&upload_graph(json!("report.html"))).is_empty());
    assert!(validate_upload_paths(&upload_graph(json!("out/report.html"))).is_empty());
}

#[test]
fn validate_upload_paths_rejects_a_parent_escape() {
    let errors = validate_upload_paths(&upload_graph(json!("../../etc/passwd")));
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("escaping with `..`"), "{}", errors[0]);
}

#[test]
fn validate_upload_paths_ignores_a_dynamic_path_expression() {
    // A `=`-expression resolves at runtime; the author-gate can't know its value,
    // so it must not reject it (the runtime check still applies).
    assert!(validate_upload_paths(&upload_graph(json!("=nodes.prep.item.json.path"))).is_empty());
}

/// (Codex feedback on PR #4826) This gate sandbox-runs every graph against
/// `json!({})` as the trigger payload, so a `tool_call` arg wired straight to
/// the trigger's own data — `"to": "=item.email"` on a node whose only
/// predecessor is the trigger — always resolves `null` here, even though a
/// real webhook/app-event/manual trigger fires with a real payload. Hard-
/// rejecting that blocked every ordinary trigger-bound workflow. Contrast
/// with `validate_required_arg_resolvability_rejects_a_null_resolved_arg`
/// above, where the same `=item.<field>` shorthand addresses a real
/// (non-trigger) upstream node and stays a hard reject.
#[tokio::test]
async fn validate_required_arg_resolvability_allows_a_trigger_scoped_null_arg() {
    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Webhook" },
            { "id": "post", "kind": "tool_call", "name": "Post",
              "config": { "slug": "GMAIL_SEND_EMAIL",
                "args": { "recipient_email": "a@b.com", "subject": "hi", "body": "=item.email" } } }
        ],
        "edges": [ { "from_node": "t", "to_node": "post" } ]
    }));
    let errors = validate_required_arg_resolvability(&g).await;
    assert!(errors.is_empty(), "{errors:?}");
}

/// The `nodes.<id>...` explicit-addressing form of the real B18 bug: an arg
/// wired to a specific upstream (non-trigger) node's output path that never
/// exists there. Unlike the trigger-scoped case above, this stays broken
/// regardless of what the trigger payload looks like at runtime, so it must
/// still hard-reject.
#[tokio::test]
async fn validate_required_arg_resolvability_rejects_an_explicit_nodes_reference() {
    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Manual" },
            { "id": "build_body", "kind": "code", "name": "Build Body",
              "config": { "language": "javascript", "source": "return {};" } },
            { "id": "post", "kind": "tool_call", "name": "Post",
              "config": { "slug": "GMAIL_SEND_EMAIL",
                "args": { "recipient_email": "a@b.com",
                  "subject": "=nodes.build_body.item.subject" } } }
        ],
        "edges": [
            { "from_node": "t", "to_node": "build_body" },
            { "from_node": "build_body", "to_node": "post" }
        ]
    }));
    let errors = validate_required_arg_resolvability(&g).await;
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("`subject`"), "{}", errors[0]);
    assert!(errors[0].contains("nodes.build_body"), "{}", errors[0]);
}

/// A required tool arg wired to a PLAIN agent node's (`no agent_ref`)
/// `output_parser.schema` field must pass this sandbox gate: the module's
/// schema-aware mock LLM synthesizes a
/// schema-valid completion, so the agent's output-parser sub-port succeeds and
/// the downstream `=nodes.<agent>.item.json.<field>` binding resolves to a typed
/// placeholder (non-null) instead of the run aborting on a schema-validation
/// failure. Without the mock LLM this gate would sink `propose_workflow`/`save`
/// on a correctly-built graph (the vendored `MockLlm` echo fails the sub-port).
#[tokio::test]
async fn validate_required_arg_resolvability_accepts_a_schema_agent_field_binding() {
    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Manual" },
            { "id": "summarize", "kind": "agent", "name": "Summarize",
              "config": { "prompt": "summarize the thread",
                "output_parser": { "schema": { "type": "object",
                    "required": ["channel"],
                    "properties": { "channel": { "type": "string" } } } } } },
            { "id": "post", "kind": "tool_call", "name": "Post",
              "config": { "slug": "SLACK_SEND_MESSAGE",
                "args": { "channel": "=nodes.summarize.item.json.channel" } } }
        ],
        "edges": [
            { "from_node": "t", "to_node": "summarize" },
            { "from_node": "summarize", "to_node": "post" }
        ]
    }));
    let errors = validate_required_arg_resolvability(&g).await;
    assert!(errors.is_empty(), "{errors:?}");
}

/// WS6: a required arg wired to the OUTPUT of an upstream Composio `tool_call`
/// must NOT be hard-rejected by this gate. The echo sandbox renders a Composio
/// `tool_call` as `{tool, args, connection}` and can never produce its real
/// output fields, so `=nodes.<composio>.item.json.data.<field>` resolves `null`
/// here even when the wiring is perfectly correct — rejecting it would block a
/// possibly-correct graph from ever being proposed (the transcript false
/// negative). Contrast `..._rejects_an_explicit_nodes_reference` above, where
/// the same explicit-`nodes` form addresses a `code` node (whose real output
/// the sandbox DOES produce) and stays a hard reject.
#[tokio::test]
async fn validate_required_arg_resolvability_downgrades_a_composio_tool_call_upstream_binding() {
    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Manual" },
            { "id": "get_me", "kind": "tool_call", "name": "Who am I",
              "config": { "slug": "TWITTER_USER_LOOKUP_ME", "args": {} } },
            { "id": "post", "kind": "tool_call", "name": "Post",
              "config": { "slug": "GMAIL_SEND_EMAIL",
                "args": { "recipient_email": "a@b.com", "subject": "hi",
                  "body": "=nodes.get_me.item.json.data.username" } } }
        ],
        "edges": [
            { "from_node": "t", "to_node": "get_me" },
            { "from_node": "get_me", "to_node": "post" }
        ]
    }));
    let errors = validate_required_arg_resolvability(&g).await;
    assert!(
        errors.is_empty(),
        "a binding to a Composio tool_call's output is UNVERIFIABLE, not a hard reject: {errors:?}"
    );
}

/// WS6 companion: the implicit `=item...` form of the same case — `post`'s only
/// predecessor is a Composio `tool_call`, so `=item.json.data.username`
/// addresses that node's (echo-only) output and is likewise unverifiable, not a
/// reject.
#[tokio::test]
async fn validate_required_arg_resolvability_downgrades_an_item_scoped_composio_upstream_binding() {
    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Manual" },
            { "id": "get_me", "kind": "tool_call", "name": "Who am I",
              "config": { "slug": "TWITTER_USER_LOOKUP_ME", "args": {} } },
            { "id": "post", "kind": "tool_call", "name": "Post",
              "config": { "slug": "GMAIL_SEND_EMAIL",
                "args": { "recipient_email": "a@b.com", "subject": "hi",
                  "body": "=item.json.data.username" } } }
        ],
        "edges": [
            { "from_node": "t", "to_node": "get_me" },
            { "from_node": "get_me", "to_node": "post" }
        ]
    }));
    let errors = validate_required_arg_resolvability(&g).await;
    assert!(errors.is_empty(), "{errors:?}");
}


// ── validate_agent_refs (agent-ref resolvability gate, PR #5114) ───────────

#[tokio::test]
async fn agent_ref_plain_node_without_ref_is_accepted() {
    // A plain `agent` node carries NO `agent_ref` — it runs on the default LLM
    // completion and never touches `OpenHumanAgentRunner`'s routing at all, so
    // this gate must never reject it. This is the exact invariant #5114 must
    // preserve: only an UNKNOWN `agent_ref` is rejected, never a plain node.
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Manual" },
            { "id": "a", "kind": "agent", "name": "Plan", "config": { "prompt": "outline it" } }
        ],
        "edges": [ { "from_node": "t", "to_node": "a" } ]
    }));
    let errors =
        validate_agent_refs(&crate::core::runtime::DispatchCtx::ambient(), &config, &g).await;
    assert!(errors.is_empty(), "{errors:?}");
}

#[tokio::test]
async fn agent_ref_blank_string_is_treated_as_absent() {
    // A whitespace-only `agent_ref` must be treated the same as no ref at all
    // rather than being resolved (and potentially rejected as "unknown").
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Manual" },
            { "id": "a", "kind": "agent", "name": "Plan",
              "config": { "agent_ref": "   ", "prompt": "outline it" } }
        ],
        "edges": [ { "from_node": "t", "to_node": "a" } ]
    }));
    let errors =
        validate_agent_refs(&crate::core::runtime::DispatchCtx::ambient(), &config, &g).await;
    assert!(errors.is_empty(), "{errors:?}");
}

#[tokio::test]
async fn agent_ref_resolving_to_a_harness_definition_is_accepted() {
    // "orchestrator" is one of the bundled built-in agent definitions
    // (see `agent_registry::defaults::default_agents_include_core_personas`),
    // so it must resolve via `AgentRoute::Harness` and never touch the
    // custom agent registry at all.
    //
    // This also exercises the CodeRabbit/Codex #5114 review fix: run via the
    // scoped `cargo test --lib flows::ops` filter, no other domain's test gets
    // to call `AgentDefinitionRegistry::init_global_builtins()` first, so this
    // only passes because `validate_agent_refs` now defensively initialises
    // the harness registry itself before resolving a ref.
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Manual" },
            { "id": "a", "kind": "agent", "name": "Plan",
              "config": { "agent_ref": "orchestrator", "prompt": "outline it" } }
        ],
        "edges": [ { "from_node": "t", "to_node": "a" } ]
    }));
    let errors =
        validate_agent_refs(&crate::core::runtime::DispatchCtx::ambient(), &config, &g).await;
    assert!(
        errors.is_empty(),
        "a real harness agent_ref must never be rejected: {errors:?}"
    );
}

#[tokio::test]
async fn agent_ref_unknown_is_rejected() {
    // The whole point of the gate (and the branch Codex flagged as uncovered on
    // #5114): an `agent` node whose `agent_ref` is NOT a real registered agent —
    // neither a bundled harness definition nor a custom registry entry — must be
    // REJECTED at author time, with the offending id named, rather than silently
    // hitting the `RegistryFallback` persona path at run time. Exercises the
    // error-construction branch of `validate_agent_refs`.
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Manual" },
            { "id": "a", "kind": "agent", "name": "Plan",
              "config": { "agent_ref": "no_such_agent_xyz", "prompt": "outline it" } }
        ],
        "edges": [ { "from_node": "t", "to_node": "a" } ]
    }));
    let errors =
        validate_agent_refs(&crate::core::runtime::DispatchCtx::ambient(), &config, &g).await;
    assert!(!errors.is_empty(), "an unknown agent_ref must be rejected");
    assert!(
        errors.iter().any(|e| e.contains("no_such_agent_xyz")),
        "the rejection error must name the offending agent_ref: {errors:?}"
    );
}

// ── dynamic agent_ref: refused at authoring, still reachable at run time ──

/// A `=`-expression `agent_ref` is no longer authorable. TinyFlows requires a
/// literal agent-registry reference so run data — which may include model
/// output — cannot choose an agent with different privileges, the same
/// reasoning this host already applies to `tool_call` slugs.
///
/// Pinned here rather than left to the vendor's own suite because the
/// `workflow_builder` agent can propose this shape, and the message a builder
/// sees on rejection is this host's contract with it.

/// The agent-ref gate resolves a custom agent through the context it is handed,
/// not the one the calling task happens to run under: two tenants' registries
/// never cross.
#[tokio::test]
async fn agent_ref_gate_reads_the_registry_of_the_explicit_context() {
    use crate::agent::registry::types::{AgentRegistryEntry, AgentRegistrySource};
    use crate::core::runtime::{CoreContext, DispatchCtx, DomainSet};

    let entry = AgentRegistryEntry {
        id: "tenant_a_only_agent".to_string(),
        name: "A".to_string(),
        description: "registered for tenant A".to_string(),
        source: AgentRegistrySource::Custom,
        enabled: true,
        model: None,
        system_prompt: None,
        tool_allowlist: Vec::new(),
        tool_denylist: Vec::new(),
        tool_rules: None,
        subagents: Default::default(),
        tags: Vec::new(),
        metadata: Value::Null,
    };
    let tmp_a = TempDir::new().unwrap();
    let tmp_b = TempDir::new().unwrap();
    let mut config_a = test_config(&tmp_a);
    config_a.agent_registry.entries.push(entry);
    let config_b = test_config(&tmp_b);
    let core_a = CoreContext::for_test_with_config(DomainSet::full(), config_a.clone());
    let core_b = CoreContext::for_test_with_config(DomainSet::full(), config_b.clone());
    let ctx_a = DispatchCtx::new(core_a);
    let ctx_b = DispatchCtx::new(std::sync::Arc::clone(&core_b));

    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Manual" },
            { "id": "a", "kind": "agent", "name": "Plan",
              "config": { "agent_ref": "tenant_a_only_agent", "prompt": "go" } }
        ],
        "edges": [ { "from_node": "t", "to_node": "a" } ]
    }));

    // Both calls run scoped to tenant B, so an ambient read would answer B's
    // registry for A's call.
    let (for_a, for_b) = CoreContext::scope(core_b, async {
        (
            validate_agent_refs(&ctx_a, &config_a, &g).await,
            validate_agent_refs(&ctx_b, &config_b, &g).await,
        )
    })
    .await;
    assert!(
        for_a.is_empty(),
        "tenant A's own agent must resolve: {for_a:?}"
    );
    assert!(
        for_b.iter().any(|e| e.contains("tenant_a_only_agent")),
        "tenant B must not see tenant A's agent: {for_b:?}"
    );
}

#[tokio::test]
async fn validate_tool_contracts_accepts_a_separated_action_slug_alias() {
    seed_live_catalog_cache("googledrive", vec![seeded_alias_contract()]);
    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Manual" },
            { "id": "find", "kind": "tool_call", "name": "Find",
              "config": { "slug": "GOOGLE_DRIVE_FIND_FILE", "args": {} } }
        ],
        "edges": [ { "from_node": "t", "to_node": "find" } ]
    }));

    let errors = validate_tool_contracts(&Config::default(), &g).await;

    assert!(
        errors.is_empty(),
        "valid action alias was rejected: {errors:?}"
    );
}

#[tokio::test]
async fn graph_wiring_warnings_resolves_a_separated_action_slug_alias() {
    seed_live_catalog_cache("googledrive", vec![seeded_alias_contract()]);
    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Manual" },
            { "id": "find", "kind": "tool_call", "name": "Find",
              "config": { "slug": "GOOGLE_DRIVE_FIND_FILE", "args": {} } },
            { "id": "split", "kind": "split_out", "name": "Split",
              "config": { "path": "json.data.wrong" } },
            { "id": "shape", "kind": "transform", "name": "Shape",
              "config": { "set": { "file": "=nodes.find.item.json.data.wrong" } } }
        ],
        "edges": [
            { "from_node": "t", "to_node": "find" },
            { "from_node": "find", "to_node": "split" },
            { "from_node": "find", "to_node": "shape" }
        ]
    }));

    let warnings = graph_wiring_warnings(&Config::default(), &g).await;

    assert_eq!(
        warnings.len(),
        2,
        "alias should reach both wiring checks: {warnings:?}"
    );
    assert!(
        warnings
            .iter()
            .any(|warning| warning.contains("json.data.files")),
        "{warnings:?}"
    );
    assert!(
        warnings.iter().any(|warning| warning.contains("wrong")),
        "{warnings:?}"
    );
}

/// (Codex feedback on this PR) `notion` ships a static curated catalog
/// (`catalog_for_toolkit`), so at RUNTIME `flow_tool_allowed`'s Path A
/// hard-rejects any slug `find_curated` doesn't recognize — even a real,
/// live action. Without this check, a real-but-uncurated action for a
/// statically-catalogued toolkit would pass authoring/save here and then
/// fail every single run as "tool not permitted". Uses its own toolkit key
/// (`notion`, not `slack`/`gmail`) since it seeds different `is_curated`
/// content than every other test sharing those keys.

#[tokio::test]
async fn validate_tool_contracts_rejects_a_real_but_uncurated_action_on_a_statically_catalogued_toolkit(
) {
    seed_live_catalog_cache(
        "notion",
        vec![ToolContract {
            slug: "NOTION_UNCURATED_ACTION".to_string(),
            toolkit: "notion".to_string(),
            description: None,
            required_args: vec![],
            input_schema: None,
            output_fields: vec![],
            output_schema: None,
            primary_array_path: None,
            // Real (a live catalog fetch found it), but NOT one of
            // OpenHuman's curated Notion actions.
            is_curated: false,
        }],
    );
    let config = Config::default();
    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Manual" },
            { "id": "post", "kind": "tool_call", "name": "Post",
              "config": { "slug": "NOTION_UNCURATED_ACTION", "args": {} } }
        ],
        "edges": [ { "from_node": "t", "to_node": "post" } ]
    }));
    let errors = validate_tool_contracts(&config, &g).await;
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("NOTION_UNCURATED_ACTION"),
        "{}",
        errors[0]
    );
    assert!(errors[0].contains("curated"), "{}", errors[0]);
}


#[tokio::test]
async fn validate_tool_contracts_skips_expression_derived_and_native_slugs() {
    let config = Config::default();
    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Manual" },
            { "id": "dynamic", "kind": "tool_call", "name": "Dynamic",
              "config": { "slug": "=item.tool", "args": {} } },
            { "id": "native", "kind": "tool_call", "name": "Native",
              "config": { "slug": "oh:web_search", "args": {} } }
        ],
        "edges": [
            { "from_node": "t", "to_node": "dynamic" },
            { "from_node": "t", "to_node": "native" }
        ]
    }));
    let errors = validate_tool_contracts(&config, &g).await;
    assert!(errors.is_empty(), "{errors:?}");
}


#[tokio::test]
async fn validate_tool_contracts_skips_rather_than_rejects_when_the_catalog_is_unreachable() {
    // No seed for this toolkit and no live backend configured — the fetch
    // fails, and the node must be SKIPPED (never false-rejected).
    let config = Config::default();
    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Manual" },
            { "id": "post", "kind": "tool_call", "name": "Post",
              "config": { "slug": "SOMEUNSEEDEDTOOLKIT_DO_THING", "args": {} } }
        ],
        "edges": [ { "from_node": "t", "to_node": "post" } ]
    }));
    let errors = validate_tool_contracts(&config, &g).await;
    assert!(
        errors.is_empty(),
        "a live-catalog fetch failure must skip, not reject: {errors:?}"
    );
}


#[tokio::test]
async fn validate_tool_contracts_rejects_an_arg_name_not_in_the_input_schema() {
    seed_live_catalog_cache(
        "slackargnametest",
        vec![seeded_slack_send_message_contract_with_schema()],
    );
    let config = Config::default();
    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Manual" },
            { "id": "post", "kind": "tool_call", "name": "Post",
              "config": { "slug": "SLACKARGNAMETEST_SEND_MESSAGE",
                "args": { "channel": "#general", "text": "hi" } } }
        ],
        "edges": [ { "from_node": "t", "to_node": "post" } ]
    }));
    let errors = validate_tool_contracts(&config, &g).await;
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("post"), "{}", errors[0]);
    assert!(errors[0].contains("`text`"), "{}", errors[0]);
    assert!(errors[0].contains("markdown_text"), "{}", errors[0]);
    assert!(errors[0].contains("get_tool_contract"), "{}", errors[0]);
}


#[tokio::test]
async fn validate_tool_contracts_passes_the_real_arg_name_from_the_input_schema() {
    seed_live_catalog_cache(
        "slackargnametest",
        vec![seeded_slack_send_message_contract_with_schema()],
    );
    let config = Config::default();
    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Manual" },
            { "id": "post", "kind": "tool_call", "name": "Post",
              "config": { "slug": "SLACKARGNAMETEST_SEND_MESSAGE",
                "args": { "channel": "#general", "markdown_text": "hi" } } }
        ],
        "edges": [ { "from_node": "t", "to_node": "post" } ]
    }));
    let errors = validate_tool_contracts(&config, &g).await;
    assert!(errors.is_empty(), "{errors:?}");
}

/// Uses its own cache key/toolkit (never `"slack"`/`"gmail"`) since the
/// arg-name check must behave identically no matter which slug it's
/// exercised against, and a dedicated, unregistered toolkit sidesteps both
/// the process-global `LIVE_CATALOG_CACHE` sharing risk the other
/// `validate_tool_contracts` tests accept AND the static curated-catalog
/// gate (this toolkit has none, so `is_curated` is irrelevant here).

#[tokio::test]
async fn validate_tool_contracts_skips_arg_name_check_when_input_schema_is_unknown() {
    seed_live_catalog_cache(
        "argschemaunknown",
        vec![ToolContract {
            slug: "ARGSCHEMAUNKNOWN_DO_THING".to_string(),
            toolkit: "argschemaunknown".to_string(),
            description: None,
            required_args: vec![],
            input_schema: None,
            output_fields: vec![],
            output_schema: None,
            primary_array_path: None,
            is_curated: false,
        }],
    );
    let config = Config::default();
    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Manual" },
            { "id": "post", "kind": "tool_call", "name": "Post",
              "config": { "slug": "ARGSCHEMAUNKNOWN_DO_THING",
                "args": { "totally_made_up_field": "hi" } } }
        ],
        "edges": [ { "from_node": "t", "to_node": "post" } ]
    }));
    let errors = validate_tool_contracts(&config, &g).await;
    assert!(
        errors.is_empty(),
        "an unknown input_schema must skip the arg-name check, never reject: {errors:?}"
    );
}


#[tokio::test]
async fn validate_tool_contracts_allows_arbitrary_arg_names_when_schema_permits_additional_properties(
) {
    seed_live_catalog_cache(
        "argschemaadditional",
        vec![ToolContract {
            slug: "ARGSCHEMAADDITIONAL_DO_THING".to_string(),
            toolkit: "argschemaadditional".to_string(),
            description: None,
            required_args: vec![],
            input_schema: Some(json!({
                "type": "object",
                "properties": { "channel": { "type": "string" } },
                "additionalProperties": true
            })),
            output_fields: vec![],
            output_schema: None,
            primary_array_path: None,
            is_curated: false,
        }],
    );
    let config = Config::default();
    let g = graph(json!({
        "nodes": [
            { "id": "t", "kind": "trigger", "name": "Manual" },
            { "id": "post", "kind": "tool_call", "name": "Post",
              "config": { "slug": "ARGSCHEMAADDITIONAL_DO_THING",
                "args": { "channel": "#general", "any_extra_field": "hi" } } }
        ],
        "edges": [ { "from_node": "t", "to_node": "post" } ]
    }));
    let errors = validate_tool_contracts(&config, &g).await;
    assert!(
        errors.is_empty(),
        "additionalProperties: true must allow arbitrary arg names: {errors:?}"
    );
}

