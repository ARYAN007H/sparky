//! Host client for the TinyFlows module's Catalog contract.
//!
//! The module owns catalog schemas, migrations, revisions, and update rules.
//! This file only translates the existing core-facing flow types to Catalog
//! requests and back. Run lifecycle and checkpoint records are owned by the
//! Runs adapter, not this definition client.

use anyhow::{Result, anyhow};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::path::PathBuf;
use tinyflows_bus::catalog::{
    self, DeleteWorkflowResult, FlowUpdateError as BusUpdateError, GetRevisionRequest,
    ListRevisionsRequest, ListWorkflowsResult, PutWorkflowRequest, PutWorkflowResponse,
    SetEnabledRequest, Workflow, WorkflowRevision,
};
use tinyflows_catalog::{Flow, FlowRevision, FlowSuggestion, SuggestionStatus};

use crate::config::Config;
use crate::modules::tinyflows as module;

/// Retained only for callers that display the legacy workflow folder path.
pub fn dir(config: &Config) -> PathBuf {
    config.workspace_dir.join("flows")
}

pub(crate) fn call_catalog<T, A>(config: &Config, member: &'static str, args: A) -> Result<T>
where
    T: DeserializeOwned + Send + 'static,
    A: Serialize + Send + 'static,
{
    let config = config.clone();
    crate::storage::block_on_anyhow(async move {
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

pub(crate) fn roundtrip<T, U>(value: T) -> Result<U>
where
    T: Serialize,
    U: DeserializeOwned,
{
    serde_json::from_value(serde_json::to_value(value)?).map_err(Into::into)
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

/// Seed a caller-chosen id through CatalogStorage for host tests.
#[cfg(test)]
pub fn upsert_flow(config: &Config, flow: &Flow) -> Result<()> {
    test_seed_flow(config, flow)
}

#[cfg(test)]
fn test_seed_flow(config: &Config, flow: &Flow) -> Result<()> {
    let storage = super::catalog_storage::register_blocking(config)?;
    let _: ListWorkflowsResult =
        call_catalog(config, catalog::members::LIST_WORKFLOWS, (storage.clone(),))?;
    let created_ms = chrono::DateTime::parse_from_rfc3339(&flow.created_at)
        .map(|date| date.timestamp_millis())
        .unwrap_or(0);
    let updated_ms = chrono::DateTime::parse_from_rfc3339(&flow.updated_at)
        .map(|date| date.timestamp_millis())
        .unwrap_or(created_ms);
    let mut doc = serde_json::json!({
        "name": flow.name,
        "graph": flow.graph,
        "enabled": flow.enabled,
        "created_ms": created_ms,
        "updated_ms": updated_ms,
        "require_approval": flow.require_approval,
        "revision": 1,
        "incarnation": uuid::Uuid::new_v4().to_string(),
    });
    if let Some(status) = &flow.last_status {
        doc["last_status"] = Value::String(status.clone());
    }
    if let Some(last_run_at) = &flow.last_run_at {
        if let Ok(date) = chrono::DateTime::parse_from_rfc3339(last_run_at) {
            doc["last_run_ms"] = Value::from(date.timestamp_millis());
        }
    }
    let request = catalog::DocPutRequest {
        collection: "flows.definitions".into(),
        id: flow.id.clone(),
        doc,
        precondition: catalog::CatalogDocumentPrecondition::None,
        schema: 2,
        expires_ms: None,
        created_ms: Some(created_ms),
    };
    let _: catalog::CatalogStorageReply = call_catalog(config, catalog::storage::PUT, (request,))?;
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
    let response: PutWorkflowResponse =
        call_catalog(config, catalog::members::PUT_WORKFLOW, (request,))?;
    put_result(response)
}

/// Create a workflow through the module's Catalog contract.
pub fn create_flow(
    config: &Config,
    name: String,
    graph: tinyflows::model::WorkflowGraph,
    require_approval: bool,
    enabled: bool,
) -> Result<Flow> {
    let storage = super::catalog_storage::register_blocking(config)?;
    let request = PutWorkflowRequest::Create {
        storage,
        name,
        graph: serde_json::to_value(graph)?,
        require_approval,
    };
    let response: PutWorkflowResponse =
        call_catalog(config, catalog::members::PUT_WORKFLOW, (request,))?;
    let flow = put_result(response)?;
    if !enabled && flow.enabled {
        set_enabled(config, &flow.id, false)
    } else {
        Ok(flow)
    }
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
    let listed: ListWorkflowsResult =
        call_catalog(config, catalog::members::LIST_WORKFLOWS, (storage.clone(),))?;
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
    Ok((
        flows.into_iter().filter(|flow| flow.enabled).collect(),
        skipped,
    ))
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
) -> std::result::Result<Flow, BusUpdateError> {
    let storage = super::catalog_storage::register_blocking(config).map_err(|error| {
        BusUpdateError::Store {
            message: error.to_string(),
        }
    })?;
    let request = PutWorkflowRequest::Update {
        storage,
        id: id.to_owned(),
        name,
        graph: serde_json::to_value(graph).map_err(|error| BusUpdateError::Store {
            message: error.to_string(),
        })?,
        require_approval,
        expected_version: expected_updated_at.map(str::to_owned),
        force_disarm_automatic: force_disarm_if_automatic,
    };
    let response: PutWorkflowResponse =
        call_catalog(config, catalog::members::PUT_WORKFLOW, (request,)).map_err(|error| {
            BusUpdateError::Store {
                message: error.to_string(),
            }
        })?;
    match response {
        PutWorkflowResponse::Ok(result) => {
            flow_from_wire(result.workflow).map_err(|error| BusUpdateError::Store {
                message: error.to_string(),
            })
        }
        PutWorkflowResponse::Error(error) => Err(error),
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
    let revisions: Vec<WorkflowRevision> =
        call_catalog(config, catalog::members::LIST_REVISIONS, (request,))?;
    revisions.into_iter().map(roundtrip).collect()
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
    let revision: Option<WorkflowRevision> =
        call_catalog(config, catalog::members::GET_REVISION, (request,))?;
    revision.map(roundtrip).transpose()
}

/// Upsert module-owned discovery suggestions.
pub fn upsert_suggestions(config: &Config, suggestions: &[FlowSuggestion]) -> Result<usize> {
    let storage = super::catalog_storage::register_blocking(config)?;
    let suggestions = suggestions
        .iter()
        .cloned()
        .map(roundtrip)
        .collect::<Result<Vec<catalog::FlowSuggestion>>>()?;
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
    let status: Option<catalog::SuggestionStatus> = status.map(roundtrip).transpose()?;
    let request = catalog::ListSuggestionsRequest {
        storage,
        status,
        limit,
    };
    let suggestions: Vec<catalog::FlowSuggestion> =
        call_catalog(config, catalog::members::LIST_SUGGESTIONS, (request,))?;
    suggestions.into_iter().map(roundtrip).collect()
}

/// Set a suggestion's module-owned lifecycle status.
pub fn set_suggestion_status(config: &Config, id: &str, status: SuggestionStatus) -> Result<bool> {
    let storage = super::catalog_storage::register_blocking(config)?;
    let status: catalog::SuggestionStatus = roundtrip(status)?;
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
