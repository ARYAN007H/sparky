//! Channel startup wiring.

mod callback;
mod chat_workload;
mod credentials;
mod delivery_import;
mod module_providers;
mod progressive_transport;
mod prompt;
#[cfg(test)]
mod relay;
mod start_channels;
mod turn_parts;

use anyhow::Result;
use tinybus::module::{ModuleCallError, ModuleClient};

#[cfg(test)]
pub(crate) type ChannelHandle = dyn tinychannels_runtime::Channel;

/// Host-owned handle to a configured channel instance. Production operations
/// are dispatched through the loaded TinyChannels module's bus object.
#[cfg(not(test))]
#[derive(Clone)]
pub(crate) enum ChannelHandle {
    Module(module_providers::ModuleChannel),
    Relay(std::sync::Arc<crate::channels::providers::relay::RelayChannel>),
}

#[cfg(not(test))]
impl ChannelHandle {
    pub(crate) fn name(&self) -> &str {
        match self {
            Self::Module(channel) => &channel.name,
            Self::Relay(channel) => channel.name(),
        }
    }

    pub(crate) fn proactive_target(&self) -> Option<String> {
        match self {
            Self::Module(channel) => channel.proactive_target(),
            Self::Relay(_) => None,
        }
    }

    pub(crate) fn capabilities(&self) -> tinychannels_bus::ChannelCapabilities {
        match self {
            Self::Module(channel) => channel.capabilities(),
            Self::Relay(_) => tinychannels_bus::ChannelCapabilities::unknown(),
        }
    }

    pub(crate) fn supports_reactions(&self) -> bool {
        match self {
            Self::Module(channel) => channel.interaction.supports_reactions,
            Self::Relay(_) => false,
        }
    }

    pub(crate) async fn send(
        &self,
        message: &tinychannels_bus::SendMessage,
    ) -> anyhow::Result<()> {
        match self {
            Self::Module(channel) => channel.send_prepared(message).await,
            Self::Relay(channel) => channel.send(message).await,
        }
    }

    pub(crate) async fn send_with_outbound_intent(
        &self,
        message: &tinychannels_bus::SendMessage,
    ) -> anyhow::Result<()> {
        self.send(message).await
    }

    pub(crate) async fn start_typing(&self, recipient: &str) -> anyhow::Result<()> {
        match self {
            Self::Module(channel) => channel
                .interact(tinychannels_bus::ChannelInteraction::StartTyping {
                    recipient: recipient.to_owned(),
                })
                .await,
            Self::Relay(_) => Ok(()),
        }
    }

    pub(crate) async fn stop_typing(&self, recipient: &str) -> anyhow::Result<()> {
        match self {
            Self::Module(channel) => channel
                .interact(tinychannels_bus::ChannelInteraction::StopTyping {
                    recipient: recipient.to_owned(),
                })
                .await,
            Self::Relay(_) => Ok(()),
        }
    }

    pub(crate) async fn begin_typing_refresh(
        &self,
        recipient: &str,
        interval_secs: u64,
    ) -> anyhow::Result<Option<String>> {
        match self {
            Self::Module(channel) => channel.begin_typing_refresh(recipient, interval_secs).await,
            Self::Relay(_) => Ok(None),
        }
    }

    pub(crate) async fn end_typing_refresh(&self, handle: &str) -> anyhow::Result<()> {
        match self {
            Self::Module(channel) => channel.end_typing_refresh(handle).await,
            Self::Relay(_) => Ok(()),
        }
    }
}

/// Ask the owning module to prepare effective execution limits for this host
/// runtime. A malformed response fails closed and reports only the module ID.
pub(crate) async fn prepare_runtime_limits(
    client: &ModuleClient,
    timeout_secs: u64,
    listener_count: usize,
) -> Result<(u64, usize)> {
    use tinychannels_bus::module_control::{ChannelControlRequest, ChannelControlResult};

    let result: ChannelControlResult = client
        .call_confidential(
            "tinychannels",
            tinychannels_bus::methods::PREPARE_CHANNEL_CONTROL,
            (ChannelControlRequest::RuntimeLimits {
                timeout_secs,
                listener_count,
            },),
        )
        .await?;
    match result {
        ChannelControlResult::RuntimeLimits {
            timeout_secs,
            max_in_flight,
        } if timeout_secs >= tinychannels_bus::MIN_CHANNEL_MESSAGE_TIMEOUT_SECS
            && max_in_flight > 0 =>
        {
            Ok((timeout_secs, max_in_flight))
        }
        _ => {
            crate::modules::report::malformed_reply("tinychannels");
            Err(anyhow::Error::new(ModuleCallError::ModuleFault)
                .context("channel runtime returned invalid runtime limits"))
        }
    }
}

pub(crate) use credentials::hydrate_channel_credentials;
pub use start_channels::start_channels;
pub(crate) use start_channels::start_channels_with_session;
pub(crate) use turn_parts::{PromptToolDescs, build_channel_turn_parts, runtime_context};

// Re-exported at module scope (rather than left as private `pub(super)`
// items reached only through their owning submodule) so the `#[path]` test
// modules below — which share this module's scope via `super::*` — can see
// them exactly as they could when this was one unsplit file.
#[cfg(test)]
use crate::config::Config;
#[cfg(test)]
use chat_workload::{ChatWorkloadResolution, resolve_chat_workload};
#[cfg(any(test, debug_assertions))]
use credentials::resolve_yuanbao_app_secret;
#[cfg(test)]
use relay::RelayInboundMessageHandler;

#[cfg(any(test, debug_assertions))]
#[path = "startup_test_support_tests.rs"]
pub mod test_support;

#[cfg(test)]
#[path = "startup_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "startup_yuanbao_secret_tests_tests.rs"]
mod yuanbao_secret_tests;

#[cfg(test)]
#[path = "startup_email_secret_tests_tests.rs"]
mod email_secret_tests;

#[cfg(all(test, feature = "modules"))]
#[path = "startup/runtime_limits_tests.rs"]
mod runtime_limits_tests;
