//! TinyFlows' framework-neutral graph vocabulary used by OpenHuman's
//! authoring and validation adapters.
//!
//! Execution, capabilities, persistence, checkpoints, and run observation are
//! served through the TinyBus module contract. This module re-exports only the
//! generic graph model and legacy graph migration needed by host-facing APIs.

pub use ::tinyflows::{migrate, model};
