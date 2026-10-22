//! `PlanReviewGate` — parks a live interactive turn on a plan the user must
//! review before execution.
//!
//! Flow (mirrors [`crate::security::approval::ApprovalGate`], in-memory):
//! 1. The orchestrator calls the `request_plan_review` tool after laying out a
//!    thread-scoped plan. The tool calls [`PlanReviewGate::request_review`].
//! 2. The gate registers a `oneshot::Sender` keyed by `request_id`, publishes
//!    [`DomainEvent::PlanReviewRequested`] (bridged to the `plan_review_request`
//!    socket event), and parks the turn on the receiver.
//! 3. The UI's `PlanReviewCard` calls `plan_review_decide` (RPC) →
//!    [`PlanReviewGate::decide`] → sends the resolution on the oneshot.
//! 4. The parked turn wakes with [`PlanReviewResolution`] and the tool returns
//!    a result that tells the agent to proceed / stop / revise.
//!
//! On TTL or a dropped sender the gate resolves to `Reject` — fail-closed, so a
//! plan never executes without an explicit approval.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

use parking_lot::Mutex;
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::core::bus::BUS;
use crate::core::events::DomainEvent;
use crate::core::runtime::Tenant;

use super::types::PlanReviewResolution;

/// How long the gate parks a turn before timing out → `Reject`. Matches the
/// default approval TTL (10 min) — long enough for a human to read the plan.
const DEFAULT_PLAN_REVIEW_TTL: Duration = Duration::from_secs(60 * 10);

/// Snapshot of a parked plan review, captured at park time. Used to replay
/// the `plan_review_request` event to a socket that (re)joins the thread
/// room after the live emit — mirrors
/// [`crate::security::approval::PendingApproval`], but in-memory only (a
/// plan review parks a live turn; it has nothing to recover across a
/// restart).
#[derive(Debug, Clone)]
pub struct ParkedReview {
    pub request_id: String,
    pub thread_id: Option<String>,
    pub client_id: Option<String>,
    pub summary: String,
    pub steps: Vec<String>,
    /// The gated tool call's provider-assigned call id, when known.
    pub tool_call_id: Option<String>,
    /// RFC3339 expiry of this parked review.
    pub expires_at: Option<String>,
}

/// In-memory registry of parked plan reviews. Process-global singleton (see
/// [`global`]); no persistence — a parked interactive turn cannot resume
/// across a restart, so an orphaned review has nothing to recover.
pub struct PlanReviewGate {
    ttl: Duration,
    waiters: Mutex<HashMap<(Tenant, String), oneshot::Sender<PlanReviewResolution>>>,
    /// Newest parked `request_id` per thread, so a typed reply or a UI action
    /// that only knows the thread can resolve the latest review.
    thread_to_request: Mutex<HashMap<(Tenant, String), String>>,
    /// request_id → [`ParkedReview`] for every currently-parked review —
    /// consulted by `parked_review_for_thread` so a socket that (re)joins a
    /// thread room can be handed whatever is parked on it, the same
    /// reconciliation `ApprovalGate::parked_request_for_thread` performs.
    parked: Mutex<HashMap<(Tenant, String), ParkedReview>>,
}

/// A tenant captured from trusted server context for a live socket. Its
/// private field prevents transport payloads from manufacturing replay scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocketReviewScope(Tenant);

