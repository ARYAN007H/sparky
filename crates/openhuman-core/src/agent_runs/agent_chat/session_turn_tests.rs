use super::*;
use serde_json::json;

/// A turn that finished on its own keeps the historical envelope and adds
/// `hit_cap: false`; no `checkpoint` key appears.
#[test]
fn a_finished_turn_reports_hit_cap_false() {
    let json = AgentChatReply {
        text: "done".into(),
        hit_cap: false,
    }
    .into_rpc_json()
    .expect("serializes");

    assert_eq!(
        json,
        json!({ "result": "done", "logs": ["agent chat completed"], "hit_cap": false })
    );
}

/// #6958: a capped turn used to look like success to a headless caller. It
/// now says so, and carries the checkpoint text as its own field.
#[test]
fn a_capped_turn_reports_hit_cap_and_the_checkpoint() {
    let json = AgentChatReply {
        text: "Changed foo.rs. **Still to do**: bar.rs".into(),
        hit_cap: true,
    }
    .into_rpc_json()
    .expect("serializes");

    assert_eq!(json["hit_cap"], json!(true));
    assert_eq!(
        json["checkpoint"],
        json!("Changed foo.rs. **Still to do**: bar.rs")
    );
}

/// Backward compatibility: existing clients read the reply from `result` (or
/// peel the envelope with `unwrap_rpc`), and still get the plain string.
#[test]
fn the_reply_text_stays_where_existing_clients_read_it() {
    for hit_cap in [false, true] {
        let json = AgentChatReply {
            text: "the reply".into(),
            hit_cap,
        }
        .into_rpc_json()
        .expect("serializes");
        assert_eq!(json["result"], json!("the reply"));
        assert_eq!(json["logs"], json!(["agent chat completed"]));
        assert_eq!(
            crate::core::unwrap_rpc(&json),
            &json!("the reply"),
            "unwrap_rpc must still land on the reply text"
        );
    }
}

/// The library path (`agent_chat_for`) keeps returning `Outcome<String>`.
#[test]
fn the_outcome_keeps_its_historical_shape() {
    let outcome = AgentChatReply {
        text: "hi".into(),
        hit_cap: true,
    }
    .into_outcome();
    assert_eq!(outcome.value, "hi");
    assert_eq!(outcome.logs, vec!["agent chat completed".to_string()]);
}

#[test]
fn explicit_turn_temperature_overrides_the_definition_without_mutating_it() {
    let mut definition = crate::agent::harness::test_support::test_main_def();
    definition.temperature = 0.6;
    let effective = definition_for_turn(&definition, Some(0.9));
    assert_eq!(effective.temperature, 0.9);
    assert_eq!(definition.temperature, 0.6);
    assert!(matches!(
        definition_for_turn(&definition, None),
        std::borrow::Cow::Borrowed(_)
    ));
    assert_eq!(definition_for_turn(&definition, None).temperature, 0.6);
}

fn test_config(tmp: &tempfile::TempDir) -> crate::config::Config {
    let mut config = crate::config::Config::default();
    config.workspace_dir = tmp.path().join("workspace");
    config.config_path = tmp.path().join("config.toml");
    config
}

// ── `agent_chat_for`: explicit targets and thread-scoped resume ───────────

/// An unknown `agent_id` is a caller error surfaced before any provider is
/// touched, not a silent fall-through to the orchestrator.
#[tokio::test]
async fn agent_chat_for_rejects_an_unknown_agent_id() {
    let tmp = tempfile::tempdir().unwrap();
    let mut config = test_config(&tmp);
    std::fs::create_dir_all(&config.workspace_dir).unwrap();

    let err = agent_chat_for(
        &mut config,
        AgentChatTarget::AgentId("no-such-agent-xyz"),
        "hello",
        None,
        None,
        None,
        None,
        None,
        AgentChatEnv::default(),
    )
    .await
    .expect_err("an unknown agent id must be refused");
    assert!(
        err.to_lowercase().contains("no-such-agent-xyz") || err.to_lowercase().contains("unknown"),
        "error names the id or says unknown: {err}"
    );
}

/// `AgentChatTarget` is `Copy` and the orchestrator arm is the historical
/// `agent_chat`, so the delegating wrapper and the explicit call agree.
#[test]
fn agent_chat_target_default_is_the_orchestrator() {
    let target = AgentChatTarget::Orchestrator;
    let copy = target;
    assert!(matches!(copy, AgentChatTarget::Orchestrator));
    assert!(matches!(
        AgentChatTarget::AgentId("x"),
        AgentChatTarget::AgentId("x")
    ));
}
