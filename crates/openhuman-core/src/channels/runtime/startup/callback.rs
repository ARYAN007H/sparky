//! Identity-scoped implementation of TinyChannels' host callbacks (durable state goes through the storage bus).

use crate::channels::context::ChannelRuntimeContext;
use crate::channels::runtime::dispatch::RuntimeChannelMessage;
use crate::config::Config;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, Weak};
use tinybus::{Error, Result};
use tinychannels_bus::module_progressive::{
    ProgressiveTransportRequest, ProgressiveTransportResult,
};
use tinychannels_bus::{
    ChannelBackendRequest, ChannelBackendResult, HostEventPublication, HostSpeechSynthesisRequest,
    HostSpeechSynthesisResult, HostTranscriptionRequest, HostTranscriptionResult,
    HostVoiceReplyPolicy, HostVoiceReplyPolicyRequest, HostVoiceTranscriptionRequest,
};
use tokio::sync::mpsc;

const MAX_CALLBACK_BYTES: usize = 128 * 1024;
const MAX_TRANSCRIPTION_BYTES: usize = 12 * 1024 * 1024;
const MAX_CONTROLLER_BYTES: usize = 1024 * 1024;

#[async_trait::async_trait]
pub(super) trait ProgressiveChannelTransport: Send + Sync {
    fn configured_name(&self) -> &str;

    async fn dispatch_progressive(
        &self,
        request: ProgressiveTransportRequest,
    ) -> Result<ProgressiveTransportResult>;
}

/// Callbacks capture one immutable host config and a weak runtime context.
/// The weak reference avoids a cycle from runtime context -> module channel ->
/// lease -> TinyBus connection -> callback -> runtime context.
pub(super) struct ChannelHostCallbacks {
    config: Arc<Config>,
    core_context: Option<Arc<crate::core::runtime::CoreContext>>,
    tenant: crate::core::runtime::Tenant,
    cancellation: tokio_util::sync::CancellationToken,
    context: Arc<OnceLock<Weak<ChannelRuntimeContext>>>,
    progressive_channels: Arc<OnceLock<Arc<HashMap<String, Arc<dyn ProgressiveChannelTransport>>>>>,
    dispatch: mpsc::Sender<RuntimeChannelMessage>,
}

impl ChannelHostCallbacks {
    pub(super) fn new(
        ctx: &crate::core::runtime::DispatchCtx,
        config: Arc<Config>,
        context: Arc<OnceLock<Weak<ChannelRuntimeContext>>>,
        progressive_channels: Arc<
            OnceLock<Arc<HashMap<String, Arc<dyn ProgressiveChannelTransport>>>>,
        >,
        dispatch: mpsc::Sender<RuntimeChannelMessage>,
    ) -> Self {
        Self {
            config,
            core_context: ctx.resolved(),
            tenant: crate::core::runtime::tenant::isolated_unless_tenant(
                ctx.tenant(),
                "channel_host_callback",
            ),
            cancellation: crate::channels::runtime::session::channel_session(),
            context,
            progressive_channels,
            dispatch,
        }
    }

    fn validate_callback_size<T: serde::Serialize>(value: &T) -> Result<()> {
        if serde_json::to_vec(value).map_or(true, |bytes| bytes.len() > MAX_CALLBACK_BYTES) {
            return Err(Error::failed("host callback payload exceeds its limit"));
        }
        Ok(())
    }

    fn voice(&self) -> crate::channels::host::voice::ChannelVoiceHost {
        crate::channels::host::voice::ChannelVoiceHost {
            config: Arc::clone(&self.config),
        }
    }

    fn context(&self) -> Result<Arc<ChannelRuntimeContext>> {
        self.context
            .get()
            .and_then(Weak::upgrade)
            .ok_or_else(|| Error::failed("channel runtime is stopping"))
    }
}

#[cfg(test)]
#[path = "callback_tests.rs"]
mod tests;

