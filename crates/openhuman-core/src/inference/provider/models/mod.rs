//! Configured and synthesized model catalogs for inference providers.

pub(crate) mod catalog_listing;
pub(crate) mod local_runtime;
mod openrouter;

#[cfg(test)]
pub(super) use catalog_listing::resolve_local_runtime_key;

#[cfg(test)]
#[path = "../models_tests.rs"]
mod tests;
