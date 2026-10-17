use super::*;

fn http_cred_store() -> (tempfile::TempDir, HttpCredentialsStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    // encrypt=true exercises the ChaCha20-Poly1305 at-rest path.
    let store = HttpCredentialsStore::new(dir.path(), true);
    (dir, store)
}

// ── Phase 2: autonomy-tier gating of acting nodes ──────────────────────

fn policy(level: crate::security::AutonomyLevel) -> SecurityPolicy {
    SecurityPolicy {
        autonomy: level,
        ..SecurityPolicy::default()
    }
}

// ── Codex P1: Prompt-tier decisions must escalate past a workflow's own
// require_approval=false default, never silently auto-allow ────────────

use crate::agent::turn_origin::{AgentTurnOrigin, TrustedAutomationSource};

fn workflow_origin(job_id: &str, require_approval: bool) -> AgentTurnOrigin {
    AgentTurnOrigin::TrustedAutomation {
        job_id: job_id.to_string(),
        source: TrustedAutomationSource::Workflow { require_approval },
    }
}

// ── Phase 7: sub_workflow-by-id resolver ───────────────────────────────

fn resolver_test_config(tmp: &tempfile::TempDir) -> Config {
    let config = Config {
        workspace_dir: tmp.path().join("workspace"),
        action_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("config.toml"),
        ..Config::default()
    };
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    config
}

fn trigger_only_graph() -> WorkflowGraph {
    use tinyflows::model::{Node, NodeKind};
    WorkflowGraph {
        nodes: vec![Node {
            id: "t".to_string(),
            kind: NodeKind::Trigger,
            type_version: 1,
            name: "Trigger".to_string(),
            config: Value::Null,
            ports: Vec::new(),
            position: None,
        }],
        ..Default::default()
    }
}

#[path = "ops_schema_and_structured_output_tests.rs"]
mod schema_and_structured_output_tests;
#[path = "ops_tier_gate_and_timeouts_tests.rs"]
mod tier_gate_and_timeouts_tests;
#[path = "ops_tool_results_and_credentials_tests.rs"]
mod tool_results_and_credentials_tests;
