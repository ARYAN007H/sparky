//! Captured managed transport for module-owned progressive replies.
use crate::backend::{BackendApiError, BackendClient};
use crate::modules::channel_runtime::ChannelRuntimeLease;
use crate::security::credentials::session_support::{
    BackendCredential, get_session_token, is_local_session_token,
};
#[cfg(test)]
use serde_json::Value;
use serde_json::json;
use tinybus::module::ModuleCallError;
use tinychannels_bus::module_progressive::{
    ChannelOutputEvent, ProgressiveCommand, ProgressiveEvent, ProgressiveResult,
    ProgressiveTransportRequest, ProgressiveTransportResult,
};
use tinychannels_bus::{ChannelBackendRequest, ChannelBackendResult, methods};

/// Transport refusals are typed facts; recovery and retry budgets belong to the module.
pub(super) fn map_backend_error(error: anyhow::Error) -> ProgressiveTransportResult {
    match error.downcast_ref::<BackendApiError>() {
        Some(BackendApiError::ChannelEditUnsupported { .. }) => {
            ProgressiveTransportResult::EditUnsupported
        }
        Some(BackendApiError::MessageNotFound { .. }) => ProgressiveTransportResult::MessageGone,
        _ => ProgressiveTransportResult::Failed,
    }
}

#[derive(Clone)]
struct ManagedTransport {
    client: BackendClient,
    credential: BackendCredential,
    channel: String,
    session: tokio_util::sync::CancellationToken,
}

impl ManagedTransport {
    fn capture(
        config: &crate::config::Config,
        channel: String,
        session: tokio_util::sync::CancellationToken,
    ) -> anyhow::Result<Self> {
        anyhow::ensure!(!session.is_cancelled(), "Channel session closed");
        let credential = match get_session_token(config)
            .map_err(|_| anyhow::anyhow!("Hosted channel credential unavailable"))?
        {
            Some(token) if !is_local_session_token(&token) => BackendCredential::Session(token),
            _ => anyhow::bail!("Hosted channel credential unavailable"),
        };
        Ok(Self {
            client: BackendClient::from_config(config)?,
            credential,
            channel,
            session,
        })
    }

    async fn execute(&self, request: ProgressiveTransportRequest) -> ProgressiveTransportResult {
        if request_channel(&request) != self.channel {
            return ProgressiveTransportResult::Failed;
        }
        tokio::select! {
            biased;
            _ = self.session.cancelled() => ProgressiveTransportResult::Unavailable,
            result = self.execute_transport(request) => result,
        }
    }

    async fn execute_transport(
        &self,
        request: ProgressiveTransportRequest,
    ) -> ProgressiveTransportResult {
        let result = match request {
            ProgressiveTransportRequest::Send {
                channel, message, ..
            } => {
                self.client
                    .send_channel_message(&channel, &self.credential, message)
                    .await
            }
            ProgressiveTransportRequest::Edit {
                channel,
                message_id,
                text,
                ..
            } => {
                self.client
                    .send_channel_edit(
                        &channel,
                        &message_id,
                        &self.credential,
                        json!({"text":text}),
                    )
                    .await
            }
            ProgressiveTransportRequest::Delete {
                channel,
                message_id,
                ..
            } => {
                self.client
                    .send_channel_delete(&channel, &message_id, &self.credential)
                    .await
            }
            ProgressiveTransportRequest::Typing { channel, .. } => {
                self.client
                    .send_channel_typing(&channel, &self.credential)
                    .await
            }
            // Typed sends and canonical output events are delivered by the
            // module's own adapters; the hosted relay transport has no route
            // for them yet.
            ProgressiveTransportRequest::OutputEvent { .. }
            | ProgressiveTransportRequest::SendTyped { .. } => {
                return ProgressiveTransportResult::Unavailable;
            }
        };
        match result {
            Ok(value) => ProgressiveTransportResult::Success(value),
            Err(error) => map_backend_error(error),
        }
    }
}

fn request_channel(request: &ProgressiveTransportRequest) -> &str {
    match request {
        ProgressiveTransportRequest::OutputEvent { channel, .. }
        | ProgressiveTransportRequest::Send { channel, .. }
        | ProgressiveTransportRequest::SendTyped { channel, .. }
        | ProgressiveTransportRequest::Edit { channel, .. }
        | ProgressiveTransportRequest::Delete { channel, .. }
        | ProgressiveTransportRequest::Typing { channel, .. } => channel,
    }
}

