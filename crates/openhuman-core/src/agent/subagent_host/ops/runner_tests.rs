use super::*;

#[tokio::test]
async fn byte_policy_offloads_full_output_before_applying_the_character_cap() {
    use crate::core::runtime::agent_scope::test_agent_context;
    use crate::core::runtime::{CoreContext, DomainSet};
    use crate::security::{SecurityPolicy, TrustedAccess, TrustedRoot};

    let scratch = tempfile::tempdir().expect("artifact workspace");
    let action_dir = scratch.path().join("action");
    let workspace_dir = scratch.path().join("workspace");
    std::fs::create_dir_all(&action_dir).expect("action directory");
    std::fs::create_dir_all(&workspace_dir).expect("workspace directory");
    let trusted_action_dir = action_dir
        .canonicalize()
        .expect("canonical action directory");
    let policy = Arc::new(SecurityPolicy {
        trusted_roots: vec![TrustedRoot {
            path: trusted_action_dir.to_string_lossy().into_owned(),
            access: TrustedAccess::ReadWrite,
        }],
        action_dir: action_dir.clone(),
        workspace_dir,
        ..SecurityPolicy::default()
    });
    let root = CoreContext::for_test(DomainSet::full(), None);
    let scoped = test_agent_context(&root, "subagent-artifact-policy-test");
    let output = "full sub-agent result that must survive the visible preview\n".repeat(16);
    let store = Arc::new(SubagentArtifactStore::new(
        Some(policy),
        action_dir.clone(),
        "worker".into(),
    ));

    let stored = CoreContext::scope(scoped, async {
        offload_result_by_policy("task-policy-test", &output, 8, &store)
            .await
            .expect("TinyAgents byte policy invokes the host artifact store")
    })
    .await;

    assert!(stored.relative_path.starts_with("outputs/"));
    assert!(stored.pointer.contains(&stored.relative_path));
    assert_ne!(stored.pointer, output);
    let materialized = std::fs::read_to_string(action_dir.join(&stored.relative_path))
        .expect("parent-readable artifact was materialized under its action root");
    assert_eq!(materialized, output, "the full original output is retained");

    let capped = tinyagents_orchestration::subagent::ResultPolicy::new()
        .with_max_chars(512)
        .apply("task-policy-test", &stored.pointer, None)
        .await;
    assert_eq!(
        capped.text, stored.pointer,
        "the existing pointer fits its cap"
    );
}
