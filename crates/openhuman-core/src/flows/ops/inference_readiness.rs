use super::*;

/// Resolve the effective inference workload for an agent node. This remains
/// beside the graph adapter because it depends on TinyFlows node configuration.
pub(super) fn agent_node_role(config: &Config, node: &tinyflows::model::Node) -> &'static str {
    if let Some(agent_ref) = node
        .config
        .get("agent_ref")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.starts_with('='))
    {
        use crate::agent::harness::host_definition::AgentDefinitionRegistry;
        if let Err(error) = AgentDefinitionRegistry::init_global(&config.workspace_dir) {
            tracing::debug!(target: "flows", %error,
                "[flows] readiness: agent definition registry unavailable; using registry fallback");
        }
        let definition = AgentDefinitionRegistry::current().and_then(|r| r.get(agent_ref).cloned());
        let custom = crate::agent::registry::find_custom_in_config(config, agent_ref);
        if definition.is_some() || custom.is_some() {
            let entry_model = if definition.is_some() {
                None
            } else {
                custom.as_ref().and_then(|entry| entry.model.as_deref())
            };
            let override_model = crate::flows::tinyflows::caps::resolve_node_model(
                &node.config,
                entry_model,
            )
            .map(|model| crate::flows::tinyflows::caps::harness_model_default_override(&model));
            return crate::agent::session_host::provider_role_for_definition(
                override_model.as_deref().or(config.default_model.as_deref()),
                definition.as_ref(),
            );
        }
    }
    let pinned_model = node
        .config
        .get("model")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if let Some(model) = pinned_model {
        return crate::inference::provider::role_for_model_tier(model);
    }
    "summarization"
}

fn format_evaluation(evaluation: crate::inference::readiness::ReadinessEvaluation) -> Option<String> {
    if evaluation.status == "ready" {
        return None;
    }
    if let [failure] = evaluation.failures.as_slice() {
        if let [node_id] = failure.node_ids.as_slice() {
            return Some(if evaluation.status == "provider_not_configured" {
                format!(
                    "This flow's agent step needs a working AI provider, but the provider \
                     returned: '{}'. Configure your provider API key in OpenHuman Settings > \
                     Providers, then try again.",
                    failure.message
                )
            } else {
                format!("This flow's agent step needs a working AI provider: {}", failure.message)
            });
        }
    }
    let details = evaluation
        .failures
        .iter()
        .map(|failure| {
            let nodes = failure
                .node_ids
                .iter()
                .map(|id| format!("'{id}'"))
                .collect::<Vec<_>>()
                .join(", ");
            if evaluation.status == "provider_not_configured"
                && failure.message.to_ascii_lowercase().contains("api key not configured")
            {
                format!(
                    "Node(s) {nodes} (role `{}`): the provider returned: '{}'. Configure \
                     your provider API key in OpenHuman Settings > Providers, then try again.",
                    failure.role, failure.message
                )
            } else {
                format!("Node(s) {nodes} (role `{}`): {}", failure.role, failure.message)
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    Some(format!(
        "This flow has {} agent step(s) that need a working AI provider:\n\n{details}",
        evaluation.failures.len()
    ))
}

/// Shared readiness evaluation for the advisory proposal status and hard run gate.
pub(super) async fn evaluate_inference_readiness(
    ctx: &crate::core::runtime::DispatchCtx,
    config: &Config,
    graph: &WorkflowGraph,
) -> Option<crate::inference::readiness::ReadinessEvaluation> {
    let roles: Vec<_> = graph
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::Agent)
        .map(|node| (node.id.clone(), agent_node_role(config, node)))
        .collect();
    crate::inference::readiness::evaluate(ctx, config, &roles).await
}

/// Run-time hard gate, returned in the flows domain's established error shape.
pub(crate) async fn validate_inference_readiness(
    ctx: &crate::core::runtime::DispatchCtx,
    config: &Config,
    graph: &WorkflowGraph,
) -> Vec<String> {
    let Some(evaluation) = evaluate_inference_readiness(ctx, config, graph).await else {
        return Vec::new();
    };
    let Some(message) = format_evaluation(evaluation.clone()) else {
        return Vec::new();
    };
    if evaluation.failures.len() == 1 && evaluation.failures[0].node_ids.len() == 1 {
        vec![format!("Node '{}': {message}", evaluation.failures[0].node_ids[0])]
    } else {
        vec![message]
    }
}
