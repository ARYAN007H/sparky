//! Normal-host prompt composition against the checksum-verified TinyFlows v0.8.26 module.

#[path = "support/env_guard.rs"]
mod env_guard;
#[path = "support/published_tinyflows.rs"]
mod published_tinyflows;

use env_guard::EnvVarGuard;

#[tokio::test]
async fn published_module_prompts_render_through_the_normal_host_path() {
    let _source_guard = EnvVarGuard::unset("OPENHUMAN_MODULE_SOURCE");
    let _source_dir_guard = EnvVarGuard::unset("OPENHUMAN_MODULE_SOURCE_DIR");
    let _tinyflows_source_guard = EnvVarGuard::unset("OPENHUMAN_MODULE_SOURCE_TINYFLOWS");
    let (library, _) = published_tinyflows::staged_v0_8_26_module();
    let temp = tempfile::tempdir().expect("scratch directory");
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace directory");
    let config_path = temp.path().join("config.toml");
    let quoted_library = serde_json::to_string(
        library
            .to_str()
            .expect("published module path is valid UTF-8"),
    )
    .expect("TOML-compatible quoted path");
    std::fs::write(
        &config_path,
        format!(
            "[components]\ntinyflows = true\n\n[modules]\nenabled = true\n\n[[modules.overrides]]\nid = \"tinyflows\"\npath = {quoted_library}\n"
        ),
    )
    .expect("module override config");

    for (agent_id, required, invariant) in [
        ("workflow_builder", "propose_workflow", "propose-only"),
        ("flow_discovery", "suggest_workflows", "read-only"),
    ] {
        let mut options = openhuman_core::agent::debug::DumpPromptOptions::new(agent_id);
        options.workspace_dir_override = Some(workspace.clone());
        options.config_path_override = Some(config_path.clone());
        let prompt = openhuman_core::agent::debug::dump_agent_prompt(options)
            .await
            .unwrap_or_else(|error| panic!("rendering {agent_id} prompt: {error:#}"));
        let lower = prompt.text.to_lowercase();
        assert!(
            prompt.text.contains(required),
            "{agent_id} prompt missing {required}"
        );
        assert!(
            lower.contains(invariant),
            "{agent_id} prompt missing {invariant}"
        );
        assert!(prompt.text.contains("## Workspace"));
    }
}
