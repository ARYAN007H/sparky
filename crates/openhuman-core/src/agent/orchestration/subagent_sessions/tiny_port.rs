//! Adapter from TinyAgents' durable subagent-session port to OpenHuman storage.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tinyagents_orchestration::subagent::{
    DriverSubagentSessionStore, DurableSubagentSession as TinySession,
    DurableSubagentStatus as TinyStatus, ReuseDecision as TinyReuseDecision,
    ScopedSubagentSessionStore, SubagentError, SubagentSessionSelector as TinySelector,
    SubagentSessionStore as TinySessionStore, SubagentSessionUpsert as TinyUpsert,
};

use super::types::{DurableSubagentSession, DurableSubagentStatus, SubagentSessionSelector};
use super::{SubagentSessionStore, SubagentSessionUpsert};

/// Opens TinyAgents' async session store in the caller's explicit storage
/// scope and binds it to one parent session/thread. The core's storage handle
/// is captured before returning, so later trait calls never consult ambient
/// task-local scope.
pub(crate) fn open_for_context(
    ctx: &crate::core::runtime::DispatchCtx,
    workspace_dir: PathBuf,
    parent_session: impl Into<String>,
    parent_thread_id: Option<String>,
    jobs: Option<tinyagents_harness_tools::subagents::SubAgentJobRegistry>,
) -> Result<Arc<dyn TinySessionStore>, SubagentError> {
    const DOMAIN: &str = "subagent_sessions";
    let open_documents = || {
        let opened = crate::storage::local::open(DOMAIN, &workspace_dir).map_err(|error| {
            SubagentError::Persistence(format!("open session store: {error:#}"))
        })?;
        let scoped = opened.scoped().map_err(|error| {
            SubagentError::Persistence(format!("scope session store: {error:#}"))
        })?;
        Ok::<_, SubagentError>(Arc::clone(scoped.documents()))
    };
    let context = ctx.resolved().ok_or_else(|| {
        SubagentError::Persistence("subagent session store has no resolved owner scope".into())
    })?;
    let documents = crate::core::runtime::CoreContext::sync_scope(context, open_documents)?;
    let driver = Arc::new(DriverSubagentSessionStore::new(documents)?);
    let parent_session = parent_session.into();
    import_host_sessions(
        ctx,
        &workspace_dir,
        &parent_session,
        parent_thread_id.as_deref(),
        &driver,
    )?;
    let owner = crate::agent::tinyagents::tool_registration::subagent_owner_key(
        &workspace_dir,
        &parent_session,
        parent_thread_id.as_deref(),
    );
    let scoped: Arc<dyn TinySessionStore> = if let Some(jobs) = jobs {
        Arc::new(ScopedSubagentSessionStore::with_close_callback(
            Arc::clone(&driver),
            parent_session.clone(),
            parent_thread_id.clone(),
            move |_session_id, _task_id, key| {
                if let Err(error) = jobs.cancel_key_owned(key, owner.clone()) {
                    log::debug!("[subagent_sessions] close cancellation was not applied: {error}");
                }
            },
        ))
    } else {
        Arc::new(ScopedSubagentSessionStore::new(
            Arc::clone(&driver),
            parent_session.clone(),
            parent_thread_id.clone(),
        ))
    };
    Ok(Arc::new(TaskIdSessionLookup {
        inner: scoped,
        parent_session,
        parent_thread_id,
    }))
}

