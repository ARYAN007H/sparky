use std::path::Path;
use std::sync::Arc;

use tinyagents_tasks::CompletionStore;

/// A temporary workspace that forgets its router when dropped, so the
/// process-wide registry does not accumulate entries for deleted directories.
#[cfg(test)]
pub(crate) struct TestWorkspace(tempfile::TempDir);

#[cfg(test)]
impl TestWorkspace {
    pub(crate) fn new() -> Self {
        Self(tempfile::tempdir().expect("tempdir"))
    }

    pub(crate) fn path(&self) -> &std::path::Path {
        self.0.path()
    }
}

#[cfg(test)]
impl Drop for TestWorkspace {
    fn drop(&mut self) {
        forget_workspace_for_test(self.0.path());
    }
}

/// Register `store` as `workspace_dir`'s completion store, so a test can inject
/// one that fails.
#[cfg(test)]
pub(crate) fn install_store_for_test(workspace_dir: &Path, store: Arc<dyn CompletionStore>) {
    let entry = Arc::new(Entry {
        router: Arc::new(new_router(store.clone())),
        store,
    });
    state().routers.insert(workspace_dir.to_path_buf(), entry);
}

/// Forget everything this process knows about `workspace_dir`, as a restart
/// would: the router (and its open log handle) and every thread/session mapping
/// that points at it. The on-disk log is untouched.
#[cfg(test)]
pub(crate) fn forget_workspace_for_test(workspace_dir: &Path) {
    let mut st = state();
    st.routers.remove(workspace_dir);
    let gone: Vec<String> = st
        .thread_workspaces
        .iter()
        .filter(|(_, ws)| ws.as_path() == workspace_dir)
        .map(|(thread, _)| thread.clone())
        .collect();
    for thread in gone {
        st.thread_workspaces.remove(&thread);
        st.stopped_threads.remove(&thread);
        st.deleted_threads.remove(&thread);
    }
    st.recovered_workspaces.remove(workspace_dir);
    st.session_threads.clear();
    st.session_order.clear();
}

