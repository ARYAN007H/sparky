//! Retiring one captured agent owner before its id or home is reusable.
use super::*;
use std::sync::{atomic::Ordering, Weak};

fn finish_removal(
    registry: &crate::runtime::AgentMap,
    id: &str,
    instance: &Weak<AgentInner>,
    home: &Path,
    purge: bool,
) -> Result<(), AgentError> {
    let mut agents = registry.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if !agents.get(id).is_some_and(|current| Weak::ptr_eq(current, instance)) {
        return Ok(());
    }
    // Keep the id reserved during purge. A second cleanup cannot delete the
    // home of a replacement that another caller has already instantiated.
    if purge && home.exists() {
        std::fs::remove_dir_all(home).map_err(|source| AgentError::Workspace {
            what: "delete the removed agent's home", source,
        })?;
    }
    agents.remove(id);
    Ok(())
}

impl AgentInner {
    pub(crate) fn cleanup_is_pending(&self) -> bool {
        self.mcp_cleanup.lock().unwrap_or_else(std::sync::PoisonError::into_inner).is_some()
    }

    pub(crate) async fn finish_mcp_cleanup(&self) -> Result<(), AgentError> {
        let cleanup = self.mcp_cleanup.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone();
        if let Some(cleanup) = cleanup {
            cleanup.finish().await.map_err(AgentError::McpCleanup)?;
            self.mcp_cleanup.lock().unwrap_or_else(std::sync::PoisonError::into_inner).take();
        }
        Ok(())
    }

    pub(crate) fn finish_removal(&self, purge: bool) -> Result<(), AgentError> {
        let instance = self.registry.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&self.id).filter(|weak| std::ptr::eq(weak.as_ptr(), self)).cloned();
        if let Some(instance) = instance {
            finish_removal(&self.registry, &self.id, &instance, &self.layout.home, purge)?;
        }
        Ok(())
    }

    /// Retry independently of the caller runtime. The task, rather than a
    /// process-global callback, holds the workspace guard while cleanup runs.
    pub(crate) fn defer_cleanup(&self, purge: bool) {
        let cleanup = self.mcp_cleanup.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone();
        if cleanup.as_ref().is_none_or(|cleanup| cleanup.finish_if_idle()) {
            if let Err(error) = self.finish_removal(purge) {
                log::warn!("[embed][agent] removed home cleanup failed: {error}");
            }
            return;
        }
        if self.cleanup_started.swap(true, Ordering::AcqRel) { return; }
        let Some(cleanup) = cleanup else { return; };
        let instance = self.registry.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&self.id).filter(|weak| std::ptr::eq(weak.as_ptr(), self)).cloned();
        let Some(instance) = instance else { return; };
        let registry = Arc::clone(&self.registry);
        let guard = Arc::clone(&self._runtime_guard);
        let home = self.layout.home.clone();
        let id = self.id.clone();
        let future = async move {
            let _guard = guard;
            let mut delay = std::time::Duration::from_millis(250);
            loop {
                let result = match cleanup.finish().await {
                    Ok(()) => finish_removal(&registry, &id, &instance, &home, purge),
                    Err(error) => Err(AgentError::McpCleanup(error)),
                };
                match result {
                    Ok(()) => return,
                    Err(error) => log::warn!("[embed][agent] captured cleanup remains pending: {error}"),
                }
                tokio::time::sleep(delay).await;
                delay = (delay * 2).min(std::time::Duration::from_secs(5));
            }
        };
        if let Err(error) = openhuman_core::mcp::host::spawn_agent_cleanup(future) {
            log::warn!("[embed][agent] cleanup executor unavailable: {error}");
        }
    }
}
