//! [`TenantSessionStores`]: one upstream rebindable provider per tenant.
//!
//! TinyAgents' rebindable provider serves one tenant and derives
//! `tenant.agent(id)` itself. The host keeps its flat session keys
//! ([`crate::core::runtime::session_key`]: the agent id for the desktop
//! operator, `<profile>~<agent>` for a SaaS profile), so this provider decodes
//! the key into `(profile, agent)` and forwards to the provider of that
//! tenant (`Scope::local()` or `Scope::tenant(profile)`), created on first
//! use. Every scope a session writes is therefore `local/agent:<id>` or
//! `u:<profile>/agent:<id>`, inside the tenant fence.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, PoisonError, RwLock, Weak};

use tinyagents_harness::store::SchemaAdmission;
use tinyagents_session::port::{
    AgentStores, SessionStoreLease, SessionStoreLeaseError, SessionStoreProvider,
};
use tinyagents_session::{RebindableSessionStores, SessionStoreRegistry};
use tinystoragedrivers::{Scope, StorageBackend, StorageError};

#[derive(Default)]
struct TenantStoreRegistry {
    providers: Arc<SessionStoreRegistry>,
    state: Mutex<TenantStoreRegistryState>,
}

#[derive(Default)]
struct TenantStoreRegistryState {
    backends: Vec<Weak<RwLock<Arc<dyn StorageBackend>>>>,
    tenants: Vec<Weak<Mutex<HashMap<Scope, Arc<RebindableSessionStores>>>>>,
    current_backend: Option<Arc<dyn StorageBackend>>,
}

impl TenantStoreRegistry {
    fn rebind(&self, backend: &Arc<dyn StorageBackend>) -> usize {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let backends = state
            .backends
            .iter()
            .filter_map(Weak::upgrade)
            .collect::<Vec<_>>();
        let tenant_maps = state
            .tenants
            .iter()
            .filter_map(Weak::upgrade)
            .collect::<Vec<_>>();
        // `route` locks tenants before reading the backend. Keep that lock
        // order while preventing new providers from appearing during rebind.
        let tenant_maps = tenant_maps
            .iter()
            .map(|tenants| tenants.lock().unwrap_or_else(PoisonError::into_inner))
            .collect::<Vec<_>>();
        let mut bindings = backends
            .iter()
            .map(|backend| backend.write().unwrap_or_else(PoisonError::into_inner))
            .collect::<Vec<_>>();

        let mut providers = tenant_maps
            .iter()
            .zip(&bindings)
            .flat_map(|(tenants, current_backend)| {
                tenants.iter().map(move |(tenant, provider)| {
                    (
                        tenant.as_str().to_owned(),
                        Arc::clone(provider),
                        Arc::clone(current_backend),
                    )
                })
            })
            .collect::<Vec<_>>();
        // A stable order makes a later-provider failure deterministic and
        // keeps rollback behavior predictable across runs.
        providers.sort_by(|left, right| left.0.cmp(&right.0));

        let mut moved: Vec<(Arc<RebindableSessionStores>, Arc<dyn StorageBackend>)> =
            Vec::with_capacity(providers.len());
        for (_, provider, old_backend) in &providers {
            if let Err(error) = provider.rebind(Arc::clone(backend)) {
                let mut rollback_failures = Vec::new();
                for (moved_provider, moved_backend) in moved.iter().rev() {
                    if let Err(rollback_error) = moved_provider.rebind(Arc::clone(moved_backend)) {
                        rollback_failures.push(rollback_error.to_string());
                    }
                }
                if rollback_failures.is_empty() {
                    tracing::error!(target: "openhuman::storage", %error, "[session_store] tenant provider rebind failed; prior providers were restored");
                    return 0;
                }
                tracing::error!(target: "openhuman::storage", %error, rollback_errors = ?rollback_failures, restored = moved.len() - rollback_failures.len(), "[session_store] tenant provider rebind and rollback failed; providers may remain split across backends");
                // The return count is intentionally non-zero when rollback
                // could not restore every provider, so callers cannot mistake
                // this for a clean no-op.
                return rollback_failures.len();
            }
            moved.push((Arc::clone(provider), Arc::clone(old_backend)));
        }

        for current in &mut bindings {
            **current = Arc::clone(backend);
        }
        state.backends.retain(|weak| weak.strong_count() > 0);
        state.tenants.retain(|weak| weak.strong_count() > 0);
        state.current_backend = Some(Arc::clone(backend));
        moved.len()
    }

