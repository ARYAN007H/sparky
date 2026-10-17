use std::path::Path;
use std::sync::Arc;

use super::super::background_completions::{
    entry_for, router_pool, router_scope, state, test_store_overrides,
};
use tinyagents_tasks::CompletionStore;

/// A temporary workspace that forgets its router when dropped, so the
/// process-wide registry does not accumulate entries for deleted directories.
pub(crate) struct TestWorkspace(tempfile::TempDir);

impl TestWorkspace {
    pub(crate) fn new() -> Self {
        Self(tempfile::tempdir().expect("tempdir"))
    }

    pub(crate) fn path(&self) -> &std::path::Path {
        self.0.path()
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        forget_workspace_for_test(self.0.path());
    }
}

/// Register `store` as `workspace_dir`'s completion store, so a test can inject
/// one that fails.
pub(crate) fn install_store_for_test(workspace_dir: &Path, store: Arc<dyn CompletionStore>) {
    let scope = router_scope(workspace_dir);
    test_store_overrides()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(scope, store);
    router_pool().clear().expect("clear completion router pool");
    let _ = entry_for(workspace_dir);
}

/// Forget everything this process knows about `workspace_dir`, as a restart
/// would: the router (and its open log handle) and every thread/session mapping
/// that points at it. The on-disk log is untouched.
pub(crate) fn forget_workspace_for_test(workspace_dir: &Path) {
    let scope = router_scope(workspace_dir);
    let _ = router_pool().clear();
    test_store_overrides()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(&scope);
    let mut st = state();
    st.stores.remove(&scope);
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
    st.recovered_workspaces.remove(&scope);
    st.session_threads.clear();
    st.session_order.clear();
}
