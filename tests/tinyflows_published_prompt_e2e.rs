//! Normal-host smoke test for the published TinyFlows Copilot prompt contract.
//!
//! Run manually with a native TinyFlows release artifact staged locally:
//! `OPENHUMAN_TINYFLOWS_PUBLISHED_LIBRARY=/path/to/libtinyflows_module.dylib \
//! cargo test -p openhuman-cli --test tinyflows_published_prompt_e2e -- --ignored`.

#[tokio::test]
#[ignore = "set OPENHUMAN_TINYFLOWS_PUBLISHED_LIBRARY to a published native module artifact"]
async fn published_module_prompts_render_through_the_normal_host_path() {
    let library = std::env::var("OPENHUMAN_TINYFLOWS_PUBLISHED_LIBRARY")
        .expect("published module library path is required");
    let library = std::path::PathBuf::from(library);
    assert!(library.is_file(), "published module library must exist");

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
            "[components]\ntinyflows = true\n\n[[modules.overrides]]\nid = \"tinyflows\"\npath = {quoted_library}\n"
        ),
    )
    .expect("module override config");

    for (agent_id, required) in [
        ("workflow_builder", "propose_workflow"),
        ("flow_discovery", "suggest_workflows"),
    ] {
        let mut options = openhuman_core::agent::debug::DumpPromptOptions::new(agent_id);
        options.workspace_dir_override = Some(workspace.clone());
        options.config_path_override = Some(config_path.clone());
        let prompt = openhuman_core::agent::debug::dump_agent_prompt(options)
            .await
            .unwrap_or_else(|error| panic!("rendering {agent_id} prompt: {error:#}"));
        assert!(prompt.text.contains(required), "{agent_id} prompt missing {required}");
        assert!(prompt.text.contains("## Workspace"));
    }
}
