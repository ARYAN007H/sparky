use super::support::envelope;
use crate::core::runtime::DispatchCtx;
use crate::core::Outcome;
use crate::threads::store as conversations;
use crate::threads::{
    ApiEnvelope, DeleteConversationThreadRequest, DeleteConversationThreadResponse,
};
use std::path::PathBuf;

pub(super) async fn thread_delete_inner(
    ctx: DispatchCtx,
    config: crate::config::Config,
    request: DeleteConversationThreadRequest,
) -> Result<Outcome<ApiEnvelope<DeleteConversationThreadResponse>>, String> {
    let dir: PathBuf = config.workspace_dir.clone();
    let deleted = conversations::blocking::delete_thread(
        ctx.clone(),
        dir.clone(),
        request.thread_id.clone(),
        request.deleted_at.clone(),
    )
    .await?;
    // Invalidate the in-process web-channel session BEFORE the
    // turn-state cleanup. The snapshot deletion is fallible and
    // returns early on error; if invalidation ran after, an active
    // session for the now-deleted thread could linger and try to
    // append to a thread index row that no longer exists.
    crate::web_chat::invalidate_thread_sessions(&ctx, &request.thread_id).await;
    // Cancel any detached sub-agents this thread spawned BEFORE clearing their
    // queued results: abort the in-flight ones first so a child can't record a
    // completion in the gap between the two calls, then discard anything already
    // queued for delivery. Both target a thread that's being deleted, so there's
    // nowhere left to deliver to — abort + cleanup is the whole behavior.
    let cancelled =
        crate::agent::orchestration::running_subagents::cancel_for_thread(&ctx, &request.thread_id);
    let discarded = crate::agent::orchestration::background_completions::discard_for_thread(
        &ctx,
        &dir,
        &request.thread_id,
    );
    log::debug!(
        "[threads] thread_delete thread_id={} cancelled_subagents={} discarded_completions={}",
        request.thread_id,
        cancelled,
        discarded
    );
    // Drop any persisted in-flight turn snapshot for this thread —
    // otherwise `threads_turn_state_list` keeps surfacing it (as
    // `Interrupted` on next restart) for a thread that no longer
    // exists. Failure here is surfaced as an RPC error so callers
    // can't observe a thread "deleted" while its snapshot (which
    // mirrors conversation-derived state) remains on disk; the thread row
    // itself is already gone at this point so the caller sees a partial
    // failure they can act on instead of silent drift.
    crate::agent::session_store::turn_states_for(&dir)
        .delete(&request.thread_id)
        .map_err(|err| {
            format!(
                "thread {} deleted but turn-snapshot cleanup failed: {err}",
                request.thread_id
            )
        })?;
    // The thread's conversation memory goes too, for good (by `memory_ids`).
    // Memory off (signed out) or a failed forget queues the deletion for the
    // next sign-in (`memory::deletion`) rather than failing the delete: the
    // thread itself is already gone.
    let forgotten = crate::memory::deletion::forget_thread(&config, &request.thread_id).await;
    log::debug!(
        "[threads] thread_delete thread_id={} memory_items_forgotten={}",
        request.thread_id,
        forgotten
    );
    Ok(envelope(
        DeleteConversationThreadResponse { deleted },
        None,
        None,
    ))
}
