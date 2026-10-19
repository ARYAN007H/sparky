//! Scope-bound callbacks used by TinyFlows authoring and catalog operations.

use std::future::Future;
use std::sync::Arc;

use serde_json::Value;
use tinybus::Result as BusResult;
use tinyflows_bus::catalog::{self, ClassifyManifestCandidatesRequest};
use tinyflows_bus::{AuthoringHost, ToolContract};

use crate::config::Config;
use crate::core::runtime::{CoreContext, DispatchCtx};
use crate::flows::ops;
use crate::modules::tinyflows as module;

#[derive(Clone)]
struct Callbacks {
    config: Arc<Config>,
    context: DispatchCtx,
    runtime: Option<tokio::runtime::Handle>,
}

impl Callbacks {
    async fn in_context<T, F>(&self, work: F) -> Result<T, String>
    where
        T: Send + 'static,
        F: Future<Output = Result<T, String>> + Send + 'static,
    {
        let context = self.context.clone();
        let work = async move {
            if let Some(core) = context.core().cloned() {
                CoreContext::scope(core, work).await
            } else {
                work.await
            }
        };
        match &self.runtime {
            Some(runtime) => runtime
                .spawn(work)
                .await
                .map_err(|error| format!("authoring host callback did not finish: {error}"))?,
            None => work.await,
        }
    }
}

#[tinybus::interface(name = "ai.tinyhumans.tinyflows.AuthoringHost")]
impl Callbacks {
    async fn resolve_workflow(&self, workflow_id: String) -> BusResult<Option<Value>> {
        let config = Arc::clone(&self.config);
        self.in_context(async move {
            ops::load_flow_graph(&config, &workflow_id)
                .map(|graph| graph.map(|graph| serde_json::to_value(graph).expect("graph serializes")))
        })
        .await
        .map_err(|error| tinybus::Error::failed(error))
    }

    async fn check_gate(&self, stage: String, graph: Value) -> BusResult<Vec<String>> {
        let config = Arc::clone(&self.config);
        let context = self.context.clone();
        self.in_context(async move {
            let graph: tinyflows::model::WorkflowGraph =
                serde_json::from_value(graph).map_err(|error| error.to_string())?;
            match stage.as_str() {
                tinyflows_bus::host::TOOL_POLICY => Ok(ops::validate_upload_paths(&graph)),
                tinyflows_bus::host::AGENT_REFERENCES => {
                    Ok(ops::validate_agent_refs(&context, &config, &graph).await)
                }
                tinyflows_bus::host::CONNECTIONS => {
                    Ok(ops::validate_connection_refs(&context, &config, &graph).await)
                }
                tinyflows_bus::host::TOOL_CONTRACTS => {
                    Ok(ops::validate_tool_contracts(&config, &graph).await)
                }
                _ => Err(format!("unknown TinyFlows authoring gate `{stage}`")),
            }
        })
        .await
        .map_err(|error| tinybus::Error::failed(error))
    }

    async fn lookup_tool_contract(&self, slug: String) -> BusResult<Option<ToolContract>> {
        let config = Arc::clone(&self.config);
        self.in_context(async move {
            let Ok((contract, _)) = crate::integrations::composio::catalog::lookup_contract(
                &config, &slug,
            )
            .await
            else {
                return Ok(None);
            };
            Ok(Some(ToolContract {
                output_fields: contract.output_fields,
                output_schema: contract.output_schema,
                primary_array_path: contract.primary_array_path,
                envelope_prefix: Some("data".into()),
                metadata_fields: vec![
                    "successful".into(),
                    "error".into(),
                    "costUsd".into(),
                    "markdownFormatted".into(),
                ],
            }))
        })
        .await
        .map_err(|error| tinybus::Error::failed(error))
    }
}

#[tinybus::interface(name = "ai.tinyhumans.tinyflows.ApprovalManifestHost")]
impl Callbacks {
    async fn classify_manifest_candidates(
        &self,
        request: ClassifyManifestCandidatesRequest,
    ) -> BusResult<catalog::ClassifyManifestCandidatesResponse> {
        let config = Arc::clone(&self.config);
        self.in_context(async move {
            use crate::flows::tinyflows::caps::classify_composio_action_for_tier;
            use crate::security::{GateDecision, SecurityPolicy};
            use tinybox_bus::CommandClass;
            use tinyflows_bus::catalog::{ManifestCandidate, ManifestClass};

            let security = SecurityPolicy::from_config(
                &config.autonomy,
                &config.workspace_dir,
                &config.action_dir,
            );
            let gated = |class: CommandClass| {
                Some(ManifestClass {
                    name: format!("{class:?}"),
                    blocked: security.gate_decision(class) == GateDecision::Block,
                    skip: false,
                })
            };
            let mut classes = Vec::with_capacity(request.candidates.len());
            for candidate in request.candidates {
                let class = match candidate {
                    ManifestCandidate::Http { .. } => gated(CommandClass::Network),
                    ManifestCandidate::Code { .. } => gated(CommandClass::Write),
                    ManifestCandidate::NativeTool { tool_name, args, .. } => {
                        let class = crate::tools::native_ops::classify_tool_call(
                            &config,
                            &tool_name,
                            &args,
                        )
                        .unwrap_or(CommandClass::Network);
                        gated(class)
                    }
                    ManifestCandidate::ConnectorTool { slug, .. } => {
                        let class = classify_composio_action_for_tier(&slug).await;
                        Some(ManifestClass {
                            name: format!("{class:?}"),
                            blocked: security.gate_decision(class) == GateDecision::Block,
                            skip: class == CommandClass::Read,
                        })
                    }
                    ManifestCandidate::DynamicTool { .. } | ManifestCandidate::Agent { .. } => None,
                };
                classes.push(class);
            }
            Ok(classes)
        })
        .await
        .map_err(|error| tinybus::Error::failed(error))
    }
}

/// A callback registration whose lifetime is bounded by one authoring call.
pub(crate) struct Registration {
    _registration: module::ServedObjectRegistration,
    pub(crate) authoring: AuthoringHost,
    pub(crate) catalog: catalog::CallbackAddress,
}

pub(crate) async fn register(
    config: Arc<Config>,
    context: DispatchCtx,
) -> Result<Registration, String> {
    let path = format!("/ai/tinyhumans/openhuman/AuthoringHost/h{}", uuid::Uuid::new_v4().simple());
    let callbacks = Callbacks {
        config,
        context,
        runtime: tokio::runtime::Handle::try_current().ok(),
    };
    let registration = module::serve_pair(path.clone(), callbacks.clone(), callbacks).await?;
    Ok(Registration {
        _registration: registration,
        authoring: AuthoringHost {
            bus_name: module::HOST_BUS_NAME.to_string(),
            object_path: path.clone(),
        },
        catalog: catalog::CallbackAddress {
            destination: module::HOST_BUS_NAME.to_string(),
            object_path: path,
        },
    })
}
