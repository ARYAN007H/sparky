//! The capability seam: five adapters implementing `tinyflows::caps` traits
//! over real OpenHuman services.
//!
//! Each tinyflows integration node hands its **whole** `node.config` to the
//! matching trait method — the adapter interprets a free-form JSON value the
//! flow author wrote, pulling a connection ref out of `config["connection_ref"]`
//! where relevant. See `my_docs/ohxtf/b1-engine-seam-domain/04-capability-seam.md`
//! for the source-verified node → trait contract this mirrors.
//!
//! All host errors are mapped to `tinyflows::error::EngineError::Capability`,
//! per the crate's contract (`caps` traits return `tinyflows::error::Result`).

#[cfg(test)]
pub(crate) use crate::integrations::composio::flow_gate::composio_connection_id;

use std::sync::Arc;

use async_trait::async_trait;
#[cfg(test)]
use serde_json::json;
use serde_json::Value;
use tinyflows::caps::*;
use tinyflows::error::{EngineError, Result};
#[cfg(test)]
use tinyflows::model::WorkflowGraph;

use crate::config::Config;
#[cfg(test)]
use crate::config::HttpRequestConfig;
#[cfg(test)]
use crate::flows;
#[cfg(test)]
use crate::security::credentials::HttpCredential;
use crate::security::credentials::HttpCredentialsStore;
#[cfg(test)]
use crate::security::GateDecision;
use crate::security::SecurityPolicy;
use tinybox_bus::CommandClass;

use super::*;

/// Parses a `"http_cred:<name>"` `connection_ref` for [`OpenHumanHttp`],
/// returning the trailing credential name. The host-side
/// [`HttpCredentialsStore`] (encrypted-at-rest bearer/basic/header
/// templates) is real and load-bearing — [`resolve_http_credential`] looks
/// the extracted name up in it and injects the resolved auth header
/// server-side. This function only does the parse; a malformed or missing
/// name (`None`) is what lets the caller fail the request closed instead of
/// silently sending it unauthenticated. See [`OpenHumanHttp::request`]'s doc
/// and the "Phase 2" note on the [`OpenHumanHttp`] struct for the full
/// resolution flow.
pub(crate) fn http_cred_name(conn: &str) -> Option<&str> {
    let name = conn.strip_prefix("http_cred:")?.trim();
    (!name.is_empty()).then_some(name)
}

pub(crate) async fn classify_composio_action_for_tier(slug: &str) -> CommandClass {
    crate::integrations::composio::flow_gate::classify_composio_action_for_tier(slug)
}

pub(crate) async fn is_curated_flow_tool(
    ctx: &crate::core::runtime::DispatchCtx,
    config: &Config,
    slug: &str,
) -> bool {
    crate::integrations::composio::flow_gate::is_curated_flow_tool(ctx, config, slug).await
}

/// [`ToolInvoker`] adapter over Composio (`crates/openhuman-core/src/integrations/composio/client.rs`).
///
/// **B2 (closes two B1 deviations, see
/// `my_docs/ohxtf/b2-triggers-trust/01-triggers-and-trust.md` §4-5):**
/// - **Curation + scope (hard allowlist)**: every call is checked against
///   [`is_curated_flow_tool`] — a deny-by-default gate that only allows a
///   slug resolving to a *known, curated* toolkit action, unlike the general
///   agent tool-call path's more permissive
///   `memory_sync::composio::providers::is_action_visible_with_pref` (see
///   [`is_curated_flow_tool`]'s doc for why the two differ). A non-curated /
///   unrecognized / out-of-scope slug is rejected with
///   `EngineError::Capability("tool not permitted: <slug>")` before any
///   Composio call. **As of tinyflows 0.3 this is load-bearing, not merely
///   defense-in-depth**: integration-node config (including `slug`) is now
///   `=`-expression evaluated against upstream/trigger data before `invoke`,
///   so a trigger payload *can* influence which tool a `=`-derived slug
///   resolves to. The curation gate runs on the **resolved** slug (verified:
///   a `=item.tool`-derived unknown slug is rejected here before Composio),
///   constraining any data-derived tool to the user's curated, in-scope,
///   connected set — and it still closes the case where an author hand-types
///   an arbitrary/typo'd slug.
/// - **connection_ref**: `conn` (`"composio:<toolkit>:<connection_id>"`) is
///   parsed and forwarded in both modes: Direct passes it to Composio's
///   execute endpoint, while Backend includes it as `connectionId` in the
///   authenticated `/agent-integrations/composio/execute` request. Omitting a
///   ref deliberately keeps the ambient-account behavior. A supplied but
///   stale/foreign id is still forwarded exactly so the provider rejects it;
///   it must never degrade to the ambient account (fixes #5751 / E-m3).
/// - **Trust gate**: invocation is also routed through the OpenHuman
///   `ApprovalGate` (mirrors `tinyagents/middleware.rs::ApprovalSecurityMiddleware`)
///   before dispatch, closing the Codex P1 finding that flow tool nodes
///   bypassed the Network/tool approval gate entirely. `ops::flows_run` /
///   `flows_resume` scope a `TrustedAutomation { Workflow }` origin around
///   the whole run, so the gate either auto-allows (pre-declared trust root)
///   or — when the flow's `require_approval` is set — parks for a real
///   decision. No gate installed (unit tests, some hosts) means no gating,
///   same as the existing agent tool-loop middleware.
///
/// // SECURITY NOTE (tinyflows 0.3, now the pinned version): integration nodes
/// // `=`-resolve config from upstream/trigger data, so a trigger-driven flow
/// // whose `slug`/`url` is `=`-derived lets untrusted trigger data pick *which*
/// // curated + in-scope + connected tool/endpoint runs (blast radius bounded by
/// // the curation + scope + connection checks above and the approval gate).
/// // For such flows authors should set `require_approval`. FOLLOW-UP: auto-force
/// // approval when a trigger-driven run's tool/http config contains `=`-exprs.
pub struct OpenHumanTools {
    /// The context the run acts under: the scope-pref reads of the curation
    /// gate resolve against it rather than the calling task's own scope.
    pub ctx: crate::core::runtime::DispatchCtx,
    pub config: Arc<Config>,
    pub security: Arc<SecurityPolicy>,
    /// The run's approval context (flow run correlation), handed to the gate
    /// with every acting dispatch.
    pub approval: crate::security::approval::ApprovalTurnContext,
}