/// Copies the pre-driver host collection into TinyAgents' collection without
/// removing or rewriting the source rows. The old collection remains the
/// owner of OpenHuman-only `parent_epoch` until the host's epoch sidecar is
/// established independently.
fn import_host_sessions(
    ctx: &crate::core::runtime::DispatchCtx,
    workspace_dir: &std::path::Path,
    parent_session: &str,
    parent_thread_id: Option<&str>,
    driver: &DriverSubagentSessionStore,
) -> Result<(), SubagentError> {
    let legacy = SubagentSessionStore::new_in_context(ctx, workspace_dir.to_path_buf())
        .map_err(SubagentError::Persistence)?
        .load()
        .map_err(SubagentError::Persistence)?;
    let converter = OpenHumanTinySubagentSessionStore::new(
        workspace_dir.to_path_buf(),
        parent_session,
        parent_thread_id.map(str::to_owned),
    );
    for row in legacy.into_iter().filter(|row| {
        row.parent_session == parent_session
            && row.parent_thread_id.as_deref() == parent_thread_id
    }) {
        let session = converter.convert(row)?;
        if driver.get(&session.subagent_session_id).is_ok_and(|found| found.is_some()) {
            continue;
        }
        // The driver's upsert API intentionally only creates a Running row;
        // import existing terminal/reusable state as its public durable shape.
        let docs = driver.document_store();
        let doc = serde_json::to_value(&session)
            .map_err(|error| SubagentError::Persistence(format!("session encode: {error}")))?;
        crate::storage::block_on(async move {
            docs.put(
                "subagent_sessions",
                &session.subagent_session_id,
                doc,
                tinystoragedrivers_core::Precondition::Absent,
            )
            .await
            .map(|_| ())
            .map_err(|error| crate::storage::StorageError::backend(error.to_string()))
        })
        .map_err(|error| SubagentError::Persistence(format!("import host sessions: {error}")))?;
    }
    Ok(())
}

/// Host compatibility seam: public orchestration tools historically accept a
/// task id as well as a durable session id. TinyAgents owns the durable store
/// and owner checks; this adapter only resolves that task-id alias in the
/// already owner-scoped row set.
struct TaskIdSessionLookup {
    inner: Arc<dyn TinySessionStore>,
    parent_session: String,
    parent_thread_id: Option<String>,
}

#[async_trait]
impl TinySessionStore for TaskIdSessionLookup {
    async fn list_for_parent(
        &self,
        parent_session: &str,
        parent_thread_id: Option<&str>,
    ) -> Result<Vec<TinySession>, SubagentError> {
        self.inner
            .list_for_parent(parent_session, parent_thread_id)
            .await
    }

    async fn get(&self, id: &str) -> Result<Option<TinySession>, SubagentError> {
        if let Some(session) = self.inner.get(id).await? {
            return Ok(Some(session));
        }
        let mut matches = self
            .inner
            .list_for_parent(&self.parent_session, self.parent_thread_id.as_deref())
            .await?
            .into_iter()
            .filter(|session| session.current_task_id.as_deref() == Some(id));
        let session = matches.next();
        if matches.next().is_some() {
            return Err(SubagentError::InvalidRequest(
                "subagent task id is ambiguous within the authenticated owner scope".into(),
            ));
        }
        Ok(session)
    }

    async fn upsert_running(
        &self,
        upsert: TinyUpsert,
    ) -> Result<(TinySession, TinyReuseDecision), SubagentError> {
        self.inner.upsert_running(upsert).await
    }

    async fn close(&self, id: &str) -> Result<bool, SubagentError> {
        self.inner.close(id).await
    }

    async fn close_if_current(
        &self,
        id: &str,
        task_id: &str,
        lifecycle_key: &tinyagents_orchestration::subagent::SubagentTaskKey,
        parent_session: &str,
        parent_thread_id: Option<&str>,
    ) -> Result<bool, SubagentError> {
        self.inner
            .close_if_current(id, task_id, lifecycle_key, parent_session, parent_thread_id)
            .await
    }

    async fn mark_finished(
        &self,
        id: &str,
        task_id: &str,
        status: TinyStatus,
        history: Vec<tinyinference_llm::message::Message>,
    ) -> Result<bool, SubagentError> {
        self.inner.mark_finished(id, task_id, status, history).await
    }