#[tinybus::interface(name = "ai.tinyhumans.tinychannels.ChannelsHost")]
impl ChannelHostCallbacks {
    async fn deliver_inbound(&self, channel: String, message: serde_json::Value) -> Result<()> {
        Self::validate_callback_size(&message)?;
        if let Ok(delivery) = serde_json::from_value::<
            tinychannels_bus::module_relay::RelayInboundDelivery,
        >(message.clone())
        {
            if delivery.envelope.channel.id != channel {
                return Err(Error::failed(
                    "relay callback channel does not match envelope",
                ));
            }
            self.dispatch
                .send(RuntimeChannelMessage::from_authenticated_relay(delivery))
                .await
                .map_err(|_| Error::failed("channel dispatch is closed"))?;
            return Ok(());
        }

        let message: tinychannels_bus::ChannelMessage = serde_json::from_value(message)
            .map_err(|_| Error::failed("invalid inbound message"))?;
        if message.channel != channel {
            return Err(Error::failed("callback channel does not match message"));
        }
        self.dispatch
            .send(RuntimeChannelMessage::from(message))
            .await
            .map_err(|_| Error::failed("channel dispatch is closed"))
    }

    async fn report_status(
        &self,
        channel: String,
        state: String,
        _detail: Option<String>,
    ) -> Result<()> {
        if channel.is_empty() || channel.len() > 256 || state.len() > 128 {
            return Err(Error::failed("channel status is invalid"));
        }
        match state.as_str() {
            "connected" => crate::core::bus::BUS
                .publish(crate::core::events::DomainEvent::ChannelConnected { channel }),
            "disconnected" | "stopped" => crate::core::bus::BUS.publish(
                crate::core::events::DomainEvent::ChannelDisconnected {
                    channel,
                    reason: state,
                },
            ),
            // `connecting`, `reconnecting`, and `unhealthy` are health facts,
            // not user-facing lifecycle events.
            _ => {}
        }
        Ok(())
    }

    async fn persist_allowed_identity(&self, channel: String, identity: String) -> Result<()> {
        crate::channels::host::ConfigAllowlistStore
            .persist_allowed_identity(&channel, &identity)
            .await
            .map_err(|_| Error::failed("channel identity persistence failed"))
    }

    async fn transcribe(
        &self,
        request: HostTranscriptionRequest,
    ) -> Result<HostTranscriptionResult> {
        if serde_json::to_vec(&request).map_or(true, |bytes| bytes.len() > MAX_TRANSCRIPTION_BYTES)
        {
            return Err(Error::failed("transcription callback exceeds its limit"));
        }
        self.voice()
            .transcribe_host(request)
            .await
            .map_err(|_| Error::failed("channel transcription failed"))
    }

    async fn transcribe_voice(
        &self,
        request: HostVoiceTranscriptionRequest,
    ) -> Result<HostTranscriptionResult> {
        if serde_json::to_vec(&request).map_or(true, |bytes| bytes.len() > MAX_TRANSCRIPTION_BYTES)
        {
            return Err(Error::failed("transcription callback exceeds its limit"));
        }
        self.voice()
            .transcribe_voice(request)
            .await
            .map_err(|_| Error::failed("channel transcription failed"))
    }

    async fn synthesize_speech(
        &self,
        request: HostSpeechSynthesisRequest,
    ) -> Result<HostSpeechSynthesisResult> {
        Self::validate_callback_size(&request)?;
        self.voice()
            .synthesize_speech(request)
            .await
            .map_err(|_| Error::failed("channel speech synthesis failed"))
    }

    async fn get_voice_reply_policy(
        &self,
        request: HostVoiceReplyPolicyRequest,
    ) -> Result<HostVoiceReplyPolicy> {
        Self::validate_callback_size(&request)?;
        Ok(self.voice().reply_policy(&request.scope))
    }

    async fn publish_event(&self, publication: HostEventPublication) -> Result<()> {
        Self::validate_callback_size(&publication)?;
        crate::channels::host::OpenHumanEventSink
            .publish_host(&publication.domain, &publication.kind, publication.payload)
            .await
            .map_err(|_| Error::failed("channel event publication failed"))
    }