pub(crate) async fn preflight_composio_args(
    config: &Config,
    slug: &str,
    args: &Value,
) -> Result<()> {
    crate::integrations::composio::flow_gate::preflight_composio_args(config, slug, args)
        .await
        .map_err(EngineError::Capability)
}

pub(crate) fn reject_unsuccessful_composio_response(
    slug: &str,
    response: crate::integrations::composio::ComposioExecuteResponse,
) -> Result<crate::integrations::composio::ComposioExecuteResponse> {
    crate::integrations::composio::flow_gate::reject_unsuccessful_composio_response(slug, response)
        .map_err(EngineError::Capability)
}

/// A [`ToolInvoker`] decorator that runs the host's Composio required-arg
/// preflight before delegating to `inner`.
///
/// Used by `dry_run_workflow`: the dry-run path executes against tinyflows'
/// echo mocks, which would happily accept a `null` required arg — wrapping
/// the mock invoker with this makes the wiring check actually check wiring,
/// so an unwired required arg fails the dry run with the same actionable
/// message a real run would produce.
pub struct PreflightToolInvoker {
    /// Host config, for the Composio schema lookup.
    pub config: Arc<Config>,
    /// The delegate that performs the actual invocation (e.g. the mock).
    pub inner: Arc<dyn ToolInvoker>,
}

#[async_trait]
impl ToolInvoker for PreflightToolInvoker {
    async fn invoke(&self, slug: &str, args: Value, conn: Option<&str>) -> Result<Value> {
        // Ask the backend that owns this slug to validate the args. Previously
        // this called the Composio preflight directly behind a
        // `!slug.starts_with("oh:")` test, which duplicated the dispatch rule
        // and hard-wired one namespace's knowledge into a generic wrapper.
        if let Some(backend) = tools::backend_for(slug) {
            backend.preflight(&self.config, slug, &args).await?;
        }
        self.inner.invoke(slug, args, conn).await
    }
}

#[async_trait]
impl ToolInvoker for OpenHumanTools {
    async fn invoke(&self, slug: &str, args: Value, conn: Option<&str>) -> Result<Value> {
        let ctx = tools::ToolCallCtx {
            dispatch: &self.ctx,
            config: &self.config,
            security: &self.security,
            approval: &self.approval,
        };
        match tools::backend_for(slug) {
            Some(backend) => backend.invoke(&ctx, slug, args, conn).await,
            None => Err(tools::unclaimed_slug_error(slug)),
        }
    }
}

