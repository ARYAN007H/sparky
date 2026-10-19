//! Configured and synthesized model catalogs for inference providers.

mod catalog_listing;
mod local_runtime;
mod openrouter;

pub use catalog_listing::{list_configured_models, list_configured_models_from_config};
pub use local_runtime::synthesize_local_runtime_entry;

#[cfg(test)]
pub(super) use catalog_listing::resolve_local_runtime_key;

#[cfg(test)]
#[path = "../models_tests.rs"]
mod tests;
