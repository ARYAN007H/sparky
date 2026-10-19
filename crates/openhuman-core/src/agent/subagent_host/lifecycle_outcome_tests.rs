use super::*;

#[test]
fn classified_stop_extension_round_trips_without_the_private_summary() {
    let cause = tinyagents_harness::no_progress::ClassifiedStopCause {
        class: "uncertain_side_effect".into(),
        operation: "web_answer_tool".into(),
        summary: "private URL https://example.com/private?q=secret".into(),
    };
    let extension = encode_classified_stop_cause(&cause);
    let mut neutral = SubagentOutcome::cancelled("task");
    neutral
        .extensions
        .insert(CLASSIFIED_STOP_CAUSE_EXTENSION.into(), extension);

    let serialized = serde_json::to_string(&neutral).expect("neutral outcome serializes");
    assert!(!serialized.contains("example.com"));
    assert!(!serialized.contains("private URL"));
    let reloaded: SubagentOutcome =
        serde_json::from_str(&serialized).expect("neutral outcome reloads");
    let reloaded_extension = reloaded
        .extensions
        .get(CLASSIFIED_STOP_CAUSE_EXTENSION)
        .expect("namespaced stop cause extension");
    let restored = decode_classified_stop_cause(reloaded_extension, "persisted reason")
        .expect("typed cause reloads");

    assert_eq!(restored.class, "uncertain_side_effect");
    assert_eq!(restored.operation, "web_answer_tool");
    assert_eq!(restored.summary, "persisted reason");
}

#[test]
fn host_incomplete_status_round_trips_through_neutral_persistence() {
    let cause = tinyagents_harness::no_progress::ClassifiedStopCause {
        class: "uncertain_side_effect".into(),
        operation: "web_answer_tool".into(),
        summary: "private URL https://example.com/private?q=secret".into(),
    };
    let host = SubagentRunOutcome {
        task_id: "task".into(),
        agent_id: "researcher".into(),
        output: "partial".into(),
        iterations: 2,
        elapsed: std::time::Duration::from_millis(10),
        mode: crate::agent::subagent_host::SubagentMode::Typed,
        status: SubagentRunStatus::Incomplete {
            reason: "stopped before finishing; reconcile external state".into(),
            failure_cause: Some(cause),
        },
        final_history: Vec::new(),
        usage: SubagentUsage::default(),
        artifact_paths: Vec::new(),
        persistence_disposition:
            tinyagents_orchestration::subagent::SubagentPersistenceDisposition::TerminalInserted,
    };
    let definition = crate::agent::harness::host_definition::AgentDefinition {
        id: "researcher".into(),
        when_to_use: "research".into(),
        display_name: None,
        system_prompt: tinyagents_definition::PromptSource::Inline("prompt".into()),
        model: tinyagents_definition::ModelSpec::Inherit,
        temperature: 0.0,
        tools: tinyagents_definition::ToolScope::Named(Vec::new()),
        disallowed_tools: Vec::new(),
        skill_filter: None,
        extra_tools: Vec::new(),
        deferred_tools: Vec::new(),
        tool_rules: None,
        max_iterations: 2,
        iteration_policy: Default::default(),
        max_result_chars: None,
        max_turn_output_tokens: None,
        timeout_secs: None,
        sandbox_mode: tinyagents_definition::SandboxMode::None,
        background: false,
        subagents: Vec::new(),
        delegate_name: None,
        agent_tier: tinyagents_definition::AgentTier::Worker,
        source: tinyagents_definition::DefinitionSource::Builtin,
        extensions: crate::agent::harness::host_definition::AgentExt {
            omit_identity: true,
            omit_memory_context: true,
            omit_safety_preamble: true,
            tokenjuice_compression: tinyjuice_bus::types::AgentTokenjuiceCompression::Auto,
            searches_connected_mcp: false,
            ..Default::default()
        },
        role: None,
    };
    let neutral = host_outcome_to_neutral(host, &definition, &SubagentRunOptions::default());
    let serialized = serde_json::to_string(&neutral).expect("neutral outcome serializes");
    assert!(!serialized.contains("example.com"));
    let reloaded: SubagentOutcome =
        serde_json::from_str(&serialized).expect("neutral outcome reloads");
    let SubagentOutcomeKind::Incomplete(incomplete) = reloaded.status else {
        panic!("incomplete status survives neutral persistence");
    };
    let restored = incomplete_run_status(incomplete.reason, &reloaded.extensions);
    let SubagentRunStatus::Incomplete {
        reason,
        failure_cause: Some(cause),
    } = restored
    else {
        panic!("typed stop cause survives neutral persistence");
    };

    assert_eq!(reason, "stopped before finishing; reconcile external state");
    assert_eq!(cause.class, "uncertain_side_effect");
    assert_eq!(cause.operation, "web_answer_tool");
    assert_eq!(cause.summary, reason);
}