    fn register_backend(
        &self,
        backend: &Arc<RwLock<Arc<dyn StorageBackend>>>,
        tenants: &Arc<Mutex<HashMap<Scope, Arc<RebindableSessionStores>>>>,
    ) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(current) = &state.current_backend {
            *backend.write().unwrap_or_else(PoisonError::into_inner) = Arc::clone(current);
        }
        state.backends.retain(|weak| weak.strong_count() > 0);
        state.backends.push(Arc::downgrade(backend));
        state.tenants.retain(|weak| weak.strong_count() > 0);
        state.tenants.push(Arc::downgrade(tenants));
    }
}

static PROCESS_SESSION_STORES: LazyLock<Arc<TenantStoreRegistry>> =
    LazyLock::new(|| Arc::new(TenantStoreRegistry::default()));

/// Rebinds the host's live tenant routers to `backend`.
pub fn rebind_all(backend: &Arc<dyn StorageBackend>) -> usize {
    PROCESS_SESSION_STORES.rebind(backend)
}

/// Session stores over one backend, routed to a per-tenant provider.
pub struct TenantSessionStores {
    backend: Arc<RwLock<Arc<dyn StorageBackend>>>,
    recover_on_open: bool,
    admission: SchemaAdmission,
    tenants: Arc<Mutex<HashMap<Scope, Arc<RebindableSessionStores>>>>,
    registry: Arc<TenantStoreRegistry>,
    /// Names this router in [`SessionStoreProvider::destination_key`].
    id: uuid::Uuid,
}

impl std::fmt::Debug for TenantSessionStores {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tenants = self.tenants.lock().unwrap_or_else(PoisonError::into_inner);
        let backend = self.backend.read().unwrap_or_else(PoisonError::into_inner);
        f.debug_struct("TenantSessionStores")
            .field("driver", &backend.driver())
            .field("tenants", &tenants.len())
            .finish_non_exhaustive()
    }
}

impl TenantSessionStores {
    fn with_registry(
        backend: Arc<dyn StorageBackend>,
        recover_on_open: bool,
        admission: SchemaAdmission,
        registry: Arc<TenantStoreRegistry>,
    ) -> Self {
        Self {
            backend: Arc::new(RwLock::new(backend)),
            recover_on_open,
            admission,
            tenants: Arc::new(Mutex::new(HashMap::new())),
            registry,
            id: uuid::Uuid::new_v4(),
        }
    }

    /// Stores over `backend`, registered for workspace rebinds.
    ///
    /// # Errors
    ///
    /// When the TinyAgents session provider cannot start.
    pub fn new(
        backend: Arc<dyn StorageBackend>,
        recover_on_open: bool,
    ) -> Result<Self, StorageError> {
        let router = Self::with_registry(
            backend,
            recover_on_open,
            SchemaAdmission::Declare,
            Arc::clone(&PROCESS_SESSION_STORES),
        );
        router
            .registry
            .register_backend(&router.backend, &router.tenants);
        Ok(router)
    }

    /// Workspace fallback stores are tied to a fixed workspace backend and
    /// therefore must not join the process-wide workspace-rebind registry.
    pub(super) fn new_workspace_local(
        backend: Arc<dyn StorageBackend>,
        recover_on_open: bool,
    ) -> Result<Self, StorageError> {
        Ok(Self::with_registry(
            backend,
            recover_on_open,
            SchemaAdmission::Declare,
            Arc::new(TenantStoreRegistry::default()),
        ))
    }