    async fn mark_finished_if_current(
        &self,
        id: &str,
        task_id: &str,
        lifecycle_key: &tinyagents_orchestration::subagent::SubagentTaskKey,
        parent_session: &str,
        parent_thread_id: Option<&str>,
        status: TinyStatus,
        history: Vec<tinyinference_llm::message::Message>,
    ) -> Result<bool, SubagentError> {
        self.inner
            .mark_finished_if_current(
                id,
                task_id,
                lifecycle_key,
                parent_session,
                parent_thread_id,
                status,
                history,
            )
            .await
    }

    async fn mark_failed(
        &self,
        id: &str,
        task_id: &str,
        error: String,
    ) -> Result<bool, SubagentError> {
        self.inner.mark_failed(id, task_id, error).await
    }

    async fn mark_failed_if_current(
        &self,
        id: &str,
        task_id: &str,
        lifecycle_key: &tinyagents_orchestration::subagent::SubagentTaskKey,
        parent_session: &str,
        parent_thread_id: Option<&str>,
        error: String,
    ) -> Result<bool, SubagentError> {
        self.inner
            .mark_failed_if_current(
                id,
                task_id,
                lifecycle_key,
                parent_session,
                parent_thread_id,
                error,
            )
            .await
    }
}

/// Routes generic Jobs/Message session operations through OpenHuman's existing
/// workspace-scoped session store. Legacy rows acquire a lifecycle key only
/// from their validated pause checkpoint; the public task id is never enough.
/// The current TinyAgents session upsert contract does not carry its driver's
/// admitted parent epoch, so these rows retain `None` and use the completion
/// router's legacy epoch-zero fencing semantics.
#[derive(Clone)]
pub(crate) struct OpenHumanTinySubagentSessionStore {
    store: SubagentSessionStore,
    checkpoint_dir: PathBuf,
    parent_session: String,
    parent_thread_id: Option<String>,
    jobs: Option<tinyagents_harness_tools::subagents::SubAgentJobRegistry>,
}

impl OpenHumanTinySubagentSessionStore {
    pub(crate) fn new(
        workspace_dir: PathBuf,
        parent_session: impl Into<String>,
        parent_thread_id: Option<String>,
    ) -> Self {
        Self {
            checkpoint_dir: workspace_dir.join(".openhuman/subagent_checkpoints"),
            store: SubagentSessionStore::new(workspace_dir),
            parent_session: parent_session.into(),
            parent_thread_id,
            jobs: None,
        }
    }

    pub(crate) fn with_jobs(
        mut self,
        jobs: tinyagents_harness_tools::subagents::SubAgentJobRegistry,
    ) -> Self {
        self.jobs = Some(jobs);
        self
    }

    fn is_owned(&self, row: &DurableSubagentSession) -> bool {
        row.parent_session == self.parent_session && row.parent_thread_id == self.parent_thread_id
    }

    fn convert(&self, row: DurableSubagentSession) -> Result<TinySession, SubagentError> {
        let lifecycle_key = match (row.lifecycle_key.clone(), row.current_task_id.as_deref()) {
            (Some(key), Some(task_id))
                if key.task_id == task_id
                    && key.thread_id.as_deref() == row.parent_thread_id.as_deref() =>
            {
                Some(key)
            }
            (Some(_), _) => None,
            (None, Some(task_id)) => super::resolve_lifecycle_key(
                &self.store,
                &row.subagent_session_id,
                &row.parent_session,
                row.parent_thread_id.as_deref(),
                &self.checkpoint_dir,
            )
            .ok()
            .filter(|key| key.task_id == task_id),
            // Keep a pre-lifecycle row visible to list/query. Its missing key
            // is not reconstructed from the raw task id, and continuation by
            // session id remains fail-closed in TinyAgents.
            (None, None) => None,
        };
        Ok(TinySession {
            subagent_session_id: row.subagent_session_id,
            parent_session: row.parent_session,
            parent_thread_id: row.parent_thread_id,
            worker_thread_id: row.worker_thread_id,
            agent_id: row.agent_id,
            display_name: row.display_name,
            model: row.model,
            sandbox_mode: row.sandbox_mode,
            action_root: row.action_root,
            task_key: row.task_key,
            lifecycle_key,
            task_title: row.task_title,
            current_task_id: row.current_task_id,
            status: match row.status {
                DurableSubagentStatus::Running => TinyStatus::Running,
                DurableSubagentStatus::Idle => TinyStatus::Idle,
                DurableSubagentStatus::AwaitingUser => TinyStatus::AwaitingUser,
                DurableSubagentStatus::Failed => TinyStatus::Failed,
                DurableSubagentStatus::Closed => TinyStatus::Closed,
            },
            reusable: row.reusable,
            latest_history: row.latest_history.map(|history| {
                history
                    .iter()
                    .map(tinyagents_runtime::transcript::transcript_to_message)
                    .collect()
            }),
            latest_error: row.latest_error,
            created_at: row.created_at,
            updated_at: row.updated_at,
            last_used_at: row.last_used_at,
        })
    }

