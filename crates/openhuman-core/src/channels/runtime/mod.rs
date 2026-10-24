//! Channel runtime entry points.

pub(crate) mod dispatch;
pub(crate) mod session;
mod startup;
pub(crate) mod turn_driver;

#[cfg(test)]
pub(crate) use startup::prepare_module_runtime_for_test_in_context;
pub(crate) use startup::prepare_runtime_limits;
pub use startup::start_channels;
pub(crate) use startup::start_channels_with_session;
pub(crate) use startup::ChannelHandle;
#[cfg(test)]
pub(crate) use startup::TestChannel;
#[cfg(test)]
pub(crate) use startup::{prepare_module_runtime_for_test, ModuleRuntimeTestGuard};
// The hosted-channel relay (`channels::providers::relay`) runs one message
// through the same pipeline, with a context built from the caller's config.
#[cfg(test)]
pub(crate) use dispatch::process_channel_message;
pub(crate) use dispatch::{process_channel_runtime_message, RuntimeChannelMessage};
pub(crate) use startup::{build_channel_turn_parts, runtime_context, PromptToolDescs};

#[cfg(test)]
pub mod test_support;

// Re-exported for `channels::tests` only; omit in normal lib builds to avoid unused-import warnings.
#[cfg(test)]
pub(crate) use dispatch::run_message_dispatch_loop;