    /// The provider of the tenant `key` belongs to, and the agent id within it.
    fn route(&self, key: &str) -> (Arc<RebindableSessionStores>, String) {
        let (profile, mut agent) = crate::core::runtime::split_session_key(key);
        if agent.is_empty() {
            agent = crate::storage::scope::DEFAULT_AGENT.to_owned();
        }
        let tenant = crate::storage::tenant_scope(profile.as_deref());
        let mut tenants = self.tenants.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(provider) = tenants.get(&tenant) {
            return (Arc::clone(provider), agent);
        }
        let provider = match RebindableSessionStores::new_in_tenant(
            &self.registry.providers,
            Arc::clone(&self.backend.read().unwrap_or_else(PoisonError::into_inner)),
            tenant.clone(),
            self.recover_on_open,
        ) {
            Ok(provider) => provider,
            Err(error) => {
                tracing::error!(target: "openhuman::storage", tenant = tenant.as_str(), %error, "[session_store] could not open tenant session provider");
                // Storage is unavailable, so keep this tenant isolated in a
                // temporary in-memory provider rather than redirecting its
                // session data into another tenant's store.
                return (
                    RebindableSessionStores::new_in_tenant(
                        &self.registry.providers,
                        Arc::new(crate::storage::MemoryStorage::new()),
                        tenant.clone(),
                        false,
                    )
                    .expect("in-memory session store provider must initialize"),
                    agent,
                );
            }
        };
        tracing::debug!(
            target: "openhuman::storage",
            tenant = tenant.as_str(),
            "[session_store] opened the tenant's session provider"
        );
        tenants.insert(tenant, Arc::clone(&provider));
        (provider, agent)
    }

    fn providers(&self) -> Vec<Arc<RebindableSessionStores>> {
        self.tenants
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
            .cloned()
            .collect()
    }
}

impl SessionStoreProvider for TenantSessionStores {
    fn for_agent(&self, agent_id: &str) -> AgentStores {
        let (provider, agent) = self.route(agent_id);
        provider.for_agent(&agent)
    }

    fn try_for_agent(&self, agent_id: &str) -> Result<AgentStores, StorageError> {
        let (provider, agent) = self.route(agent_id);
        provider.try_for_agent(&agent)
    }

    fn lease_for_agent(&self, agent_id: &str) -> Result<SessionStoreLease, SessionStoreLeaseError> {
        let (provider, agent) = self.route(agent_id);
        provider.lease_for_agent(&agent)
    }

    fn bind_actor(
        &self,
        backend: Arc<dyn StorageBackend>,
    ) -> Result<Option<Arc<dyn SessionStoreProvider>>, StorageError> {
        self.bind_actor_with_schema_admission(backend, self.admission)
    }

    fn bind_actor_with_schema_admission(
        &self,
        backend: Arc<dyn StorageBackend>,
        admission: SchemaAdmission,
    ) -> Result<Option<Arc<dyn SessionStoreProvider>>, StorageError> {
        let actor = Self::with_registry(
            backend,
            self.recover_on_open,
            admission,
            Arc::new(TenantStoreRegistry::default()),
        );
        Ok(Some(Arc::new(actor)))
    }

    fn run_ledger_for_agent(
        &self,
        agent_id: &str,
    ) -> tinyagents_session::run_ledger::LedgerResult<
        Option<Arc<dyn tinyagents_session::run_ledger::RunLedgerStore>>,
    > {
        let (provider, agent) = self.route(agent_id);
        provider.run_ledger_for_agent(&agent)
    }

    fn conversations_for_agent(
        &self,
        agent_id: &str,
    ) -> tinyagents_session::threads::ConversationResult<
        Option<Arc<dyn tinyagents_session::threads::ConversationPersistence>>,
    > {
        let (provider, agent) = self.route(agent_id);
        provider.conversations_for_agent(&agent)
    }

    fn recover(&self) -> anyhow::Result<()> {
        for provider in self.providers() {
            provider.recover()?;
        }
        Ok(())
    }

    fn destination_key(&self) -> Option<String> {
        Some(format!(
            "{}://{}",
            self.backend
                .read()
                .unwrap_or_else(PoisonError::into_inner)
                .driver(),
            self.id
        ))
    }
}

#[cfg(test)]
#[path = "tenant_router_tests.rs"]
mod tests;
