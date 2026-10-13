//! The session store for code that has a workspace but no store of its own.
//!
//! Hosts install a [`SessionStoreProvider`] over the storage backend
//! ([`super::install`]). Code that was written against the workspace layout
//! (the run ledger, goals, todos, the turn journal, reapers) asks here: the
//! installed provider when there is one, else a provider over the same default
//! a host would have installed for that workspace
//! ([`crate::storage::local::backend_for`]). The answer is always the storage
//! ports; there is no file layout to fall back to any more.
//!
//! The tenant a call acts for picks the agent's stores
//! ([`crate::core::runtime::session_key`]): the desktop operator's is
//! [`DEFAULT_AGENT`](super::DEFAULT_AGENT), an embedded agent's its own id, a
//! SaaS profile's `<profile>~<agent>`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

use tinyagents_session::port::{AgentStores, SessionStoreProvider};
use tinyagents_session::run_ledger::RunLedgerStore;
use tinyagents_session::DriverSessionStores;
use tinystoragedrivers::{DocumentStore, MemoryStorage, StorageBackend};

/// One fallback provider and the backend it was built over.
struct Fallback {
    backend: Arc<dyn StorageBackend>,
    provider: Arc<DriverSessionStores>,
}

static FALLBACKS: LazyLock<Mutex<HashMap<PathBuf, Fallback>>> = LazyLock::new(Mutex::default);

/// Drops the fallback providers (and, with them, the default backends they
/// held), so a data reset can delete the files.
pub fn release_workspace_defaults() {
    FALLBACKS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clear();
    crate::storage::local::release_all();
}

/// The provider for `workspace_dir`: the installed one, else a
/// `DriverSessionStores` over the workspace's default backend.
///
/// Never fails: a default that cannot be opened (a read-only workspace) is
/// logged and replaced by an in-memory backend for that workspace, so a turn
/// degrades to a session that is not kept rather than to no session at all.
pub fn provider_for_workspace(workspace_dir: &Path) -> Arc<dyn SessionStoreProvider> {
    match super::installed() {
        Some(installed) => installed,
        None => fallback_provider(workspace_dir),
    }
}

/// The provider over `workspace_dir`'s default backend, installed or not.
fn fallback_provider(workspace_dir: &Path) -> Arc<dyn SessionStoreProvider> {
    let backend = match crate::storage::local::backend_for(workspace_dir) {
        Ok(backend) => backend,
        Err(error) => {
            log::error!(
                "[session_store] no storage backend for workspace={}: {error:#}",
                workspace_dir.display()
            );
            return memory_fallback(workspace_dir);
        }
    };
    let mut fallbacks = FALLBACKS.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(existing) = fallbacks.get(workspace_dir) {
        if Arc::ptr_eq(&existing.backend, &backend) {
            return existing.provider.clone();
        }
    }
    match DriverSessionStores::new(Arc::clone(&backend)) {
        Ok(provider) => {
            let provider = Arc::new(provider.recover_on_open(false));
            fallbacks.insert(
                workspace_dir.to_path_buf(),
                Fallback {
                    backend,
                    provider: provider.clone(),
                },
            );
            provider
        }
        Err(error) => {
            log::error!("[session_store] could not start the session store bridge: {error}");
            memory_fallback(workspace_dir)
        }
    }
}

fn memory_fallback(workspace_dir: &Path) -> Arc<dyn SessionStoreProvider> {
    static MEMORY: LazyLock<Mutex<HashMap<PathBuf, Arc<dyn SessionStoreProvider>>>> =
        LazyLock::new(Mutex::default);
    let mut memory = MEMORY.lock().unwrap_or_else(PoisonError::into_inner);
    memory
        .entry(workspace_dir.to_path_buf())
        .or_insert_with(|| {
            let backend: Arc<dyn StorageBackend> = Arc::new(MemoryStorage::new());
            match DriverSessionStores::new(backend) {
                Ok(provider) => Arc::new(provider.recover_on_open(false)),
                Err(_) => Arc::new(tinyagents_session::port::InMemorySessionStores::new()),
            }
        })
        .clone()
}

