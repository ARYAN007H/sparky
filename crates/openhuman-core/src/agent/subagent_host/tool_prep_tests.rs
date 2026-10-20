use super::*;

#[test]
fn custom_delegate_is_treated_as_spawn_tool() {
    assert!(is_subagent_spawn_tool("spawn_subagent"));
    assert!(is_subagent_spawn_tool("delegate_researcher"));
    assert!(!is_subagent_spawn_tool("directory_resolve"));
}

#[test]
fn child_tool_surface_never_contains_any_spawn_or_continuation_tool() {
    let spawn_names = tinyagents_definition::SPAWN_TOOL_NAMES
        .iter()
        .copied()
        .chain(["delegate_researcher", "continue_subagent"])
        .collect::<Vec<_>>();
    for name in &spawn_names {
        assert!(
            crate::agent::tinyagents::is_subagent_spawn_or_delegate_tool(name),
            "shared spawn predicate missed {name}"
        );
    }
    let tools: Vec<Box<dyn Tool>> = spawn_names
        .into_iter()
        .map(|name| Box::new(NamedTool(name)) as Box<dyn Tool>)
        .collect();

    let indices = filter_tool_indices(&tools, &ToolScope::Wildcard, &[], None);

    assert!(
        indices.is_empty(),
        "child tool surface retained: {:?}",
        indices
            .iter()
            .map(|&index| tools[index].name())
            .collect::<Vec<_>>()
    );
}

#[test]
fn unprefixed_delegate_name_overrides_are_treated_as_spawn_tools() {
    // Most synthesised delegation tools use an unprefixed
    // `delegate_name` override (`plan`, `manage_tasks`, `create_image`, …).
    // They must be stripped from every sub-agent surface, exactly like
    // the `delegate_*`-prefixed defaults.
    let mut registry =
        crate::agent_registry::host_definition::AgentDefinitionRegistry::builtins_only();
    let mut plan_alias = registry
        .get("orchestrator")
        .expect("built-in orchestrator")
        .clone();
    plan_alias.id = "test_plan_alias".to_string();
    plan_alias.delegate_name = Some("plan".to_string());
    registry.insert(plan_alias);
    for delegate in [
        "plan",
        "manage_tasks",
        "create_image",
        #[cfg(feature = "flows")]
        "build_workflow",
        #[cfg(feature = "flows")]
        "discover_workflows",
        // `make_presentation` is `presentation_agent`'s `delegate_name`; the agent —
        // and therefore this delegate tool — is compiled out with the
        // `documents` feature.
        #[cfg(feature = "documents")]
        "make_presentation",
    ] {
        assert!(
            super::is_registered_delegate_alias(delegate, Some(&registry)),
            "`{delegate}` is a synthesised delegation tool and must be \
             stripped from sub-agent tool surfaces"
        );
    }
    // Ordinary worker tools stay visible.
    for plain in ["shell", "file_read", "web_fetch", "todo"] {
        assert!(
            !super::is_registered_delegate_alias(plain, Some(&registry)),
            "`{plain}` must not be classified as a spawn tool"
        );
    }
}

#[test]
fn child_keeps_the_parents_protocol() {
    use tinytools_agent::dialect::ToolCallFormat;

    for parent in [
        ToolCallFormat::Native,
        ToolCallFormat::PFormat,
        ToolCallFormat::Json,
    ] {
        let (format, _) = subagent_prompt_protocol(parent, &[]);
        assert_eq!(format, parent);
    }
}

fn dynamic_subagent_prompt(
    ctx: &crate::agent::prompts::PromptContext<'_>,
) -> anyhow::Result<String> {
    crate::prompts::SystemPromptBuilder::for_subagent("child prompt".into(), true, true).build(ctx)
}

#[test]
fn dynamic_child_prompt_lists_text_dialect_catalogue_once() {
    use crate::agent::prompts::{PromptContext, PromptTool};
    use tinyagents_definition::PromptSource;
    use tinytools_agent::dialect::ToolCallFormat;

    let workspace = tempfile::TempDir::new().unwrap();
    let schema = r#"{"type":"object","properties":{"path":{"type":"string"}}}"#;
    let tools = [PromptTool {
        name: std::borrow::Cow::Borrowed("unique_probe_tool"),
        description: std::borrow::Cow::Borrowed("catalogue sentinel for one visible probe"),
        parameters_schema: Some(schema.to_owned()),
    }];
    let workflows = [];
    let integrations = [];
    let visible = ["unique_probe_tool".to_owned()].into_iter().collect();

    let formats = [
        ToolCallFormat::PFormat,
        ToolCallFormat::Python,
        ToolCallFormat::TypeScript,
    ];
    let mut catalogue_counts = Vec::with_capacity(formats.len());
    for format in formats {
        let specs = [tinytools::ToolSpec {
            name: "unique_probe_tool".into(),
            description: "catalogue sentinel for one visible probe".into(),
            parameters: serde_json::from_str(schema).unwrap(),
        }];
        let (_, dispatcher_instructions) = subagent_prompt_protocol(format, &specs);
        let context = PromptContext {
            workspace_dir: workspace.path(),
            model_name: "test-model",
            agent_id: "dynamic-child",
            tools: &tools,
            workflows: &workflows,
            dispatcher_instructions: &dispatcher_instructions,
            visible_tool_names: &visible,
            tool_call_format: format,
            connected_integrations: &integrations,
            connected_identities_md: String::new(),
            user_identity: None,
            personality_roster: vec![],
            agents_md_global: None,
            agents_md_local: None,
        };
        let definition = AgentDefinition {
            id: "dynamic-child".into(),
            system_prompt: PromptSource::Dynamic(()),
            extensions: crate::agent_registry::host_definition::AgentExt {
                prompt_builder: Some(dynamic_subagent_prompt),
                ..Default::default()
            },
            ..AgentDefinition::default()
        };

        let prompt = load_prompt_source(&definition, &context).unwrap();
        catalogue_counts.push(
            prompt
                .matches("catalogue sentinel for one visible probe")
                .count(),
        );
    }
    assert_eq!(
        catalogue_counts,
        [1, 1, 1],
        "dynamic PFormat/Python/TypeScript prompts should each list the tool catalogue once"
    );
}

struct NamedTool(&'static str);

#[async_trait::async_trait]
impl Tool for NamedTool {
    fn name(&self) -> &str {
        self.0
    }
    fn description(&self) -> &str {
        "named"
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({ "type": "object" })
    }
    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<tinytools::ToolResult> {
        Ok(tinytools::ToolResult::success("ok"))
    }
}

#[test]
fn rule_withheld_tools_leave_the_child_prompt_but_stay_allowed() {
    let tools: Vec<Box<dyn Tool>> = vec![
        Box::new(NamedTool("file_read")),
        Box::new(NamedTool("shell")),
        Box::new(NamedTool("web_fetch")),
    ];
    let rules: tinytools::ToolRules = serde_json::from_value(serde_json::json!({ "rules": [
        { "effect": "hide", "match": { "name": "shell" } },
        { "effect": "deny", "match": { "name": "web_*" } },
    ] }))
    .unwrap();
    let policy = tinyagents_harness::tool::ToolRulePolicy::new(rules);
    let allowed = vec![0, 1, 2];

    let listed = rule_listed_indices(&allowed, &tools, Some(&policy));
    assert_eq!(listed, vec![0], "hidden and denied tools leave the listing");
    let prompt = prompt_tools_for(&listed, &tools);
    assert_eq!(prompt.len(), 1);
    assert_eq!(prompt[0].name, "file_read");
    assert_eq!(rule_listed_indices(&allowed, &tools, None), allowed);
}