    async fn dispatch_controller_backend(
        &self,
        request: ChannelBackendRequest,
    ) -> Result<ChannelBackendResult> {
        if serde_json::to_vec(&request).map_or(true, |bytes| bytes.len() > MAX_CONTROLLER_BYTES) {
            return Err(Error::failed("controller callback exceeds its limit"));
        }
        match request {
            ChannelBackendRequest::RemoteControl(request) => {
                let context = self.context()?;
                let tenant = &self.tenant;
                let cancellation = &self.cancellation;
                // The captured context is the carrier; without one the host's
                // default context (none in SaaS, which fails closed).
                let dispatch_ctx = match &self.core_context {
                    Some(core) => crate::core::runtime::DispatchCtx::new(Arc::clone(core)),
                    None => crate::core::runtime::DispatchCtx::host_default(),
                };
                let dispatch = async {
                    tokio::select! {
                        biased;
                        _ = cancellation.cancelled() => {
                            Err(Error::failed("channel session ended"))
                        }
                        result = crate::channels::host::remote_control::dispatch_callback(
                            &dispatch_ctx,
                            context.as_ref(),
                            tenant,
                            cancellation,
                            request,
                        ) => result.map(ChannelBackendResult::RemoteControl),
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
            ChannelBackendRequest::ProgressiveTransport(request) => self
                .dispatch_progressive_transport(*request)
                .await
                .map(|result| ChannelBackendResult::ProgressiveTransport(Box::new(result))),
            request => {
                crate::channels::controllers::OpenHumanChannelBackend::new((*self.config).clone())
                    .dispatch_callback(request)
                    .await
                    .map_err(|_| Error::failed("channel controller callback failed"))
            }
        }
    }

    async fn dispatch_progressive_transport(
        &self,
        request: ProgressiveTransportRequest,
    ) -> Result<ProgressiveTransportResult> {
        if let ProgressiveTransportRequest::OutputEvent {
            channel_ref: Some(channel),
            conversation: Some(conversation),
            turn_id: Some(turn_id),
            sequence,
            event,
            ..
        } = &request
        {
            let event_kind = serde_json::to_value(event.as_ref())
                .ok()
                .and_then(|value| {
                    value
                        .get("type")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned)
                })
                .ok_or_else(|| Error::failed("invalid progressive output event"))?;
            let publication = crate::core::events::DomainEvent::ChannelOutputObserved {
                channel: channel.clone(),
                conversation: conversation.clone(),
                turn_id: turn_id.clone(),
                sequence: *sequence,
                event_kind,
            };
            let publish = async {
                crate::core::bus::BUS.publish(publication);
                Ok(ProgressiveTransportResult::Acknowledged)
            };
            return match &self.core_context {
                Some(core_context) => {
                    crate::core::runtime::CoreContext::scope(Arc::clone(core_context), publish)
                        .await
                }
                None => publish.await,
            };
        }
        let (channel, channel_ref) = match &request {
            ProgressiveTransportRequest::OutputEvent {
                channel,
                channel_ref,
                ..
            }
            | ProgressiveTransportRequest::Send {
                channel,
                channel_ref,
                ..
            }
            | ProgressiveTransportRequest::SendTyped {
                channel,
                channel_ref,
                ..
            }
            | ProgressiveTransportRequest::Edit {
                channel,
                channel_ref,
                ..
            }
            | ProgressiveTransportRequest::Delete {
                channel,
                channel_ref,
                ..
            }
            | ProgressiveTransportRequest::Typing {
                channel,
                channel_ref,
                ..
            } => (channel.as_str(), channel_ref.as_ref()),
        };
        self.progressive_channel(channel, channel_ref)?
            .dispatch_progressive(request)
            .await
    }

    fn progressive_channel(
        &self,
        channel: &str,
        channel_ref: Option<&tinychannels_bus::ChannelRef>,
    ) -> Result<Arc<dyn ProgressiveChannelTransport>> {
        let reference = channel_ref.ok_or_else(|| Error::failed("channel account is required"))?;
        let account = reference
            .account_id
            .as_deref()
            .ok_or_else(|| Error::failed("configured channel account is required"))?;
        let adapters = self
            .progressive_channels
            .get()
            .ok_or_else(|| Error::failed("progressive channel adapters are unavailable"))?;
        let adapter = adapters
            .get(account)
            .cloned()
            .ok_or_else(|| Error::failed("configured channel account is unavailable"))?;
        let configured_provider =
            tinychannels_bus::capabilities::provider_id(adapter.configured_name());
        if configured_provider != reference.id
            || tinychannels_bus::capabilities::provider_id(channel) != reference.id
        {
            return Err(Error::failed(
                "configured channel account does not match request",
            ));
        }
        Ok(adapter)
    }
}
