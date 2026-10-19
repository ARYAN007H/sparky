//! One trusted `CapsHost` callback object for a single TinyFlows run.

use std::future::Future;
use std::{collections::HashMap, sync::Arc};

use crate::config::Config;
use crate::core::runtime::DispatchCtx;
use crate::flows::catalog_storage::CatalogStorageRegistration;
use crate::modules::tinyflows as module;
use tinyflows_bus::runs::{CallbackAddress, CapsHost};

use super::{agent::AgentRunState, service::CapsHostService};

/// Owns the host callback and its dependent owner-scoped catalog callback.
/// Keep this handle alive until the module has settled or parked the run.
pub(crate) struct CapsHostRegistration {
    _registration: module::ServedObjectRegistration,
    _catalog: Arc<CatalogStorageRegistration>,
    _binding: Arc<CapsHostBinding>,
    address: CapsHost,
}

/// Request-scoped values shared by all capability callbacks for one run.
pub(crate) struct CapsHostBinding {
    pub(crate) ctx: DispatchCtx,
    pub(crate) config: Arc<Config>,
    pub(crate) flow_id: String,
    pub(crate) run_id: String,
    pub(crate) security: Arc<crate::security::SecurityPolicy>,
    pub(crate) approval: crate::security::approval::ApprovalTurnContext,
    pub(crate) cancellation: tokio_util::sync::CancellationToken,
    pub(crate) agent_runs:
        tokio::sync::Mutex<HashMap<String, Arc<tokio::sync::Mutex<AgentRunState>>>>,
}

impl Drop for CapsHostRegistration {
    fn drop(&mut self) {
        self._binding.cancellation.cancel();
    }
}

impl CapsHostBinding {
    /// Re-enter the request's captured tenant, identity and turn origin for a
    /// bus callback that runs on the module runtime.
    pub(crate) async fn within<F: Future>(&self, future: F) -> F::Output {
        match self.ctx.core() {
            Some(context) => {
                crate::core::runtime::CoreContext::scope(Arc::clone(context), future).await
            }
            None => future.await,
        }
    }
}

impl CapsHostRegistration {
    /// Address to pass unchanged to `Runs.StartRun` or `Runs.ResumeRun`.
    pub(crate) fn address(&self) -> CapsHost {
        self.address.clone()
    }
}

/// Bind and serve all capability callbacks for one run generation.
pub(crate) async fn serve(
    ctx: DispatchCtx,
    config: Arc<Config>,
    flow_id: String,
    run_id: String,
    require_approval: bool,
    catalog: Arc<CatalogStorageRegistration>,
) -> Result<CapsHostRegistration, String> {
    let origin = crate::agent::turn_origin::AgentTurnOrigin::TrustedAutomation {
        job_id: flow_id.clone(),
        source: crate::agent::turn_origin::TrustedAutomationSource::Workflow { require_approval },
    };
    let ctx = ctx.with_turn_origin(origin);
    let security = Arc::new(crate::security::SecurityPolicy::from_config(
        &config.autonomy,
        &config.workspace_dir,
        &config.action_dir,
    ));
    let approval = crate::security::approval::ApprovalTurnContext {
        flow_run: Some(crate::security::approval::FlowRunContext {
            flow_id: flow_id.clone(),
            run_id: run_id.clone(),
        }),
        ..Default::default()
    };
    let binding = Arc::new(CapsHostBinding {
        ctx,
        config,
        flow_id,
        run_id,
        security,
        approval,
        cancellation: tokio_util::sync::CancellationToken::new(),
        agent_runs: tokio::sync::Mutex::new(HashMap::new()),
    });
    let path = format!(
        "/ai/tinyhumans/openhuman/CapsHost/{}",
        uuid::Uuid::new_v4().simple()
    );
    let address = CapsHost {
        callback: CallbackAddress {
            destination: module::HOST_BUS_NAME.to_string(),
            object_path: path.clone(),
        },
    };
    let service = CapsHostService::new(Arc::clone(&binding));
    let registration = module::serve_registered(path, service).await?;
    Ok(CapsHostRegistration {
        _registration: registration,
        _catalog: catalog,
        _binding: binding,
        address,
    })
}
