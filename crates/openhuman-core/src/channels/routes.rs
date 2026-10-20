//! Per-sender routing and runtime command handling.

#[cfg(test)]
use super::context::clear_sender_history;
use super::context::{ChannelRouteSelection, ChannelRuntimeContext};
use super::{Channel, ChannelSendExt, SendMessage};
use crate::inference::provider;
use std::sync::Arc;
use tinychannels_bus as traits;
use tinychannels_bus::module_control::{
    ChannelControlRequest, ChannelControlResult, ProviderDescriptor,
};

fn provider_descriptors() -> Vec<ProviderDescriptor> {
    provider::list_providers()
        .into_iter()
        .map(|provider| ProviderDescriptor {
            name: provider.name.to_string(),
            aliases: provider.aliases.iter().map(ToString::to_string).collect(),
        })
        .collect()
}

fn default_route_selection(ctx: &ChannelRuntimeContext) -> ChannelRouteSelection {
    ChannelRouteSelection {
        provider: ctx.default_provider.as_str().to_string(),
        model: ctx.model.as_str().to_string(),
    }
}

pub(crate) fn get_route_selection(
    ctx: &ChannelRuntimeContext,
    sender_key: &str,
) -> ChannelRouteSelection {
    ctx.route_overrides
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(sender_key)
        .cloned()
        .unwrap_or_else(|| default_route_selection(ctx))
}

fn set_route_selection(ctx: &ChannelRuntimeContext, sender_key: &str, next: ChannelRouteSelection) {
    let default_route = default_route_selection(ctx);
    let mut routes = ctx
        .route_overrides
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if next == default_route {
        routes.remove(sender_key);
    } else {
        routes.insert(sender_key.to_string(), next);
    }
}

pub(crate) async fn get_or_create_turn_model_source(
    ctx: &ChannelRuntimeContext,
    provider_name: &str,
) -> anyhow::Result<crate::agent::tinyagents::TurnModelSource> {
    if provider_name == ctx.default_provider.as_str() {
        return ctx.turn_model_source.as_ref().cloned().ok_or_else(|| {
            anyhow::anyhow!("no injected channel model source for '{provider_name}'")
        });
    }

    if let Some(existing) = ctx
        .turn_model_source_cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(provider_name)
        .cloned()
    {
        return Ok(existing);
    }

    anyhow::bail!(
        "no injected channel model source for '{provider_name}'; production routes use crate-native model sources"
    )
}

struct RouteCallbacks {
    context: ChannelRuntimeContext,
    core_context: Option<Arc<crate::core::runtime::CoreContext>>,
    tenant: crate::core::runtime::Tenant,
    cancellation: tokio_util::sync::CancellationToken,
    reset_context: Option<crate::channels::host::remote_control::RemoteResetContext>,
}

impl RouteCallbacks {
    fn new(
        context: ChannelRuntimeContext,
        cancellation: tokio_util::sync::CancellationToken,
        reset_context: Option<crate::channels::host::remote_control::RemoteResetContext>,
    ) -> Self {
        let ctx = context.dispatch_ctx();
        Self {
            core_context: ctx.resolved(),
            tenant: crate::core::runtime::tenant::isolated_unless_tenant(
                ctx.tenant(),
                "channel_remote_control",
            ),
            context,
            cancellation,
            reset_context,
        }
    }
}

#[tinybus::interface(name = "ai.tinyhumans.tinychannels.ChannelsHost")]
impl RouteCallbacks {
    async fn dispatch_controller_backend(
        &self,
        request: tinychannels_bus::ChannelBackendRequest,
    ) -> tinybus::Result<tinychannels_bus::ChannelBackendResult> {
        match request {
            tinychannels_bus::ChannelBackendRequest::RemoteControl(request) => {
                // The captured context is the carrier; without one the host's
                // default context (none in SaaS, which fails closed).
                let dispatch_ctx = match &self.core_context {
                    Some(core) => crate::core::runtime::DispatchCtx::new(Arc::clone(core)),
                    None => crate::core::runtime::DispatchCtx::host_default(),
                };
                let dispatch = async {
                    tokio::select! {
                        biased;
                        _ = self.cancellation.cancelled() => {
                            Err(tinybus::Error::failed("channel session ended"))
                        }
                        result = super::host::remote_control::dispatch_callback(
                            &dispatch_ctx,
                            &self.context,
                            &self.tenant,
                            &self.cancellation,
                            self.reset_context.as_ref(),
                            request,
                        ) => result.map(tinychannels_bus::ChannelBackendResult::RemoteControl),
                    }
                };
                match &self.core_context {
                    Some(core_context) => {
                        crate::core::runtime::CoreContext::scope(Arc::clone(core_context), dispatch)
                            .await
                    }
                    None => dispatch.await,
                }
            }
            _ => Err(tinybus::Error::failed("channel callback unavailable")),
        }
    }
}

