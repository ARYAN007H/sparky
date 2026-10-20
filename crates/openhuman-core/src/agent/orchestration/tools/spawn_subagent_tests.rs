use super::*;
use crate::agent::subagent_host::SubagentMode;
use std::time::Duration;
use tempfile::TempDir;

fn sample_outcome(output: &str) -> SubagentRunOutcome {
    SubagentRunOutcome {
        agent_id: "task_manager_agent".into(),
        task_id: "sub-test-1".into(),
        output: output.to_string(),
        elapsed: Duration::from_millis(120),
        iterations: 3,
        failure_cause: None,
        mode: SubagentMode::Typed,
        status: SubagentRunStatus::Completed,
        final_history: Vec::new(),
        usage: Default::default(),
        artifact_paths: Vec::new(),
        persistence_disposition:
            tinyagents_orchestration::subagent::SubagentPersistenceDisposition::TerminalInserted,
    }
}

#[test]
fn build_worker_thread_title_collapses_whitespace_and_caps_length() {
    let prompt =
        "  draft\n a very long\tplan that\nrambles ".to_string() + "x".repeat(200).as_str();
    let title = build_worker_thread_title(&prompt);
    assert!(title.starts_with("draft a very long plan"));
    assert!(title.chars().count() <= WORKER_THREAD_TITLE_MAX_CHARS + 1);
    assert!(title.ends_with('…'));
}

#[test]
fn build_worker_thread_title_falls_back_when_empty() {
    assert_eq!(build_worker_thread_title("   \n\t  "), "Worker task");
}

#[test]
fn parameters_schema_hides_legacy_mode_flags_and_keeps_required_agent_id() {
    let tool = SpawnSubagentTool::new();
    let schema = tool.parameters_schema();
    let props = schema.get("properties").expect("schema has properties");
    for legacy in ["blocking", "dedicated_thread", "archetype", "toolkit"] {
        assert!(props.get(legacy).is_none(), "legacy `{legacy}` is not advertised");
    }
    assert!(schema["required"].as_array().is_some_and(|required| {
        required.iter().any(|key| key.as_str() == Some("agent_id"))
    }));
}

#[test]
fn parameters_schema_advertises_optional_model_override() {
    let tool = SpawnSubagentTool::new();
    let schema = tool.parameters_schema();
    let props = schema.get("properties").expect("schema has properties");
    let model = props.get("model").expect("model override advertised");
    assert_eq!(model.get("type").and_then(|v| v.as_str()), Some("string"));
    assert!(schema
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().all(|s| s.as_str() != Some("model")))
        .unwrap_or(true));
}

#[test]
fn render_worker_thread_result_carries_machine_readable_envelope() {
    let outcome = sample_outcome("done");
    let rendered = render_worker_thread_result("worker-abc", "task_manager_agent", &outcome);
    assert!(rendered.contains("Spawned worker thread `worker-abc`"));
    assert!(rendered.contains("[worker_thread_ref]"));
    assert!(rendered.contains("[/worker_thread_ref]"));
    // The JSON payload between the markers must round-trip.
    let start = rendered.find("[worker_thread_ref]\n").unwrap() + "[worker_thread_ref]\n".len();
    let end = rendered.find("\n[/worker_thread_ref]").unwrap();
    let payload: serde_json::Value =
        serde_json::from_str(&rendered[start..end]).expect("valid json envelope");
    assert_eq!(payload["thread_id"], "worker-abc");
    assert_eq!(payload["label"], "worker");
    assert_eq!(payload["agent_id"], "task_manager_agent");
    assert_eq!(payload["task_id"], "sub-test-1");
    assert_eq!(payload["iterations"], 3);
}