/// Builds the [`Capabilities`] bundle for one run, wiring each supported
/// host-injected traits to a real OpenHuman adapter (see each adapter above,
/// and [`super::memory_adapter::OpenHumanMemory`] for `memory`, for its
/// contract).
///
/// `state_namespace` scopes the flow state KV ([`crate::flows::tinyflows::state::FlowState`]:
/// `flows.db`, or the storage backend when one is configured) so two saved flows that
/// use the same state key never read or overwrite each other — callers pass a
/// per-flow namespace (e.g. `"flow:<id>"`). This KV namespace is unrelated to
/// flow-scoped memory: `OpenHumanMemory` tags memory items with
/// `flows::flow_tag`, derived independently from the run's trusted origin.
///
/// `ctx` is the context the run acts under: the agent, code and HTTP adapters
/// act for it instead of reading the calling task's own scope.
///
/// `flow_run` is the run the capabilities serve: every acting dispatch hands it
/// to the approval gate (with the tool call), so a parked approval is stamped
/// with the exact flow and run. `None` for a run with no flow identity (a dry
/// run, a preview).
pub fn build_capabilities(
    ctx: &crate::core::runtime::DispatchCtx,
    config: Arc<Config>,
    state_namespace: impl Into<String>,
    flow_run: Option<crate::security::approval::FlowRunContext>,
) -> Capabilities {
    let approval = crate::security::approval::ApprovalTurnContext {
        flow_run,
        ..Default::default()
    };
    let security = Arc::new(SecurityPolicy::from_config(
        &config.autonomy,
        &config.workspace_dir,
        &config.action_dir,
    ));
    let http_config = config.http_request.clone();
    let http_creds = Arc::new(HttpCredentialsStore::from_config(&config));

    Capabilities {
        llm: Arc::new(OpenHumanLlm {
            ctx: ctx.clone(),
            config: config.clone(),
        }),
        tools: Arc::new(OpenHumanTools {
            ctx: ctx.clone(),
            config: config.clone(),
            security: security.clone(),
            approval: approval.clone(),
        }),
        http: Arc::new(OpenHumanHttp {
            ctx: ctx.clone(),
            config: config.clone(),
            security: security.clone(),
            http_config,
            http_creds,
            approval: approval.clone(),
        }),
        code: Arc::new(OpenHumanCode {
            ctx: ctx.clone(),
            config: config.clone(),
            security: security.clone(),
            approval: approval.clone(),
        }),
        state: crate::flows::tinyflows::state::FlowState::open(&config, state_namespace)
            .into_state_store(),
        agent: Some(Arc::new(OpenHumanAgentRunner {
            ctx: ctx.clone(),
            config: config.clone(),
            approval: approval.clone(),
        })),
        // Shell execution needs a dedicated OpenHuman adapter that applies the
        // host's autonomy and sandbox policy. Keep the capability unavailable
        // until that boundary exists rather than inheriting ambient process
        // access from the workflow engine.
        shell: None,
        // `spawn`/`gate` overlap only when a TaskRunner is injected. With
        // `None` the engine still produces the right answer — `spawn` runs its
        // work inline and hands back a settled ticket — so the failure mode is
        // a silent loss of concurrency rather than an error, which is exactly
        // the kind that survives a smoke test. Flow runs are already on tokio,
        // so take the crate's tokio-backed runner. It is in-process only:
        // tickets do not survive a restart, which is the right bound for work
        // a single run collects at its own gate.
        tasks: Some(Arc::new(TokioTaskRunner::new())),
        // OpenHuman already persists `RunOutcome::pending_approvals` and
        // resumes named gates through `flows_resume`. Leaving the optional
        // provider unset deliberately selects Tinyflows' compatible fallback
        // instead of creating a second review store beside that surface.
        approvals: None,
        memory: Some(Arc::new(
            crate::flows::tinyflows::memory_adapter::OpenHumanMemory {
                ctx: ctx.clone(),
                config: config.clone(),
                security,
                approval,
            },
        )),
        resolver: Arc::new(OpenHumanWorkflowResolver { config }),
    }
}

/// Opens the durable, cross-process checkpointer a `flows_run` uses via
/// `tinyflows::engine::run_with_checkpointer`:
/// `tinyflows_drivers::DriverCheckpointer` over the flow catalog's storage
/// ([`crate::flows::store`]) in the acting agent's scope.
///
/// tinyflows no longer has a SQLite checkpointer; the `checkpoints.db` it wrote
/// is imported once, so a run interrupted before the upgrade still resumes.
pub fn open_flow_checkpointer(
    config: &Config,
) -> anyhow::Result<Arc<dyn tinyflows::engine::Checkpointer<serde_json::Value>>> {
    let scoped = crate::flows::store::scoped(config)?;
    tracing::debug!(target: "flows", "[flows] opening checkpointer on the flow storage");
    Ok(Arc::new(tinyflows_drivers::DriverCheckpointer::<
        serde_json::Value,
    >::new(Arc::clone(scoped.documents()))))
}

#[cfg(test)]
#[path = "ops_tests.rs"]
mod tests;