    fn selector(selector: TinySelector) -> SubagentSessionSelector {
        SubagentSessionSelector {
            parent_session: selector.parent_session,
            parent_thread_id: selector.parent_thread_id,
            agent_id: selector.agent_id,
            model: selector.model,
            sandbox_mode: selector.sandbox_mode,
            action_root: selector.action_root,
            task_key: selector.task_key,
        }
    }
}

#[async_trait]
impl TinySessionStore for OpenHumanTinySubagentSessionStore {
    async fn list_for_parent(
        &self,
        parent_session: &str,
        parent_thread_id: Option<&str>,
    ) -> Result<Vec<TinySession>, SubagentError> {
        if parent_session != self.parent_session {
            return Ok(Vec::new());
        }
        if parent_thread_id
            .is_some_and(|requested| self.parent_thread_id.as_deref() != Some(requested))
        {
            return Ok(Vec::new());
        }
        super::list_for_parent(
            &self.store,
            parent_session,
            self.parent_thread_id.as_deref(),
        )
        .map_err(SubagentError::Persistence)?
        .into_iter()
        .filter(|row| self.is_owned(row))
        .map(|row| self.convert(row))
        .collect()
    }

    async fn get(&self, id: &str) -> Result<Option<TinySession>, SubagentError> {
        let mut owned_rows = self
            .store
            .load()
            .map_err(SubagentError::Persistence)?
            .into_iter()
            // Filter the authenticated parent and thread before matching either
            // public identifier. Raw task IDs can collide across owners, so a
            // foreign row must never shadow a matching owned row.
            .filter(|row| self.is_owned(row))
            .collect::<Vec<_>>();

        // Product callers historically pass the durable session ID, while
        // accepted spawn results also expose `current_task_id`. Prefer the
        // unambiguous session key when both happen to match.
        let row = if let Some(index) = owned_rows
            .iter()
            .position(|row| row.subagent_session_id == id)
        {
            Some(owned_rows.remove(index))
        } else {
            let mut matches = owned_rows
                .into_iter()
                .filter(|row| row.current_task_id.as_deref() == Some(id));
            let row = matches.next();
            if matches.next().is_some() {
                return Err(SubagentError::InvalidRequest(
                    "subagent task id is ambiguous within the authenticated owner scope".into(),
                ));
            }
            row
        };

        row.map(|row| self.convert(row)).transpose()
    }

