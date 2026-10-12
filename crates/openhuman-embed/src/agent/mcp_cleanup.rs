//! Retiring one captured agent owner before its id or home is reusable.
use super::*;
use crate::runtime::PendingCleanup;
use std::sync::{atomic::Ordering, Weak};

pub(crate) fn finish_removal(
    registry: &crate::runtime::AgentMap,
    id: &str,
    pending: &PendingCleanup,
) -> Result<(), AgentError> {
    let mut agents = registry
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !agents
        .get(id)
        .is_some_and(|current| Weak::ptr_eq(current, &pending.instance))
    {
        return Ok(());
    }
    let purge = pending.purge || agents.pending.get(id).is_some_and(|current| current.purge);
    // Keep the id reserved throughout purge; never touch a successor's home.
    if purge && pending.home.exists() {
        std::fs::remove_dir_all(&pending.home).map_err(|source| AgentError::Workspace {
            what: "delete the removed agent's home",
            source,
        })?;
    }
    agents.remove(id);
    agents.pending.remove(id);
    Ok(())
}

/// Run at most three retries on the process module executor. The future owns
/// the workspace guard through at most 90 seconds of polled MCP cleanup and
/// 750 ms of backoff. It releases the guard before filesystem purge; retained
/// records contain no guard. Executor scheduling can delay these timed waits.
pub(crate) fn defer_pending(
    registry: Arc<crate::runtime::AgentMap>,
    id: String,
    pending: PendingCleanup,
    guard: Arc<crate::runtime::CoreGuard>,
) {
    if pending
        .cleanup
        .as_ref()
        .is_none_or(|cleanup| cleanup.finish_if_idle())
    {
        if let Err(error) = finish_removal(&registry, &id, &pending) {
            log::warn!("[embed][agent] removed home cleanup remains pending: {error}");
        }
        return;
    }
    if pending.retry_running.swap(true, Ordering::AcqRel) {
        return;
    }
    let running = Arc::clone(&pending.retry_running);
    let executor_failed = Arc::clone(&running);
    let future = async move {
        let mut guard = Some(guard);
        for attempt in 0..3 {
            let result = match &pending.cleanup {
                Some(cleanup) => cleanup.finish().await.map_err(AgentError::McpCleanup),
                None => Ok(()),
            };
            let result = result.and_then(|()| {
                complete_with_guard(&mut guard, || finish_removal(&registry, &id, &pending))
            });
            match result {
                Ok(()) => {
                    running.store(false, Ordering::Release);
                    return;
                }
                Err(error) => {
                    log::warn!("[embed][agent] captured cleanup remains pending: {error}")
                }
            }
            if attempt < 2 {
                tokio::time::sleep(std::time::Duration::from_millis(250 * (1 << attempt))).await;
            }
        }
        // The original receipt, id reservation and purge intent stay in the
        // registry. A later remove_agent call retries them explicitly.
        running.store(false, Ordering::Release);
    };
    if let Err(error) = openhuman_core::mcp::host::spawn_agent_cleanup(future) {
        executor_failed.store(false, Ordering::Release);
        log::warn!("[embed][agent] cleanup executor unavailable: {error}");
    }
}

fn complete_with_guard<T>(
    guard: &mut Option<Arc<crate::runtime::CoreGuard>>,
    complete: impl FnOnce() -> T,
) -> T {
    drop(guard.take());
    complete()
}

impl AgentInner {
    pub(crate) fn reserve_cleanup(&self, purge: bool) -> Option<PendingCleanup> {
        let cleanup = self
            .mcp_cleanup
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let mut agents = self
            .registry
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let instance = agents
            .get(&self.id)
            .filter(|weak| std::ptr::eq(weak.as_ptr(), self))
            .cloned()?;
        let pending = agents
            .pending
            .entry(self.id.clone())
            .or_insert_with(|| PendingCleanup {
                cleanup,
                instance,
                home: self.layout.home.clone(),
                purge,
                retry_running: Default::default(),
            });
        pending.purge |= purge;
        Some(pending.clone())
    }

    pub(crate) async fn finish_mcp_cleanup(&self) -> Result<(), AgentError> {
        let cleanup = self
            .mcp_cleanup
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        if let Some(cleanup) = cleanup {
            cleanup.finish().await.map_err(AgentError::McpCleanup)?;
        }
        Ok(())
    }

    pub(crate) fn finish_removal(&self, purge: bool) -> Result<(), AgentError> {
        if let Some(pending) = self.reserve_cleanup(purge) {
            finish_removal(&self.registry, &self.id, &pending)?;
        }
        Ok(())
    }

    pub(crate) fn defer_cleanup(&self, purge: bool) {
        if let Some(pending) = self.reserve_cleanup(purge) {
            defer_pending(
                Arc::clone(&self.registry),
                self.id.clone(),
                pending,
                Arc::clone(&self._runtime_guard),
            );
        }
    }
}

#[cfg(test)]
#[path = "mcp_cleanup_tests.rs"]
mod tests;