#[test]
fn persist_worker_thread_creates_thread_with_tasks_label_and_messages() {
    let temp = TempDir::new().expect("tempdir");
    let outcome = sample_outcome("the answer is 42");
    let thread_id = persist_worker_thread(
        &crate::core::runtime::DispatchCtx::ambient(),
        temp.path(),
        "task_manager_agent",
        "draft a long research plan",
        &outcome,
    )
    .expect("worker thread persisted");

    assert!(thread_id.starts_with("worker-"));

    let threads = conversations::list_threads(
        &crate::core::runtime::DispatchCtx::ambient(),
        temp.path().to_path_buf(),
    )
    .expect("list threads");
    let worker = threads
        .iter()
        .find(|t| t.id == thread_id)
        .expect("worker thread present");
    assert!(worker.labels.contains(&"tasks".to_string()));
    assert!(worker.title.starts_with("draft a long research plan"));

    let messages = conversations::get_messages(
        &crate::core::runtime::DispatchCtx::ambient(),
        temp.path().to_path_buf(),
        &thread_id,
    )
    .expect("messages");
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].sender, "user");
    assert_eq!(messages[0].content, "draft a long research plan");
    assert_eq!(messages[1].sender, "agent");
    assert_eq!(messages[1].content, "the answer is 42");
    assert_eq!(messages[1].extra_metadata["iterations"], 3);
    assert_eq!(messages[1].extra_metadata["scope"], "worker_thread");
}

#[tokio::test]
async fn missing_agent_id_returns_error() {
    let tool = SpawnSubagentTool::new();
    let result = tool
        .execute(json!({
            "prompt": "do thing"
        }))
        .await
        .unwrap();
    assert!(result.is_error);
    assert!(result.output().contains("agent_id"));
}

#[tokio::test]
async fn missing_prompt_returns_error() {
    let tool = SpawnSubagentTool::new();
    let result = tool
        .execute(json!({
            "agent_id": "task_manager_agent"
        }))
        .await
        .unwrap();
    assert!(result.is_error);
    assert!(result.output().contains("prompt"));
}

#[tokio::test]
async fn no_registry_returns_clear_error() {
    // The global registry has not been initialised in this test.
    let tool = SpawnSubagentTool::new();
    let result = tool
        .execute(json!({
            "agent_id": "task_manager_agent",
            "prompt": "find x",
        }))
        .await
        .unwrap();
    // Either: registry uninitialised → clear init error, OR
    // registry was initialised by a previous test → "no parent context"
    // because we're not running inside an OpenHumanSessionHost::turn. Both are
    // acceptable: the tool gracefully refuses.
    assert!(result.is_error);
}

#[tokio::test]
async fn unknown_agent_id_lists_available() {
    // Force-init the global registry with builtins.
    let _ = AgentDefinitionRegistry::init_global_builtins();
    let tool = SpawnSubagentTool::new();
    let result = tool
        .execute(json!({
            "agent_id": "totally_made_up",
            "prompt": "x",
        }))
        .await
        .unwrap();
    assert!(result.is_error);
    let out = result.output();
    // Should list at least one valid built-in.
    assert!(out.contains("task_manager_agent"));
}

#[test]
fn classify_subagent_failure_reframes_upstream_provider_outages() {
    let msg = SpawnSubagentTool::classify_subagent_failure(
        "provider call failed: all providers/models failed: upstream unavailable",
    );
    assert!(msg.contains("upstream inference unavailable"));
    assert!(msg.contains("NOT a Composio/integration auth issue"));
}

#[test]
fn frozen_legacy_mode_arguments_are_accepted_but_ignored() {
    let args = json!({
        "agent_id": "task_manager_agent",
        "prompt": "inspect the frozen-session compatibility route",
        "blocking": true,
        "dedicated_thread": true,
    });
    let decoded = decode_async_spawn_args(&args);
    let plain = decode_async_spawn_args(&json!({
        "agent_id": "task_manager_agent",
        "prompt": "inspect the frozen-session compatibility route",
    }));
    assert_eq!(decoded.agent_id, "task_manager_agent");
    assert_eq!(decoded.prompt, "inspect the frozen-session compatibility route");
    assert_eq!(decoded.task_key, plain.task_key);
    assert!(!decoded.force_fresh);
}
