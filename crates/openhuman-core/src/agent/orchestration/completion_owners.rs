//! Which SaaS profile a detached sub-agent's completion belongs to, for the
//! bus subscriber that delivers it.
//!
//! The background-completion tables are keyed per profile
//! (`tenant::profile_key_in`), but the delivery subscriber
//! ([`super::background_delivery`]) runs off-task with no `CoreContext` scope,
//! so in SaaS it cannot read them. [`record_outcome`] runs inside the user's
//! scope: it notes the profile here under the sub-agent's task id (core-minted,
//! unique) and the parent session id, and the subscriber re-enters that
//! profile's context ([`context_for_profile`]) before it looks anything up.
//!
//! A session id is not unique across profiles (a web-chat one is
//! `{client_id, thread_id}`), so it can name several owners; each is drained in
//! its own scope, where it can only reach its own tables. Desktop contexts
//! carry no profile, so nothing is noted there and delivery stays unscoped.
//!
//! [`record_outcome`]: super::background_completions::record_outcome

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use crate::core::runtime::CoreContext;

/// Bound on noted ids; the oldest is evicted first.
const OWNERS_CAP: usize = 4096;

/// Bound on the profiles one (session) id can name.
const PROFILES_PER_ID: usize = 16;

#[derive(Default)]
struct Owners {
    by_id: HashMap<String, Vec<String>>,
    order: VecDeque<String>,
    driver_tasks: HashMap<String, DriverTaskOwner>,
    driver_task_order: VecDeque<String>,
}

#[derive(Clone)]
pub(crate) struct DriverTaskOwner {
    pub(crate) task_id: String,
    pub(crate) profile: Option<String>,
    pub(crate) workspace_dir: PathBuf,
    pub(crate) workspace: tinytools::WorkspaceDescriptor,
    pub(crate) parent_session: String,
    pub(crate) parent_thread_id: Option<String>,
}

fn owners() -> MutexGuard<'static, Owners> {
    static OWNERS: OnceLock<Mutex<Owners>> = OnceLock::new();
    OWNERS
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn driver_task_key(ctx: &crate::core::runtime::DispatchCtx, task_id: &str) -> String {
    use crate::core::runtime::tenant;
    tenant::profile_key_in(
        &tenant::isolated_unless_tenant(ctx.tenant(), "driver_task_owner"),
        task_id,
    )
}

/// Remember the exact scope needed to resolve and control a live driver task.
/// The job registry owns the lifecycle; this bounded index recovers its
/// parent/session scope from the product-facing task id.
pub(crate) fn note_driver_task(
    ctx: &crate::core::runtime::DispatchCtx,
    task_id: &str,
    workspace_dir: PathBuf,
    workspace: tinytools::WorkspaceDescriptor,
    parent_session: String,
    parent_thread_id: Option<String>,
) {
    let owner = DriverTaskOwner {
        task_id: task_id.to_owned(),
        profile: ctx.tenant().ok().and_then(|tenant| tenant.profile),
        workspace_dir,
        workspace,
        parent_session,
        parent_thread_id,
    };
    let key = driver_task_key(ctx, task_id);
    let mut st = owners();
    if !st.driver_tasks.contains_key(&key) {
        st.driver_task_order.push_back(key.clone());
    }
    st.driver_tasks.insert(key, owner);
    while st.driver_task_order.len() > OWNERS_CAP {
        if let Some(oldest) = st.driver_task_order.pop_front() {
            st.driver_tasks.remove(&oldest);
        }
    }
}

/// Return the exact parent scope cached when the current process registered
/// this task. A missing entry means the process has no live owner metadata.
pub(crate) fn driver_task_owner(
    ctx: &crate::core::runtime::DispatchCtx,
    task_id: &str,
) -> Option<DriverTaskOwner> {
    owners().driver_tasks.get(&driver_task_key(ctx, task_id)).cloned()
}

/// Return registered task scopes for one exact workspace/thread pair.
pub(crate) fn driver_tasks_for_thread(
    ctx: &crate::core::runtime::DispatchCtx,
    workspace_dir: &std::path::Path,
    thread_id: &str,
) -> Vec<(String, DriverTaskOwner)> {
    let profile = ctx.tenant().ok().and_then(|tenant| tenant.profile);
    owners()
        .driver_tasks
        .iter()
        .filter_map(|(key, owner)| {
            (owner.profile == profile
                && owner.workspace_dir == workspace_dir
                && owner.parent_thread_id.as_deref() == Some(thread_id))
                .then(|| (owner.task_id.clone(), owner.clone()))
        })
        .collect()
}

/// Note the calling task's profile as an owner of each id. A no-op outside a
/// profile (the desktop, embedded agents).
pub(crate) fn note(ctx: &crate::core::runtime::DispatchCtx, ids: &[&str]) {
    let Ok(Some(profile)) = ctx.tenant().map(|t| t.profile) else {
        return;
    };
    note_profile(ids, &profile);
}

/// Note `profile` as an owner of each id.
pub(crate) fn note_profile(ids: &[&str], profile: &str) {
    let mut st = owners();
    for id in ids {
        let fresh = !st.by_id.contains_key(*id);
        let profiles = st.by_id.entry((*id).to_string()).or_default();
        if !profiles.iter().any(|p| p == profile) {
            if profiles.len() >= PROFILES_PER_ID {
                profiles.remove(0);
            }
            profiles.push(profile.to_string());
        }
        if fresh {
            st.order.push_back((*id).to_string());
        }
    }
    while st.order.len() > OWNERS_CAP {
        if let Some(oldest) = st.order.pop_front() {
            st.by_id.remove(&oldest);
        }
    }
}

/// The profiles noted as owners of `id`, oldest first.
pub(crate) fn profiles_of(id: &str) -> Vec<String> {
    owners().by_id.get(id).cloned().unwrap_or_default()
}

/// The context to act for `profile` under, off-task: the SaaS agent host's
/// agent for it, reopened if it was evicted. `None` outside SaaS or for a
/// profile the host does not know.
pub(crate) async fn context_for_profile(profile: &str) -> Option<Arc<CoreContext>> {
    let id = crate::profiles::ProfileId::parse(profile).ok()?;
    match crate::profiles::host::host()?.open(&id).await {
        Ok(state) => Some(Arc::clone(state.context())),
        Err(error) => {
            log::warn!("[completion_owners] cannot open the owning agent: {error}");
            None
        }
    }
}