#[tinybus::interface(name = "ai.tinyhumans.tinychannels.ChannelsHost")]
impl ManagedTransport {
    async fn dispatch_controller_backend(
        &self,
        request: ChannelBackendRequest,
    ) -> tinybus::Result<ChannelBackendResult> {
        let ChannelBackendRequest::ProgressiveTransport(request) = request else {
            return Err(tinybus::Error::failed("callback capability unavailable"));
        };
        Ok(ChannelBackendResult::ProgressiveTransport(Box::new(
            self.execute(*request).await,
        )))
    }
}

/// The host projects ordered turn facts; the module owns timers and delivery state.
pub(super) struct ModuleProgressiveReply {
    runtime: ChannelRuntimeLease,
    handle: String,
}

/// Native listener output handle on the already-running compiled module
/// object. Its callback is the captured configured provider adapter, so native
/// output never falls back to the hosted REST transport.
pub(crate) struct NativeProgressiveReply {
    object: tinybus::module::ModuleObjectClient,
    handle: String,
    // Hosted relay handles own their callback runtime. Keep that lease alive
    // until Close is acknowledged or the final reply owner is dropped.
    runtime: Option<ChannelRuntimeLease>,
}

impl NativeProgressiveReply {
    pub(crate) async fn open(
        object: tinybus::module::ModuleObjectClient,
        channel: tinychannels_bus::channel::ChannelRef,
        conversation: tinychannels_bus::channel::ConversationRef,
        turn_id: String,
        lifetime: std::time::Duration,
    ) -> anyhow::Result<Self> {
        let result: ProgressiveResult = object
            .call_confidential(
                methods::EXECUTE_PROGRESSIVE,
                (ProgressiveCommand::OpenOutput {
                    channel,
                    conversation,
                    turn_id,
                    lifetime_ms: lifetime.as_millis().min(u64::MAX as u128) as u64,
                },),
            )
            .await?;
        match result {
            ProgressiveResult::Opened { handle } if !handle.is_empty() && handle.len() <= 128 => {
                Ok(Self {
                    object,
                    handle,
                    runtime: None,
                })
            }
            _ => {
                crate::modules::report::malformed_reply("tinychannels");
                Err(ModuleCallError::ModuleFault.into())
            }
        }
    }

    pub(crate) async fn open_hosted(
        runtime: ChannelRuntimeLease,
        channel: tinychannels_bus::channel::ChannelRef,
        conversation: tinychannels_bus::channel::ConversationRef,
        turn_id: String,
        lifetime: std::time::Duration,
    ) -> anyhow::Result<Self> {
        let object = runtime.object().clone();
        let result: ProgressiveResult = object
            .call_confidential(
                methods::EXECUTE_PROGRESSIVE,
                (ProgressiveCommand::OpenHostedOutput {
                    channel,
                    conversation,
                    turn_id,
                    lifetime_ms: lifetime.as_millis().min(u64::MAX as u128) as u64,
                },),
            )
            .await?;
        match result {
            ProgressiveResult::Opened { handle } if !handle.is_empty() && handle.len() <= 128 => {
                Ok(Self {
                    object,
                    handle,
                    runtime: Some(runtime),
                })
            }
            _ => {
                crate::modules::report::malformed_reply("tinychannels");
                Err(ModuleCallError::ModuleFault.into())
            }
        }
    }

    pub(crate) async fn feed(
        &self,
        events: Vec<ChannelOutputEvent>,
    ) -> Result<ProgressiveResult, ModuleCallError> {
        let result: ProgressiveResult = self
            .object
            .call_confidential(
                methods::EXECUTE_PROGRESSIVE,
                (ProgressiveCommand::FeedOutput {
                    handle: self.handle.clone(),
                    events,
                },),
            )
            .await?;
        if !matches!(
            result,
            ProgressiveResult::Updated
                | ProgressiveResult::DeliveryOutcome { .. }
                | ProgressiveResult::DeliveryOutcomes { .. }
                | ProgressiveResult::DeliveryUnsupported { .. }
                | ProgressiveResult::DeliveryBatch { .. }
        ) {
            crate::modules::report::malformed_reply("tinychannels");
            return Err(ModuleCallError::ModuleFault);
        }
        Ok(result)
    }

