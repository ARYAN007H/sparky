//! Captured cleanup of an agent's existing configured module object.
use super::*;

/// A retired agent's original MCP owner and its retryable close receipts.
/// Capturing this handle never creates a client or loads a module.
#[derive(Clone)]
pub struct AgentMcpCleanup {
    directory: PathBuf,
    client: Arc<McpConfiguredClient>,
}

/// Retire the existing configured client for this exact agent directory.
/// An absent entry requires no cleanup and leaves the module loader untouched.
pub fn take_agent_cleanup(workspace: &std::path::Path, agent_id: &str) -> Option<AgentMcpCleanup> {
    let directory = workspace.join("agents").join(agent_id);
    let clients = CLIENTS
        .get()?
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let client = Arc::clone(clients.get(&directory)?);
    client.retiring.store(true, Ordering::Release);
    Some(AgentMcpCleanup { directory, client })
}

impl AgentMcpCleanup {
    /// Close the original object and evict it only after confirmed cleanup.
    /// The whole attempt, including lock acquisition and bus calls, has a
    /// 30-second deadline. Timeout returns `TransportFailed`; cancellation or
    /// error retains the same client and transition receipt for explicit retry.
    pub async fn finish(&self) -> Result<(), ModuleCallError> {
        tokio::time::timeout(TRANSITION_TIMEOUT, self.client.close())
            .await
            .map_err(|_| ModuleCallError::TransportFailed)??;
        self.evict();
        Ok(())
    }

    /// Finish an unused client without starting an asynchronous executor.
    /// An accepted call holding the lifecycle lock requires asynchronous drain.
    pub fn finish_if_idle(&self) -> bool {
        let Ok(state) = self.client.state.try_lock() else {
            return false;
        };
        if !matches!(*state, State::Closed) {
            return false;
        }
        self.evict();
        true
    }

    fn evict(&self) {
        let Some(clients) = CLIENTS.get() else {
            return;
        };
        let mut clients = clients
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if clients
            .get(&self.directory)
            .is_some_and(|current| Arc::ptr_eq(current, &self.client))
        {
            clients.remove(&self.directory);
        }
    }
}

/// Run captured cleanup on the process module executor when modules are enabled.
/// Without modules, the caller must keep its current Tokio runtime alive.
/// This creates no configured client and loads no MCP artifact.
pub fn spawn_agent_cleanup<F>(
    future: F,
) -> Result<tokio::task::JoinHandle<F::Output>, ModuleCallError>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    #[cfg(feature = "modules")]
    {
        crate::modules::host::spawn_cleanup(future).map_err(|_| ModuleCallError::TransportFailed)
    }
    #[cfg(not(feature = "modules"))]
    {
        tokio::runtime::Handle::try_current()
            .map(|handle| handle.spawn(future))
            .map_err(|_| ModuleCallError::Unavailable)
    }
}

/// A process-local directory reservation with no runtime guard or callback.
/// Dropping the token retains the fence; only confirmed host completion clears it.
#[derive(Clone)]
pub struct AgentDirectoryCleanup {
    directory: PathBuf,
    reservation: Arc<CleanupReservation>,
}

/// Reserve an agent's directory until host-owned cleanup succeeds.
/// This never creates a client or loads a module.
pub fn reserve_agent_directory(
    workspace: &std::path::Path,
    agent_id: &str,
) -> AgentDirectoryCleanup {
    let directory = workspace.join("agents").join(agent_id);
    let mut reservations = CLEANUP_RESERVATIONS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let reservation = Arc::clone(
        reservations
            .entry(directory.clone())
            .or_insert_with(|| Arc::new(CleanupReservation::default())),
    );
    reservation
        .host_completion_pending
        .store(true, Ordering::Release);
    AgentDirectoryCleanup {
        directory,
        reservation,
    }
}

impl AgentDirectoryCleanup {
    /// Release this exact reservation after module close and host purge succeed.
    /// An independently registered canonical callback keeps its own reservation.
    pub fn complete(&self) {
        let after_close = self
            .reservation
            .after_close
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut reservations = CLEANUP_RESERVATIONS
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if reservations
            .get(&self.directory)
            .is_some_and(|current| Arc::ptr_eq(current, &self.reservation))
        {
            self.reservation
                .host_completion_pending
                .store(false, Ordering::Release);
            if after_close.is_none() {
                reservations.remove(&self.directory);
            }
        }
    }
}
