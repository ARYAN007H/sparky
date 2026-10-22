//! TinyAgents-owned context settings re-exported through OpenHuman's config API.
//!
//! OpenHuman keeps its product-specific environment overlays in
//! [`super::load`], while the shared settings, defaults, parsing, and
//! serialization contract live with TinyAgents.

pub use tinyagents_harness::config::{CompactionSettings, CompactionStrategy, ContextConfig};