/// The key the calling tenant's stores are under, or `None` for a SaaS task
/// with no scope (which must not act).
fn tenant_agent_key() -> Option<String> {
    match crate::core::runtime::current_tenant() {
        Ok(tenant) => Some(crate::core::runtime::session_key(&tenant)),
        Err(no_tenant) => {
            log::warn!("[session_store] {no_tenant}");
            None
        }
    }
}

/// The calling tenant's stores for `workspace_dir` ([`provider_for_workspace`]).
#[must_use]
pub fn stores_for(workspace_dir: &Path) -> Option<AgentStores> {
    let key = tenant_agent_key()?;
    Some(provider_for_workspace(workspace_dir).for_agent(&key))
}

/// [`stores_for`] `workspace_dir`, or throwaway in-memory stores for a call
/// that has no tenant to act for (a SaaS task with no scope), so it neither
/// fails nor reaches another tenant's records.
#[must_use]
pub fn stores_or_ephemeral(workspace_dir: &Path) -> AgentStores {
    stores_for(workspace_dir).unwrap_or_else(|| {
        log::warn!("[session_store] no tenant; using throwaway in-memory stores");
        tinyagents_session::port::InMemorySessionStores::new().for_agent("ephemeral")
    })
}

/// The calling tenant's run ledger for `workspace_dir`.
///
/// The installed provider's when it keeps one, else the workspace default's.
/// `None` for a SaaS task with no scope; a ledger the backend refuses is
/// logged and also `None`, so best-effort recording never takes a turn down.
#[must_use]
pub fn ledger_for(workspace_dir: &Path) -> Option<Arc<dyn RunLedgerStore>> {
    let key = tenant_agent_key()?;
    if let Some(installed) = super::installed() {
        match installed.run_ledger_for_agent(&key) {
            Ok(Some(ledger)) => return Some(ledger),
            Ok(None) => {}
            Err(error) => {
                log::warn!("[session_store] run ledger unavailable agent={key}: {error}");
                return None;
            }
        }
    }
    let fallback = fallback_provider(workspace_dir);
    match fallback.run_ledger_for_agent(&key) {
        Ok(ledger) => ledger,
        Err(error) => {
            log::warn!("[session_store] run ledger unavailable agent={key}: {error}");
            None
        }
    }
}

/// The calling tenant's document store for `workspace_dir`, in the same scope
/// as its session stores — for the stores (run status, task ledgers) that are
/// plain collections beside them.
#[must_use]
pub fn current_documents_for(workspace_dir: &Path) -> Option<Arc<dyn DocumentStore>> {
    let key = tenant_agent_key()?;
    let backend = match crate::storage::local::backend_for(workspace_dir) {
        Ok(backend) => backend,
        Err(error) => {
            log::warn!("[session_store] no storage backend for documents: {error:#}");
            return None;
        }
    };
    match backend.for_scope(&crate::storage::scope_for_agent(&key)) {
        Ok(scoped) => Some(Arc::clone(scoped.documents())),
        Err(error) => {
            log::warn!("[session_store] documents scope refused agent={key}: {error}");
            None
        }
    }
}

/// The calling tenant's durable run-status store for `workspace_dir`: the
/// `harness_run_status` documents of its agent scope, beside its session
/// stores. A call with no tenant gets a throwaway in-memory store.
#[must_use]
pub fn status_store_for(workspace_dir: &Path) -> tinyagents_harness::observability::DriverStatusStore {
    let docs = current_documents_for(workspace_dir).unwrap_or_else(|| {
        log::warn!("[session_store] no tenant; using a throwaway run-status store");
        let scoped = MemoryStorage::new()
            .for_scope(&tinystoragedrivers::Scope::local())
            .expect("the memory backend binds the local scope");
        Arc::clone(scoped.documents())
    });
    tinyagents_harness::observability::DriverStatusStore::new(docs)
}

/// Runs a run-ledger call from synchronous code on the shared storage bridge.
///
/// # Errors
///
/// The ledger's own error, or the bridge's when it cannot run.
pub fn run_ledger_blocking<T, F>(future: F) -> Result<T, String>
where
    F: std::future::Future<Output = tinyagents_session::run_ledger::LedgerResult<T>>
        + Send
        + 'static,
    T: Send + 'static,
{
    crate::storage::block_on(async move { Ok(future.await) })
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}
