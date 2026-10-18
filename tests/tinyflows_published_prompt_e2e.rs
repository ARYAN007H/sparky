//! Normal-host prompt composition against the checksum-verified TinyFlows v0.8.26 module.

#[path = "support/env_guard.rs"]
mod env_guard;
#[path = "support/published_tinyflows.rs"]
mod published_tinyflows;

use env_guard::EnvVarGuard;

#[tokio::test]
async fn published_module_prompts_render_through_the_normal_host_path() {
    let _env_lock = env_guard::env_lock_async().await;
    let temp = tempfile::tempdir().expect("isolated home and workspace");
    let _home_guard = EnvVarGuard::set_to_path("HOME", temp.path());
    let _backend_url_guard = EnvVarGuard::unset("BACKEND_URL");
    let _vite_backend_guard = EnvVarGuard::unset("VITE_BACKEND_URL");
    let _api_key_guard = EnvVarGuard::unset("OPENHUMAN_BACKEND_API_KEY");
    let _session_guard = EnvVarGuard::unset("OPENHUMAN_BACKEND_SESSION_TOKEN");
    let _storage_guard = EnvVarGuard::unset("OPENHUMAN_STORAGE_URL");
    let _source_guard = EnvVarGuard::unset("OPENHUMAN_MODULE_SOURCE");
    let _source_dir_guard = EnvVarGuard::unset("OPENHUMAN_MODULE_SOURCE_DIR");
    let _tinyflows_source_guard = EnvVarGuard::unset("OPENHUMAN_MODULE_SOURCE_TINYFLOWS");
    let (library, _) = published_tinyflows::staged_v0_8_26_module();
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace directory");
    let _workspace_guard = EnvVarGuard::set_to_path("OPENHUMAN_WORKSPACE", &workspace);
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
            "api_url = \"http://127.0.0.1:1\"\n\n[components]\ntinyflows = true\n\n[modules]\nenabled = true\n\n[[modules.overrides]]\nid = \"tinyflows\"\npath = {quoted_library}\n"
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

    // This is the actual published module path: the host must materialize the
    // bus-owned skill, then the normal TinySkills discovery and resource reader
    // must expose that Builtin under the separate dynamic root.
    let config = openhuman_core::config::Config::load_from_config_path(&config_path, &workspace)
        .await
        .expect("load the same scoped Config used by the prompt dump");
    let ctx = openhuman_core::core::runtime::DispatchCtx::host_default();
    let discovered = openhuman_core::skills::load_workflow_metadata_for(&ctx, &config);
    let flow = discovered
        .iter()
        .find(|skill| skill.id() == "flow-authoring")
        .expect("the published module's flow-authoring skill is discoverable");
    assert_eq!(flow.scope, tinyskills::SkillScope::Builtin);
    let flow_dir = flow
        .location
        .as_deref()
        .and_then(std::path::Path::parent)
        .expect("discovered flow-authoring location");
    assert!(
        flow_dir.starts_with(workspace.join(".openhuman").join("builtin-flow-skills")),
        "the dynamic builtin uses the separate product root: {}",
        flow_dir.display()
    );
    assert!(
        !flow_dir.starts_with(workspace.join(".openhuman").join("builtin-skills")),
        "TinyFlows resources must not share the static desktop-control root"
    );
    for resource in &flow.resources {
        let body = openhuman_core::skills::read_workflow_resource_for(
            &ctx,
            &config,
            "flow-authoring",
            resource,
        )
        .unwrap_or_else(|error| {
            panic!("reading published flow-authoring resource {resource:?}: {error}")
        });
        assert!(!body.is_empty(), "published resource {resource:?} is empty");
    }

    let disabled_config_path = temp.path().join("disabled-config.toml");
    std::fs::write(
        &disabled_config_path,
        "api_url = \"http://127.0.0.1:1\"\n\n[components]\nskills = true\nflows = true\ntinyflows = true\n\n[modules]\nenabled = false\n",
    )
    .expect("disabled components config");
    let disabled_config = openhuman_core::config::Config::load_from_config_path(
        &disabled_config_path,
        &workspace,
    )
    .await
    .expect("load disabled profile config");
    let disabled = openhuman_core::skills::load_workflow_metadata_for(&ctx, &disabled_config);
    assert!(
        !disabled.iter().any(|skill| skill.id() == "flow-authoring"),
        "a profile with modules disabled cannot see the shared workspace builtin"
    );
    let mut disabled_options =
        openhuman_core::agent::debug::DumpPromptOptions::new("orchestrator");
    disabled_options.workspace_dir_override = Some(workspace.clone());
    disabled_options.config_path_override = Some(disabled_config_path);
    let ordinary_prompt = openhuman_core::agent::debug::dump_agent_prompt(disabled_options)
        .await
        .expect("ordinary prompt construction continues when modules are disabled");
    assert!(ordinary_prompt.text.contains("## Workspace"));

    // Re-enable the same profile after the disabled view and ensure the
    // async host preflight restores the module-backed catalogue.
    let mut enabled_options =
        openhuman_core::agent::debug::DumpPromptOptions::new("workflow_builder");
    enabled_options.workspace_dir_override = Some(workspace.clone());
    enabled_options.config_path_override = Some(config_path.clone());
    openhuman_core::agent::debug::dump_agent_prompt(enabled_options)
        .await
        .expect("enabled profile prompt after disabled profile");
    let reenabled = openhuman_core::skills::load_workflow_metadata_for(&ctx, &config);
    assert!(
        reenabled.iter().any(|skill| skill.id() == "flow-authoring"),
        "re-enabling TinyFlows restores the module-backed builtin"
    );

    // A live disk edit must invalidate both the cached metadata and the
    // resource-read path; cached Builtin metadata cannot bless changed bytes.
    let resource = flow.resources.first().expect("published reference files");
    std::fs::write(flow_dir.join(resource), "tampered resource").expect("tamper resource");
    let after_tamper = openhuman_core::skills::load_workflow_metadata_for(&ctx, &config);
    assert!(
        !after_tamper
            .iter()
            .any(|skill| skill.id() == "flow-authoring"),
        "a cache hit must not advertise tampered module resources"
    );
    assert!(
        openhuman_core::skills::read_workflow_resource_for(
            &ctx,
            &config,
            "flow-authoring",
            resource,
        )
        .is_err(),
        "the resource reader must refuse tampered module bytes"
    );
}