impl PlanReviewGate {
    fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            waiters: Mutex::new(HashMap::new()),
            thread_to_request: Mutex::new(HashMap::new()),
            parked: Mutex::new(HashMap::new()),
        }
    }

    /// Park the current turn on a plan review and block until the user decides
    /// or the TTL elapses. `summary` is a one-line description; `steps` are the
    /// ordered plan items shown in the review card. `thread_id` / `client_id`
    /// route the surface to the originating chat (absent → no routable surface,
    /// so the park TTL-rejects). `tool_call_id` is the gated `request_plan_review`
    /// call's provider-assigned id, when known, so the UI can correlate the
    /// review card back to the exact timeline row.
    pub async fn request_review(
        &self,
        tenant: Tenant,
        thread_id: Option<String>,
        client_id: Option<String>,
        summary: String,
        steps: Vec<String>,
        tool_call_id: Option<String>,
    ) -> PlanReviewResolution {
        let request_id = format!("plan-{}", Uuid::new_v4());
        let (tx, rx) = oneshot::channel();
        // Requests run under a derived agent context, while the profile's UI
        // decides and replays them from its profile-level context. Share the
        // profile's gate entries across its agents, but isolate other profiles.
        let scope = tenant.profile_only();
        let request_key = (scope.clone(), request_id.clone());
        self.waiters.lock().insert(request_key.clone(), tx);
        if let Some(tid) = thread_id.clone() {
            self.thread_to_request
                .lock()
                .insert((scope.clone(), tid), request_id.clone());
        }
        let expires_at = (chrono::Utc::now()
            + chrono::Duration::from_std(self.ttl).unwrap_or_default())
        .to_rfc3339();
        self.parked.lock().insert(
            request_key,
            ParkedReview {
                request_id: request_id.clone(),
                thread_id: thread_id.clone(),
                client_id: client_id.clone(),
                summary: summary.clone(),
                steps: steps.clone(),
                tool_call_id: tool_call_id.clone(),
                expires_at: Some(expires_at.clone()),
            },
        );

        // RAII cleanup: remove the waiter + thread + parked mappings on ANY
        // exit path, including when the parked future is cancelled/dropped
        // before `rx.await` completes (turn cancel, supervisor shutdown).
        // Without this, a cancelled review would leak entries that could be
        // re-decided against a dead turn or replayed as still-pending.
        let mut guard = ParkGuard {
            gate: self,
            tenant: scope,
            request_id: request_id.clone(),
            thread_id: thread_id.clone(),
            client_id: client_id.clone(),
            tool_call_id: tool_call_id.clone(),
            completed: false,
        };

        tracing::info!(
            request_id = %request_id,
            thread_id = ?thread_id,
            steps = steps.len(),
            "[plan_review::gate] parking turn for plan review"
        );

        BUS.publish(DomainEvent::PlanReviewRequested {
            request_id: request_id.clone(),
            thread_id: thread_id.clone(),
            client_id: client_id.clone(),
            summary,
            steps,
            tool_call_id: tool_call_id.clone(),
            expires_at: Some(expires_at),
        });

        let (resolution, terminal_reason) = match tokio::time::timeout(self.ttl, rx).await {
            Ok(Ok(resolution)) => (resolution, None),
            Ok(Err(_)) => {
                tracing::warn!(
                    request_id = %request_id,
                    "[plan_review::gate] review sender dropped → reject"
                );
                (PlanReviewResolution::Reject, Some("cancelled"))
            }
            Err(_) => {
                tracing::warn!(
                    request_id = %request_id,
                    "[plan_review::gate] review timed out → reject"
                );
                (PlanReviewResolution::Reject, Some("expired"))
            }
        };

        guard.publish_decided(&resolution, terminal_reason);
        tracing::info!(
            request_id = %request_id,
            decision = resolution.as_str(),
            "[plan_review::gate] review resolved"
        );
        resolution
    }

    /// The parked review on `thread_id`, if any — lets a socket (re)joining a
    /// thread room be handed whatever review is still parked on it.
    pub fn parked_review_for_thread(
        &self,
        tenant: &Tenant,
        thread_id: &str,
    ) -> Option<ParkedReview> {
        let scope = tenant.profile_only();
        let key = (scope.clone(), thread_id.to_string());
        let request_id = self.thread_to_request.lock().get(&key)?.clone();
        self.parked.lock().get(&(scope, request_id)).cloned()
    }

    /// The parked review a socket connected under its server-bound tenant
    /// should replay. The caller must capture this tenant from trusted server
    /// context when the socket connects; client event payloads are not identity.
    pub fn parked_review_for_socket(
        &self,
        socket_tenant: &SocketReviewScope,
        thread_id: &str,
    ) -> Option<ParkedReview> {
        self.parked_review_for_thread(&socket_tenant.0, thread_id)
    }

    /// Capture this socket's replay scope from the request's server-bound
    /// dispatch context. Socket transports may run connection handlers in a
    /// new task, so the request context must be passed explicitly rather than
    /// read from ambient task-local state. SaaS requests without a scoped
    /// tenant fail closed; local hosts retain their active profile/default.
    pub fn socket_scope_for_dispatch(
        &self,
        dispatch: &crate::core::runtime::DispatchCtx,
    ) -> Option<SocketReviewScope> {
        socket_scope_from_context(dispatch.tenant(), crate::core::runtime::is_saas())
    }

    /// Resolve a parked review by `request_id`. Returns `true` when a waiter was
    /// woken; `false` when the id is unknown (already decided / expired).
    pub fn decide(
        &self,
        tenant: &Tenant,
        request_id: &str,
        resolution: PlanReviewResolution,
    ) -> bool {
        let sender = self
            .waiters
            .lock()
            .remove(&(tenant.profile_only(), request_id.to_string()));
        match sender {
            Some(tx) => tx.send(resolution).is_ok(),
            None => {
                tracing::debug!(
                    request_id = %request_id,
                    "[plan_review::gate] decide for unknown/expired request"
                );
                false
            }
        }
    }

    /// Resolve the newest parked review on `thread_id` (typed-reply / thread-
    /// scoped path). Returns `true` when a waiter was woken.
    pub fn decide_by_thread(
        &self,
        tenant: &Tenant,
        thread_id: &str,
        resolution: PlanReviewResolution,
    ) -> bool {
        let scope = tenant.profile_only();
        let request_id = self
            .thread_to_request
            .lock()
            .get(&(scope, thread_id.to_string()))
            .cloned();
        match request_id {
            Some(id) => self.decide(tenant, &id, resolution),
            None => false,
        }
    }
}