pub(crate) async fn handle_runtime_command_with_reset<F, Fut>(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    target_channel: Option<&Arc<dyn Channel>>,
    sender_key: &str,
    reset_scope: Option<tinychannels_bus::module_remote::RemoteSessionScope>,
    durable_session_key: Option<String>,
    reset_message: &traits::ChannelMessage,
    reset_history: F,
) -> bool
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    // This is a host dispatch hint; the module alone parses and recognizes
    // commands. Ordinary messages avoid creating a private callback object.
    if !msg.content.trim_start().starts_with('/') {
        return false;
    }
    let current = get_route_selection(ctx, sender_key);
    let history_len = durable_channel_history_len(
        ctx,
        &current,
        &msg.channel,
        sender_key,
        durable_session_key.as_deref(),
    )
    .await;
    let client = super::context::module_client(ctx).await;
    let operation = async {
        let lease = crate::modules::channel_runtime::ChannelRuntimeLease::open(
            client,
            RouteCallbacks::new(
                ctx.clone(),
                super::runtime::session::channel_session(),
                reset_scope
                    .clone()
                    .zip(durable_session_key)
                    .map(|(scope, durable_session_key)| {
                        crate::channels::host::remote_control::RemoteResetContext {
                            scope,
                            durable_session_key,
                            message: reset_message.clone(),
                            history_key: sender_key.to_owned(),
                        }
                    }),
            ),
        )
        .await?;
        let response: Result<ChannelControlResult, _> = lease
            .object()
            .call_confidential(
                tinychannels_bus::methods::PREPARE_CHANNEL_CONTROL,
                (ChannelControlRequest::Route {
                    workspace_dir: ctx.workspace_dir.to_string_lossy().into_owned(),
                    message: Box::new(msg.clone()),
                    route: tinychannels_bus::module_remote::RemoteRoute {
                        provider: current.provider,
                        model: current.model,
                    },
                    history_len,
                    session_scope: reset_scope.map(Box::new),
                    providers: provider_descriptors(),
                },),
            )
            .await;
        let close = lease.close().await;
        let response = response?;
        close?;
        Ok::<_, tinybus::module::ModuleCallError>(response)
    }
    .await;
    let (reply, next_route, clear_history) = match operation {
        Ok(ChannelControlResult::Route {
            reply,
            next_route,
            clear_history,
        }) => (reply, next_route, clear_history),
        Ok(_) => {
            crate::modules::report::malformed_reply("tinychannels");
            return true;
        }
        Err(_) => {
            if let Some(channel) = target_channel {
                let _ = channel
                    .send_with_outbound_intent(
                        &SendMessage::new("Channel module unavailable.", &msg.reply_target)
                            .in_thread(msg.thread_ts.clone()),
                    )
                    .await;
            }
            return true;
        }
    };
    if let Some(route) = next_route {
        set_route_selection(
            ctx,
            sender_key,
            ChannelRouteSelection {
                provider: route.provider,
                model: route.model,
            },
        );
    }
    if clear_history {
        if reset_history().await.is_err() {
            tracing::warn!("channel durable history reset failed");
            if let Some(channel) = target_channel {
                let copy = crate::web_chat::classify_inference_error(
                    "OpenHuman session persistence failed while resetting this conversation",
                );
                let _ = channel
                    .send_with_outbound_intent(
                        &SendMessage::new(copy.message, &msg.reply_target)
                            .in_thread(msg.thread_ts.clone()),
                    )
                    .await;
            }
            return true;
        }
    }
    let Some(reply) = reply else {
        return false;
    };
    if let Some(channel) = target_channel {
        if channel
            .send_with_outbound_intent(
                &SendMessage::new(reply, &msg.reply_target).in_thread(msg.thread_ts.clone()),
            )
            .await
            .is_err()
        {
            tracing::warn!("channel command response failed");
        }
    }
    true
}

async fn durable_channel_history_len(
    ctx: &ChannelRuntimeContext,
    route: &ChannelRouteSelection,
    channel: &str,
    history_key: &str,
    session_key: Option<&str>,
) -> usize {
    let Some(session_key) = session_key else {
        return 0;
    };
    let agent_id = match crate::channels::runtime::dispatch::host_agent::route(ctx, channel) {
        crate::channels::runtime::dispatch::host_agent::HostRoute::Agent(host) => {
            host.definition.id.clone()
        }
        crate::channels::runtime::dispatch::host_agent::HostRoute::Missing(_) => return 0,
        crate::channels::runtime::dispatch::host_agent::HostRoute::Unbound => {
            match crate::channels::runtime::dispatch::host_agent::configured_agent(ctx, route).await
            {
                Ok(host) => host.definition.id,
                Err(error) => {
                    tracing::debug!(%error, "channel history count unavailable");
                    return 0;
                }
            }
        }
    };
    let dispatch = ctx.dispatch_ctx();
    let Some(core) = dispatch.resolved() else {
        return 0;
    };
    let locator = crate::agent::session_store::transcripts_or_files_in(
        &core,
        &agent_id,
        &ctx.workspace_dir,
    );
    let session = tinyagents_session::transcript::SessionRef::scoped(session_key, &agent_id);
    let head = locator.head_generation(&session);
    let Some(read) = locator.read_session_transcript(&head) else {
        return 0;
    };
    let Ok(Some(transcript)) = read.read_session() else {
        return 0;
    };
    transcript
        .messages
        .iter()
        .filter(|message| message.role == "user" || message.role == "assistant")
        .count()
}

#[cfg(test)]
pub(crate) async fn handle_runtime_command_if_needed(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    target_channel: Option<&Arc<dyn Channel>>,
    sender_key: &str,
) -> bool {
    handle_runtime_command_with_reset(
        ctx,
        msg,
        target_channel,
        sender_key,
        None,
        None,
        msg,
        || async {
            clear_sender_history(&ctx.conversation_histories, sender_key);
            Ok(())
        },
    )
    .await
}

#[cfg(test)]
#[path = "routes_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "routes_remote_reset_tests.rs"]
mod remote_reset_tests;
