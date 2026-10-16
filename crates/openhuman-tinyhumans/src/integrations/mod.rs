//! Hosted integrations: the integrations client, budget gate, file storage,
//! media generation, web3 crypto client and the Composio backend route.
//!
//! This code used to live in `openhuman-core` (`integrations/client`,
//! `integrations/tools`, `integrations/file_storage`, `media/generation/provider`,
//! `web3/client`) next to the hosted routes it calls. It plugs back into the
//! core only through the generic registries on `openhuman_embed::__host`:
//! [`tool_sources`](openhuman_embed::__host::tool_sources) for the agent tools,
//! [`composio`](openhuman_embed::__host::composio) for the backend route, and
//! the module wallet-backend hook for the web3 crypto client. [`register`] is
//! the one place it does so; `crate::register::register_all` calls it from
//! `install()`.

pub mod client;
pub mod file_storage;
pub mod media;
pub mod tools;
pub mod types;
pub mod web3;

pub use client::{build_client, pricing_for_config, IntegrationClient};
pub use types::{
    BackendResponse, IntegrationPricing, IntegrationPricingEntry, PricingIntegrations,
};

use crate::install::InstallOptions;

/// Registers this area's hooks with the core. Idempotent.
pub(crate) fn register(_options: &InstallOptions) {
    log::trace!("[tinyhumans] integrations: nothing to register yet");
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
