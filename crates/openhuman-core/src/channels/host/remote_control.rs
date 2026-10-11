//! Captured host conversation and session state supplied to the channel module.

use crate::channels::context::{ChannelRuntimeContext, clear_sender_history};
use crate::threads::store::{self as conversations, CreateConversationThread};

/// Serves product-owned conversation state to the captured module runtime.
pub(crate) async fn dispatch_callback(
    ctx: &ChannelRuntimeContext,
    request: tinychannels_bus::module_remote::RemoteHostRequest,
) -> tinybus::Result<tinychannels_bus::module_remote::RemoteHostResult> {
    use tinychannels_bus::module_remote::{
        RemoteHostRequest as Request, RemoteHostResult as Response,
    };
    match request {
        Request::ListThreads => {
            let threads = conversations::list_threads(ctx.workspace_dir.to_path_buf())
                .map_err(|_| tinybus::Error::failed("conversation state unavailable"))?;
            Ok(Response::Threads(
                threads
                    .into_iter()
                    .map(
                        |thread| tinychannels_bus::module_remote::RemoteThreadSummary {
                            id: thread.id,
                            title: thread.title,
                            message_count: thread.message_count,
                            last_message_at: thread.last_message_at,
                        },
                    )
                    .collect(),
            ))
        }
        Request::CreateThread(thread) => {
            conversations::ensure_thread(
                ctx.workspace_dir.to_path_buf(),
                CreateConversationThread {
                    id: thread.id,
                    title: thread.title,
                    created_at: thread.created_at,
                    parent_thread_id: None,
                    labels: Some(thread.labels),
                    personality_id: None,
                    working_dir: None,
                },
            )
            .map_err(|_| tinybus::Error::failed("conversation state unavailable"))?;
            Ok(Response::Completed)
        }
        Request::ClearHistory { sender_key } => {
            clear_sender_history(ctx, &sender_key);
            Ok(Response::Completed)
        }
        Request::InvalidateThread { thread_id } => {
            crate::web_chat::invalidate_thread_sessions(&thread_id).await;
            Ok(Response::Completed)
        }
    }
}
