//! The explicit context a controller handler runs under.
//!
//! Every registered controller handler is `fn(DispatchCtx, Map) -> ControllerFuture`.
//! The transport that accepted the call (the JSON-RPC server, the CLI, the
//! device tunnel, [`CoreRuntime::invoke_in`](super::CoreRuntime::invoke_in))
//! builds the [`DispatchCtx`] and hands it to
//! [`invoke_method`](crate::core::invoke::invoke_method); the registry passes it
//! on to the handler. Nothing in between reads it from task state, so two
//! concurrent dispatches for different tenants cannot see each other's value.
//!
//! A handler hands the context to whatever it calls (`ctx.core()` for the
//! [`CoreContext`], or the helpers built on it) rather than reaching for an
//! ambient one.

use crate::core::runtime::DispatchCtx;
use std::sync::Arc;

use super::CoreContext;

/// The per-call context a controller handler receives.
///
/// Cheap to clone (one `Arc`). It is `unbound` only when no [`CoreContext`]
/// exists at all: a unit test that never booted the core, or a call made
/// before boot. A booted host always dispatches with a bound context.
#[derive(Clone, Default)]
pub struct DispatchCtx {
    core: Option<Arc<CoreContext>>,
}

impl DispatchCtx {
    /// Dispatch under `core`: the tenant's context for a SaaS request, an
    /// agent's derived context for an embedded call, the host's own otherwise.
    pub fn new(core: Arc<CoreContext>) -> Self {
        Self { core: Some(core) }
    }

    /// Dispatch with no context. Only for tests and pre-boot callers; every
    /// reader of the context treats this as "nothing configured here".
    pub fn unbound() -> Self {
        Self { core: None }
    }

    /// Dispatch under the process default context (the first one built), or
    /// unbound before any exists. What the CLI and the desktop shell use: a
    /// single-user host has exactly one tenant.
    pub fn process_default() -> Self {
        Self {
            core: CoreContext::default_context(),
        }
    }

    /// The context this call runs under, if any.
    pub fn core(&self) -> Option<&Arc<CoreContext>> {
        self.core.as_ref()
    }

    /// Takes the context out, for a caller that spawns a task and must hand
    /// it over explicitly.
    pub fn into_core(self) -> Option<Arc<CoreContext>> {
        self.core
    }
}

impl From<Arc<CoreContext>> for DispatchCtx {
    fn from(core: Arc<CoreContext>) -> Self {
        Self::new(core)
    }
}

impl std::fmt::Debug for DispatchCtx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DispatchCtx")
            .field("bound", &self.core.is_some())
            .finish()
    }
}

#[cfg(test)]
#[path = "dispatch_ctx_tests.rs"]
mod tests;
