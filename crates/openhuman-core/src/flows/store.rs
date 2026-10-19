//! Host client for the TinyFlows module's Catalog contract.
//!
//! The module owns catalog schemas, migrations, revisions, and update rules.
//! This file only translates the existing core-facing flow types to Catalog
//! requests and back. Run lifecycle and checkpoint records are owned by the
//! Runs adapter, not this definition client.

use anyhow::{Result, anyhow};
use serde::{Serialize, de::DeserializeOwned};
use std::path::PathBuf;
use tinyflows_bus::catalog::{
    self, DeleteWorkflowResult, FlowUpdateError as BusUpdateError, GetRevisionRequest,
    ListRevisionsRequest, ListWorkflowsResult, PutWorkflowRequest, PutWorkflowResponse,
    SetEnabledRequest, Workflow, WorkflowRevision,
};
use tinyflows_catalog::{
    Flow, FlowRevision, FlowSuggestion, SuggestionStatus,
    store::FlowUpdateError,
};

use crate::config::Config;
use crate::core::runtime::DispatchCtx;
use crate::modules::tinyflows as module;

/// Retained only for callers that display the legacy workflow folder path.
pub fn dir(config: &Config) -> PathBuf {
    config.workspace_dir.join("flows")
}

fn call_catalog<T, A>(config: &Config, member: &'static str, args: A) -> Result<T>
where
    T: DeserializeOwned + Send + 'static,
    A: Serialize + Send + 'static,
{
    let context = DispatchCtx::ambient();
    let config = config.clone();
    crate::storage::block_on_anyhow(async move {
        let _registration =
            super::catalog_storage::register(&context, &config)
                .await
                .map_err(anyhow::Error::msg)?;
        let proxy = module::catalog_proxy(&config)
            .await
            .map_err(anyhow::Error::msg)?;
        proxy
            .call(member, &args)
            .await
            .map_err(|error| anyhow!(error))
    })
}

fn flow_from_wire(workflow: Workflow) -> Result<Flow> {
    Ok(Flow {
        id: workflow.id,
        name: workflow.name,
        enabled: workflow.enabled,
        graph: serde_json::from_value(workflow.graph)
            .map_err(|error| anyhow!("decode workflow graph: {error}"))?,
        created_at: workflow.created_at,
        updated_at: workflow.updated_at,
        last_run_at: workflow.last_run_at,
        last_status: workflow.last_status,
        require_approval: workflow.require_approval,
    })
}

fn flow_to_wire(flow: &Flow) -> Result<Workflow> {
    Ok(Workflow {
        id: flow.id.clone(),
        name: flow.name.clone(),
        enabled: flow.enabled,
        graph: serde_json::to_value(&flow.graph)
            .map_err(|error| anyhow!("encode workflow graph: {error}"))?,
        created_at: flow.created_at.clone(),
        updated_at: flow.updated_at.clone(),
        last_run_at: flow.last_run_at.clone(),
        last_status: flow.last_status.clone(),
        require_approval: flow.require_approval,
    })
}

fn update_error_from_wire(error: BusUpdateError) -> FlowUpdateError {
    match error {
        BusUpdateError::NotFound { .. } => FlowUpdateError::NotFound,
        BusUpdateError::VersionConflict { current, .. } => match flow_from_wire(*current) {
            Ok(current) => FlowUpdateError::Conflict(Box::new(current)),
            Err(error) => FlowUpdateError::Store(error),
        },
        BusUpdateError::Store { message } => FlowUpdateError::Store(anyhow!(message)),
    }
}

fn put_result(response: PutWorkflowResponse) -> Result<Flow> {
    match response {
        PutWorkflowResponse::Ok(result) => flow_from_wire(result.workflow),
        PutWorkflowResponse::Error(error) => Err(anyhow!(match error {
            BusUpdateError::NotFound { message }
            | BusUpdateError::VersionConflict { message, .. }
            | BusUpdateError::Store { message } => message,
        })),
    }
}

/// Save a workflow through the module's Catalog contract.
pub fn upsert_flow(config: &Config, flow: &Flow) -> Result<()> {
    let storage = super::catalog_storage::register_blocking(config)?;
    let request = PutWorkflowRequest::Update {
        storage,
        id: flow.id.clone(),
        name: flow.name.clone(),
        graph: serde_json::to_value(&flow.graph)?,
        require_approval: flow.require_approval,
        expected_version: None,
        force_disarm_automatic: false,
    };
    let response: PutWorkflowResponse = call_catalog(config, catalog::members::PUT_WORKFLOW, (request,))?;
    put_result(response)?;
    Ok(())
}

/// Duplicate a workflow through the module's Catalog contract.
pub fn insert_duplicate_flow(config: &Config, source: &Flow, new_name: String) -> Result<Flow> {
    let storage = super::catalog_storage::register_blocking(config)?;
    let request = PutWorkflowRequest::Duplicate {
        storage,
        id: source.id.clone(),
        name: new_name,
    };
    let response: PutWorkflowResponse = call_catalog(config, catalog::members::PUT_WORKFLOW, (request,))?;
    put_result(response)
}

/// Create a workflow through the module's Catalog contract.
pub fn create_flow(
    config: &Config,
    name: String,
    graph: tinyflows::model::WorkflowGraph,
    require_approval: bool,
    _enabled: bool,
) -> Result<Flow> {
    let storage = super::catalog_storage::register_blocking(config)?;
    let request = PutWorkflowRequest::Create {
        storage,
        name,
        graph: serde_json::to_value(graph)?,
        require_approval,
    };
    let response: PutWorkflowResponse = call_catalog(config, catalog::members::PUT_WORKFLOW, (request,))?;
    put_result(response)
}

/// Read one workflow by id.
pub fn get_flow(config: &Config, id: &str) -> Result<Option<Flow>> {
    let storage = super::catalog_storage::register_blocking(config)?;
    let workflow: Option<Workflow> = call_catalog(
        config,
        catalog::members::GET_WORKFLOW,
        (storage, id.to_owned()),
    )?;
    workflow.map(flow_from_wire).transpose()
}

/// List workflows and their full graphs through the module's catalog client.
pub fn list_flows(config: &Config) -> Result<(Vec<Flow>, usize)> {
    let storage = super::catalog_storage::register_blocking(config)?;
    let listed: ListWorkflowsResult = call_catalog(
        config,
        catalog::members::LIST_WORKFLOWS,
        (storage.clone(),),
    )?;
    let mut flows = Vec::with_capacity(listed.workflows.len());
    let mut skipped = listed.skipped_rows;
    for summary in listed.workflows {
        let workflow: Option<Workflow> = call_catalog(
            config,
            catalog::members::GET_WORKFLOW,
            (storage.clone(), summary.id),
        )?;
        match workflow.map(flow_from_wire).transpose() {
            Ok(Some(flow)) => flows.push(flow),
            Ok(None) => skipped += 1,
            Err(error) => {
                tracing::warn!(%error, "[flows] catalog returned a workflow that could not be decoded");
                skipped += 1;
            }
        }
    }
    Ok((flows, skipped))
}

/// List enabled workflows through the same module-owned catalog surface.
pub fn list_enabled_flows(config: &Config) -> Result<(Vec<Flow>, usize)> {
    let (flows, skipped) = list_flows(config)?;
    Ok((flows.into_iter().filter(|flow| flow.enabled).collect(), skipped))
}

/// Remove a workflow through the module's Catalog contract.
pub fn remove_flow(config: &Config, id: &str) -> Result<()> {
    let storage = super::catalog_storage::register_blocking(config)?;
    let _: DeleteWorkflowResult = call_catalog(
        config,
        catalog::members::DELETE_WORKFLOW,
        (storage, id.to_owned()),
    )?;
    Ok(())
}

/// Enable or disable a workflow through the module's Catalog contract.
pub fn set_enabled(config: &Config, id: &str, enabled: bool) -> Result<Flow> {
    let storage = super::catalog_storage::register_blocking(config)?;
    let request = SetEnabledRequest {
        storage,
        id: id.to_owned(),
        enabled,
    };
    let workflow: Workflow = call_catalog(config, catalog::members::SET_ENABLED, (request,))?;
    flow_from_wire(workflow)
}

/// Update a workflow while preserving the module's optimistic concurrency.
#[allow(clippy::too_many_arguments)]
pub fn update_flow_graph(
    config: &Config,
    id: &str,
    name: String,
    graph: tinyflows::model::WorkflowGraph,
    require_approval: bool,
    _enabled_override: Option<bool>,
    force_disarm_if_automatic: bool,
    expected_updated_at: Option<&str>,
) -> std::result::Result<Flow, FlowUpdateError> {
    let storage = super::catalog_storage::register_blocking(config)
        .map_err(|error| FlowUpdateError::Store(error.into()))?;
    let request = PutWorkflowRequest::Update {
        storage,
        id: id.to_owned(),
        name,
        graph: serde_json::to_value(graph)
            .map_err(|error| FlowUpdateError::Store(anyhow!(error)))?,
        require_approval,
        expected_version: expected_updated_at.map(str::to_owned),
        force_disarm_automatic: force_disarm_if_automatic,
    };
    let response: PutWorkflowResponse = call_catalog(config, catalog::members::PUT_WORKFLOW, (request,))
        .map_err(FlowUpdateError::Store)?;
    match response {
        PutWorkflowResponse::Ok(result) => {
            flow_from_wire(result.workflow).map_err(FlowUpdateError::Store)
        }
        PutWorkflowResponse::Error(error) => Err(update_error_from_wire(error)),
    }
}

/// List a workflow's committed revisions.
pub fn list_revisions(config: &Config, flow_id: &str, limit: usize) -> Result<Vec<FlowRevision>> {
    let storage = super::catalog_storage::register_blocking(config)?;
    let request = ListRevisionsRequest {
        storage,
        flow_id: flow_id.to_owned(),
        limit,
    };
    let revisions: Vec<WorkflowRevision> = call_catalog(
        config,
        catalog::members::LIST_REVISIONS,
        (request,),
    )?;
    revisions
        .into_iter()
        .map(|revision| serde_json::from_value(serde_json::to_value(revision)?).map_err(Into::into))
        .collect()
}

/// Read one workflow revision by id.
pub fn revision_by_id(
    config: &Config,
    flow_id: &str,
    revision_id: &str,
) -> Result<Option<FlowRevision>> {
    let storage = super::catalog_storage::register_blocking(config)?;
    let request = GetRevisionRequest {
        storage,
        flow_id: flow_id.to_owned(),
        revision_id: revision_id.to_owned(),
    };
    let revision: Option<WorkflowRevision> = call_catalog(
        config,
        catalog::members::GET_REVISION,
        (request,),
    )?;
    revision
        .map(|revision| serde_json::from_value(serde_json::to_value(revision)?).map_err(Into::into))
        .transpose()
}

/// Upsert module-owned discovery suggestions.
pub fn upsert_suggestions(config: &Config, suggestions: &[FlowSuggestion]) -> Result<usize> {
    let storage = super::catalog_storage::register_blocking(config)?;
    let suggestions = suggestions
        .iter()
        .cloned()
        .map(|suggestion| serde_json::from_value(serde_json::to_value(suggestion).unwrap_or(Value::Null)))
        .collect::<std::result::Result<Vec<catalog::FlowSuggestion>, _>>()?;
    let request = catalog::UpsertSuggestionsRequest {
        storage,
        suggestions,
    };
    call_catalog(config, catalog::members::UPSERT_SUGGESTIONS, (request,))
}

/// List module-owned discovery suggestions.
pub fn list_suggestions(
    config: &Config,
    status: Option<SuggestionStatus>,
    limit: usize,
) -> Result<Vec<FlowSuggestion>> {
    let storage = super::catalog_storage::register_blocking(config)?;
    let status: Option<catalog::SuggestionStatus> = status
        .map(|status| serde_json::from_value(serde_json::to_value(status).unwrap_or(Value::Null)))
        .transpose()?;
    let request = catalog::ListSuggestionsRequest {
        storage,
        status,
        limit,
    };
    let suggestions: Vec<catalog::FlowSuggestion> =
        call_catalog(config, catalog::members::LIST_SUGGESTIONS, (request,))?;
    suggestions
        .into_iter()
        .map(|suggestion| serde_json::from_value(serde_json::to_value(suggestion)?).map_err(Into::into))
        .collect()
}

/// Set a suggestion's module-owned lifecycle status.
pub fn set_suggestion_status(
    config: &Config,
    id: &str,
    status: SuggestionStatus,
) -> Result<bool> {
    let storage = super::catalog_storage::register_blocking(config)?;
    let status: catalog::SuggestionStatus =
        serde_json::from_value(serde_json::to_value(status)?)?;
    let request = catalog::SetSuggestionStatusRequest {
        storage,
        id: id.to_owned(),
        status,
    };
    call_catalog(config, catalog::members::SET_SUGGESTION_STATUS, (request,))
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
