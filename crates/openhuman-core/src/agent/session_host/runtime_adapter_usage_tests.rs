use super::runtime_session::{account_committed_turn_against_goal, holistic_last_turn_usage};

#[test]
fn inclusive_child_usage_counts_each_public_entry_once_and_keeps_unknown_cost() {
    let charged = tinyagents_harness::cost::CostTally {
        known_usd: 0.002,
        source: tinyagents_harness::cost::CostSource::Charged,
    };
    let mut sidecar = crate::agent::tinyagents::host::run_context::SessionTurnSidecar {
        input_tokens: 10,
        output_tokens: 4,
        cost: charged,
        ..Default::default()
    };
    sidecar.subagents = vec![
        crate::agent::tinyagents::host::SubagentUsageEntry {
            task_id: "reused-task".into(),
            agent_id: "researcher".into(),
            usage: crate::agent::subagent_host::SubagentUsage {
                // Includes this run's grandchild totals; that grandchild is
                // intentionally absent as a separate public ledger entry.
                input_tokens: 20,
                output_tokens: 8,
                cached_input_tokens: 3,
                charged_amount_usd: charged.known_usd,
                cost_source: charged.source,
            },
        },
        crate::agent::tinyagents::host::SubagentUsageEntry {
            // A second entry for the same task remains a distinct legacy
            // contribution; this public ledger carries no run identity.
            task_id: "reused-task".into(),
            agent_id: "researcher".into(),
            usage: crate::agent::subagent_host::SubagentUsage {
                input_tokens: 7,
                output_tokens: 2,
                cost_source: tinyagents_harness::cost::CostSource::Unknown,
                ..Default::default()
            },
        },
    ];

    let usage = holistic_last_turn_usage(&sidecar);
    assert_eq!(usage.input_tokens, 37);
    assert_eq!(usage.output_tokens, 14);
    assert_eq!(usage.cached_input_tokens, 3);
    assert_eq!(usage.subagents.len(), 2);
    assert_eq!(usage.cost_usd, None, "unknown child cost stays unknown");
    assert_eq!(
        usage.cost_source,
        tinyagents_harness::cost::CostSource::Unknown
    );
}

#[tokio::test]
async fn committed_goal_accounting_saturates_direct_plus_inclusive_child_tokens() {
    let workspace = tempfile::tempdir().unwrap();
    tinyagents_graph::goals::store::set(
        &crate::storage::sessions::stores_or_ephemeral(workspace.path()).kv,
        "saturated-goal",
        "complete safely",
        Some(u64::MAX),
    )
    .await
    .unwrap();
    let mut sidecar = crate::agent::tinyagents::host::run_context::SessionTurnSidecar {
        input_tokens: u64::MAX - 1,
        output_tokens: u64::MAX - 2,
        ..Default::default()
    };
    sidecar
        .subagents
        .push(crate::agent::tinyagents::host::SubagentUsageEntry {
            task_id: "large-child".into(),
            agent_id: "researcher".into(),
            usage: crate::agent::subagent_host::SubagentUsage {
                input_tokens: 10,
                output_tokens: 10,
                ..Default::default()
            },
        });

    account_committed_turn_against_goal(workspace.path(), Some("saturated-goal"), &sidecar).await;

    let goal = tinyagents_graph::goals::store::get(
        &crate::storage::sessions::stores_or_ephemeral(workspace.path()).kv,
        "saturated-goal",
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(goal.tokens_used, u64::MAX);
}
