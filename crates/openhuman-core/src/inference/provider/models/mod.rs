//! Configured and synthesized model catalogs for inference providers.

pub(crate) mod catalog_listing;
pub(crate) mod local_runtime;
mod openrouter;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
