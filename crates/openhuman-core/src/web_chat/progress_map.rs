//! Compatibility view of the shared agent progress projection.
//!
//! Web progress retains its surface-specific lifecycle and sub-agent handling,
//! while the event constructors live in the agent domain so native channels
//! use the same payload caps and structured tool failures.

pub(crate) use crate::agent::turn_progress::*;

#[cfg(test)]
use serde_json::Value;
#[cfg(test)]
use tinyagents_harness::progress::AgentProgress;
#[cfg(test)]
use tinychannels_bus::module_progressive::{ChannelOutputEvent, SubagentOutput};

#[cfg(test)]
#[path = "progress_map_tests.rs"]
mod tests;