    async fn upsert_running(
        &self,
        upsert: TinyUpsert,
    ) -> Result<(TinySession, TinyReuseDecision), SubagentError> {
        if upsert.selector.parent_session != self.parent_session
            || upsert.selector.parent_thread_id != self.parent_thread_id
            || upsert.lifecycle_key.thread_id != self.parent_thread_id
            || upsert.lifecycle_key.task_id != upsert.task_id
        {
            return Err(SubagentError::InvalidRequest(
                "subagent session upsert is outside its authenticated owner scope".into(),
            ));
        }
        let selector = Self::selector(upsert.selector);
        let reuse = if upsert.force_fresh {
            None
        } else {
            super::find_reusable(&self.store, &selector).map_err(SubagentError::Persistence)?
        };
        let decision = if upsert.force_fresh {
            TinyReuseDecision::ForcedFresh
        } else if let Some(session) = reuse.as_ref() {
            if session.status == DurableSubagentStatus::Running {
                TinyReuseDecision::ReusedRunning
            } else {
                TinyReuseDecision::ReusedIdle
            }
        } else {
            TinyReuseDecision::SpawnedNew
        };
        let row = super::upsert_running(
            &self.store,
            SubagentSessionUpsert {
                selector,
                display_name: upsert.display_name,
                task_title: upsert.task_title,
                worker_thread_id: upsert.worker_thread_id,
                task_id: upsert.task_id,
                lifecycle_key: Some(upsert.lifecycle_key),
                // TinyUpsert has no admitted parent epoch. Do not infer one
                // from the live router here; admission happened separately in
                // the owning driver and a later lifecycle could now be open.
                parent_epoch: None,
            },
            reuse.as_ref(),
        )
        .map_err(SubagentError::Persistence)?;
        Ok((self.convert(row)?, decision))
    }

    async fn close(&self, id: &str) -> Result<bool, SubagentError> {
        let session = self
            .store
            .load()
            .map_err(SubagentError::Persistence)?
            .into_iter()
            .find(|session| session.subagent_session_id == id && self.is_owned(session));
        let Some(session) = session else {
            return Ok(false);
        };
        if session.status == DurableSubagentStatus::Closed {
            return super::close_if_current(&self.store, &session)
                .map_err(SubagentError::Persistence);
        }
        if let Some(jobs) = &self.jobs {
            let _ = super::cancel_driver_job_for_observed_session(
                &self.store,
                &self.checkpoint_dir,
                jobs,
                &session,
                &self.parent_session,
                self.parent_thread_id.as_deref(),
            );
        }
        match super::close_if_current(&self.store, &session).map_err(SubagentError::Persistence)? {
            true => Ok(true),
            false => Err(SubagentError::InvalidRequest(
                "subagent session changed while it was being closed".into(),
            )),
        }
    }

    async fn mark_finished(
        &self,
        id: &str,
        task_id: &str,
        status: TinyStatus,
        history: Vec<tinyinference_llm::message::Message>,
    ) -> Result<bool, SubagentError> {
        if self.get(id).await?.is_none() {
            return Ok(false);
        }
        let status = match status {
            TinyStatus::Running => crate::agent::subagent_host::SubagentRunStatus::Incomplete {
                reason: "still running".into(),
                failure_cause: None,
            },
            TinyStatus::Idle => crate::agent::subagent_host::SubagentRunStatus::Completed,
            TinyStatus::AwaitingUser => {
                crate::agent::subagent_host::SubagentRunStatus::AwaitingUser {
                    question: history
                        .last()
                        .map(tinyinference_llm::message::Message::text)
                        .unwrap_or_default(),
                    options: None,
                    checkpoint: Some(self.checkpoint_dir.clone()),
                }
            }
            TinyStatus::Failed => crate::agent::subagent_host::SubagentRunStatus::Cancelled,
            TinyStatus::Closed => return self.close(id).await,
        };
        let history = history
            .iter()
            .filter_map(tinyagents_runtime::transcript::message_to_native_transcript)
            .collect();
        super::mark_finished(&self.store, id, task_id, &status, history)
            .map_err(SubagentError::Persistence)?;
        Ok(true)
    }

    async fn mark_failed(
        &self,
        id: &str,
        task_id: &str,
        error: String,
    ) -> Result<bool, SubagentError> {
        if self.get(id).await?.is_none() {
            return Ok(false);
        }
        super::mark_failed(&self.store, id, task_id, error).map_err(SubagentError::Persistence)?;
        Ok(true)
    }
}

#[cfg(test)]
#[path = "tiny_port_tests.rs"]
mod tests;