    /// Deliver the canonical final event and return every receipt/failure fact.
    pub(crate) async fn finish(&self, text: &str) -> Result<ProgressiveResult, ModuleCallError> {
        let result: ProgressiveResult = self
            .object
            .call_confidential(
                methods::EXECUTE_PROGRESSIVE,
                (ProgressiveCommand::FeedOutput {
                    handle: self.handle.clone(),
                    events: vec![ChannelOutputEvent::FinalMessage {
                        text: text.to_owned(),
                        parts: Vec::new(),
                        receipt: None,
                    }],
                },),
            )
            .await?;
        if !matches!(
            result,
            ProgressiveResult::DeliveryOutcome { .. }
                | ProgressiveResult::DeliveryOutcomes { .. }
                | ProgressiveResult::DeliveryUnsupported { .. }
                | ProgressiveResult::DeliveryBatch { .. }
        ) {
            crate::modules::report::malformed_reply("tinychannels");
            return Err(ModuleCallError::ModuleFault);
        }
        Ok(result)
    }

    /// Abort an unfinished reply after cancellation or a failed progress call.
    pub(crate) async fn close(&self) -> Result<(), ModuleCallError> {
        let result: ProgressiveResult = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.object.call_confidential(
                methods::EXECUTE_PROGRESSIVE,
                (ProgressiveCommand::Close {
                    handle: self.handle.clone(),
                },),
            ),
        )
        .await
        .map_err(|_| ModuleCallError::ModuleFault)??;
        if result != ProgressiveResult::Closed {
            crate::modules::report::malformed_reply("tinychannels");
            return Err(ModuleCallError::ModuleFault);
        }
        if let Some(runtime) = &self.runtime {
            runtime.close().await?;
        }
        Ok(())
    }
}

impl Drop for NativeProgressiveReply {
    fn drop(&mut self) {
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let object = self.object.clone();
        let handle = self.handle.clone();
        runtime.spawn(async move {
            let close = object.call_confidential::<ProgressiveResult>(
                methods::EXECUTE_PROGRESSIVE,
                (ProgressiveCommand::Close { handle },),
            );
            match tokio::time::timeout(std::time::Duration::from_secs(5), close).await {
                Ok(Ok(ProgressiveResult::Closed)) => {}
                Ok(Ok(_)) => crate::modules::report::malformed_reply("tinychannels"),
                Ok(Err(_)) | Err(_) => {
                    tracing::debug!("[channels] native progressive cleanup did not complete")
                }
            }
        });
    }
}

impl ModuleProgressiveReply {
    pub(super) async fn open(
        config: crate::config::Config,
        channel: String,
        session: tokio_util::sync::CancellationToken,
    ) -> anyhow::Result<Self> {
        let callback = ManagedTransport::capture(&config, channel.clone(), session)?;
        let runtime = ChannelRuntimeLease::open(crate::modules::client(config), callback).await?;
        let result: ProgressiveResult = runtime
            .object()
            .call_confidential(
                methods::EXECUTE_PROGRESSIVE,
                (ProgressiveCommand::Open { channel },),
            )
            .await?;
        match result {
            ProgressiveResult::Opened { handle } if !handle.is_empty() && handle.len() <= 128 => {
                Ok(Self { runtime, handle })
            }
            _ => {
                crate::modules::report::malformed_reply("tinychannels");
                Err(ModuleCallError::ModuleFault.into())
            }
        }
    }

    pub(super) async fn feed(&self, events: Vec<ProgressiveEvent>) -> Result<(), ModuleCallError> {
        let result: ProgressiveResult = self
            .runtime
            .object()
            .call_confidential(
                methods::EXECUTE_PROGRESSIVE,
                (ProgressiveCommand::Feed {
                    handle: self.handle.clone(),
                    events,
                },),
            )
            .await?;
        if result != ProgressiveResult::Updated {
            crate::modules::report::malformed_reply("tinychannels");
            return Err(ModuleCallError::ModuleFault);
        }
        Ok(())
    }

    pub(super) async fn finalize(&self, text: &str) -> Result<(), ModuleCallError> {
        let result: ProgressiveResult = self
            .runtime
            .object()
            .call_confidential(
                methods::EXECUTE_PROGRESSIVE,
                (ProgressiveCommand::Finish {
                    handle: self.handle.clone(),
                    text: text.into(),
                },),
            )
            .await?;
        if result != ProgressiveResult::Closed {
            crate::modules::report::malformed_reply("tinychannels");
            return Err(ModuleCallError::ModuleFault);
        }
        self.runtime.close().await
    }
}

// Compatibility fixtures exercise the owning library only in test builds.
#[cfg(test)]
pub(super) fn channel_message_body_with_idempotency(channel: &str, body: Value) -> Value {
    let intent = tinychannels::outbound_intent_from_legacy_message(channel, body);
    tinychannels::legacy_message_value_from_outbound_intent(&intent)
}

#[cfg(test)]
#[path = "delivery_tests.rs"]
mod tests;
