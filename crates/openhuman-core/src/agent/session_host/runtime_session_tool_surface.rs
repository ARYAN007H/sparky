//! Refresh the executable, declared and policy tool surfaces at a turn boundary.

use super::*;

impl OpenHumanSessionHost {
    /// Publish a caller-driven visibility/policy mutation into the live hook
    /// state. The runtime keeps generic snapshots immutable per request; this
    /// simply replaces the host composition from which the *next* snapshot,
    /// execution source, and enforcement object are built.
    pub(in crate::agent::session_host) fn sync_runtime_tool_surface(&self) {
        self.sync_runtime_tool_surface_with_auto_include(None);
    }

    /// Publish a visibility mutation and record whether future dynamically
    /// connected delegates may enter the surface without an explicit allow.
    pub(in crate::agent::session_host) fn sync_runtime_tool_surface_with_auto_include(
        &self,
        auto_include_new_synthesized_tools: Option<bool>,
    ) {
        let prelude = self
            .runtime_state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .prelude
            .clone();
        let Some(prelude) = prelude else {
            return;
        };
        let prior_auto_include = prelude
            .tool_surface
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .auto_include_new_synthesized_tools;
        prelude.replace_tool_surface(OpenHumanTurnToolSurface {
            tools: self.tools.clone(),
            synthesized_tools: self.synthesized_tools.clone(),
            tool_specs: self.tool_specs.clone(),
            durable_tool_specs: self.durable_tool_specs.clone(),
            visible_tool_specs: self.visible_tool_specs.clone(),
            visible_tool_names: self.visible_tool_names.clone(),
            deferred_tool_names: self.deferred_tool_names.clone(),
            permanent_tool_names: self.permanent_tool_names.clone(),
            discovery_enabled: self.discovery_enabled,
            requested_deferred_tools: self.requested_deferred_tools.clone(),
            auto_include_new_synthesized_tools: auto_include_new_synthesized_tools
                .unwrap_or(prior_auto_include),
            synthesized_tool_names: self.synthesized_tool_names.clone(),
            tool_policy_session: self.tool_policy_session.clone(),
            event_session_id: self.event_session_id.clone(),
            event_channel: self.event_channel.clone(),
            agent_definition_name: self.agent_definition_name.clone(),
        });
    }
}

/// Reconcile visible delegate names without reopening a caller's explicit
/// restriction. Revoked names disappear; a newly connected name is admitted
/// only for the wildcard belt.
pub(in crate::agent::session_host) fn reconcile_synthesized_visibility(
    visible: &mut std::collections::HashSet<String>,
    previous: &std::collections::HashSet<String>,
    next: &std::collections::HashSet<String>,
    auto_include_new: bool,
) {
    for name in previous.difference(next) {
        visible.remove(name);
    }
    if auto_include_new {
        visible.extend(next.iter().cloned());
    }
}