fn socket_scope_from_context(
    tenant: Result<Tenant, crate::core::runtime::tenant::NoTenant>,
    saas: bool,
) -> Option<SocketReviewScope> {
    match tenant {
        Ok(tenant) => Some(SocketReviewScope(tenant)),
        Err(_) if saas => None,
        Err(_) => Some(SocketReviewScope(Tenant::default())),
    }
}

/// Removes a parked review's registry entries on drop — covers both the normal
/// return and cancellation of the parked future.
struct ParkGuard<'a> {
    gate: &'a PlanReviewGate,
    tenant: Tenant,
    request_id: String,
    thread_id: Option<String>,
    client_id: Option<String>,
    tool_call_id: Option<String>,
    completed: bool,
}

impl ParkGuard<'_> {
    fn publish_decided(&mut self, decision: &PlanReviewResolution, resolution: Option<&str>) {
        if self.completed {
            return;
        }
        BUS.publish(DomainEvent::PlanReviewDecided {
            request_id: self.request_id.clone(),
            decision: decision.as_str().to_string(),
            thread_id: self.thread_id.clone(),
            client_id: self.client_id.clone(),
            tool_call_id: self.tool_call_id.clone(),
            resolution: resolution.map(str::to_string),
        });
        self.completed = true;
    }
}

impl Drop for ParkGuard<'_> {
    fn drop(&mut self) {
        // Dropping the parked future is a terminal cancellation too. Publish
        // before removing routing metadata so the frontend can clear its card.
        self.publish_decided(&PlanReviewResolution::Reject, Some("cancelled"));
        self.gate
            .waiters
            .lock()
            .remove(&(self.tenant.clone(), self.request_id.clone()));
        self.gate
            .parked
            .lock()
            .remove(&(self.tenant.clone(), self.request_id.clone()));
        if let Some(tid) = &self.thread_id {
            let mut map = self.gate.thread_to_request.lock();
            let key = (self.tenant.clone(), tid.clone());
            if map.get(&key) == Some(&self.request_id) {
                map.remove(&key);
            }
        }
    }
}

/// Process-global plan-review gate.
pub fn global() -> &'static PlanReviewGate {
    static GATE: OnceLock<PlanReviewGate> = OnceLock::new();
    GATE.get_or_init(|| PlanReviewGate::new(DEFAULT_PLAN_REVIEW_TTL))
}

#[cfg(test)]
#[path = "gate_tests.rs"]
mod tests;