impl OpenHumanTurnPrelude {
    /// Rebuild every delegation-dependent tool view from the current cached
    /// integration set, replacing rather than appending. A revoked delegate is
    /// removed from the executable source, schema, and policy together before
    /// this request is prepared.
    pub(super) fn refresh_delegation_tool_surface(&self) -> anyhow::Result<()> {
        use crate::agent::harness::host_definition::AgentDefinitionRegistry;
        use crate::tools::orchestrator_tools::collect_orchestrator_tools;
        use tinytools::ToolPolicyEngine;

        let Some(registry) = AgentDefinitionRegistry::current() else {
            return Ok(());
        };
        let definition = self.session_definition(&registry);
        let has_recorded_tool_snapshot = self
            .mutable
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .recorded_tool_snapshot_adopted;
        #[cfg(feature = "mcp")]
        let has_configured_mcp_tools = !self
            .mutable
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .mcp_tools
            .configured
            .is_empty();
        #[cfg(not(feature = "mcp"))]
        let has_configured_mcp_tools = false;
        if definition
            .as_ref()
            .is_none_or(|definition| definition.subagents.is_empty())
            && !has_recorded_tool_snapshot
            && !has_configured_mcp_tools
        {
            return Ok(());
        }
        let (integrations, integrations_are_authoritative, recorded_tool_snapshot_adopted) = {
            let mutable = self
                .mutable
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            (
                mutable.connected_integrations.clone(),
                mutable.connected_integrations_authoritative,
                mutable.recorded_tool_snapshot_adopted,
            )
        };
        #[cfg(feature = "mcp")]
        let mcp_tools = self.collect_mcp_search_tools();
        #[cfg(feature = "mcp")]
        let configured_mcp_tools = self.collect_configured_mcp_tools();
        let mut surface = self
            .tool_surface
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut collected = definition
            .as_ref()
            .map(|definition| collect_orchestrator_tools(definition, &registry, &integrations))
            .unwrap_or_default();
        #[cfg(feature = "mcp")]
        collected.extend(mcp_tools);
        #[cfg(feature = "mcp")]
        collected.extend(configured_mcp_tools);
        let rebuilt = definition.as_ref().map_or_else(Vec::new, |definition| {
            self.rebuilt_recorded_tools(
                definition,
                &surface.tools,
                &collected,
                &integrations,
                integrations_are_authoritative,
            )
        });
        collected.extend(rebuilt);
        tinyagents_runtime::reject_synthesized_collisions(
            &surface.permanent_tool_names,
            &collected,
        )
        .map_err(anyhow::Error::msg)?;
        let synthesized =
            super::super::builder::drop_synthesized_name_collisions(&surface.tools, collected);
        let synthesized_names = synthesized
            .iter()
            .map(|tool| tool.name().to_string())
            .collect::<std::collections::HashSet<_>>();
        let previous_synthesized = std::mem::replace(
            &mut surface.synthesized_tool_names,
            synthesized_names.clone(),
        );
        let auto_include_new_synthesized_tools = surface.auto_include_new_synthesized_tools;
        reconcile_synthesized_visibility(
            &mut surface.visible_tool_names,
            &previous_synthesized,
            &synthesized_names,
            auto_include_new_synthesized_tools,
        );
        if recorded_tool_snapshot_adopted {
            surface.visible_tool_names.extend(
                self.mutable
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .recorded_repl_tools
                    .iter()
                    .map(|spec| spec.name.clone()),
            );
        }
        // Preserve permanently attached tools when re-deriving the surface.
        permanent::refresh_visibility(&mut surface, &synthesized);

        let mut specs = surface
            .durable_tool_specs
            .iter()
            .cloned()
            .chain(synthesized.iter().map(|tool| Arc::new(tool.spec())))
            .collect::<Vec<_>>();
        {
            let recorded = self
                .mutable
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .recorded_repl_tools
                .clone();
            let recorded_names = recorded
                .iter()
                .map(|spec| spec.name.as_str())
                .collect::<std::collections::HashSet<_>>();
            if recorded_tool_snapshot_adopted {
                specs.retain(|spec| {
                    !crate::inference::tokenjuice::is_repl_tool(&spec.name)
                        || recorded_names.contains(spec.name.as_str())
                });
                surface.visible_tool_names.retain(|name| {
                    !crate::inference::tokenjuice::is_repl_tool(name)
                        || recorded_names.contains(name.as_str())
                });
            }
            let recorded = recorded
                .iter()
                .map(|spec| (spec.name.as_str(), spec))
                .collect::<std::collections::HashMap<_, _>>();
            for spec in &mut specs {
                if let Some(frozen) = recorded.get(spec.name.as_str()) {
                    *spec = Arc::new((*frozen).clone());
                }
            }
        }
        #[cfg(feature = "mcp")]
        {
            let recorded = self
                .mutable
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .mcp_tools
                .recorded_specs
                .clone();
            let recorded = recorded
                .iter()
                .map(|spec| (spec.name.as_str(), spec))
                .collect::<std::collections::HashMap<_, _>>();
            // A resumed thread keeps the exact MCP declaration it was shown,
            // while the current module presentation still supplies its live
            // executor. This mirrors the REPL snapshot restoration above.
            for spec in &mut specs {
                if let Some(frozen) = recorded.get(spec.name.as_str()) {
                    *spec = Arc::new((*frozen).clone());
                }
            }
        }
        let synthesized_tools = Arc::new(synthesized);
        crate::tools::toolpacks::bind_synthesized_pack_registry(&surface.tools, &synthesized_tools);
        let all_tools = surface
            .tools
            .iter()
            .chain(synthesized_tools.iter())
            .map(|tool| tool.as_ref())
            .collect::<Vec<_>>();
        // Advertised plus deferred, like the session host: a deferred tool
        // outside the set would be `HideFromPrompt`, which the direct-call
        // gate refuses.
        let reachable: std::collections::HashSet<String> = if surface.visible_tool_names.is_empty()
        {
            std::collections::HashSet::new()
        } else {
            surface
                .visible_tool_names
                .iter()
                .chain(surface.deferred_tool_names.iter())
                .cloned()
                .collect()
        };
        let mut policy = ToolPolicyEngine::build_session_from_refs(
            &surface.agent_definition_name,
            &surface.event_channel,
            "session",
            &self.config.channel_permissions,
            &all_tools,
            &reachable,
        );
        crate::tools::toolpacks::close_handed_off_packs(
            &mut policy,
            &surface.agent_definition_name,
            &all_tools,
        );
        let visible = super::super::builder::dedup_visible_tool_specs(
            super::super::builder::visible_tool_specs_for_policy(
                &specs,
                &surface.visible_tool_names,
                &policy,
            ),
        );
        surface.tool_specs = Arc::new(specs);
        surface.synthesized_tools = synthesized_tools;
        surface.visible_tool_specs = Arc::new(visible);
        surface.tool_policy_session = policy;
        Ok(())
    }
}
