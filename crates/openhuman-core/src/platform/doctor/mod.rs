//! Diagnostic checks for OpenHuman configuration, workspace health, and daemon state.

mod core;
pub mod ops;
mod schemas;
pub mod tools;

pub use core::*;
pub use ops as rpc;
pub use ops::*;
pub(crate) use schemas::all_controller_entries as all_doctor_controller_entries;
pub use schemas::{
    all_controller_schemas as all_doctor_controller_schemas,
    all_registered_controllers as all_doctor_registered_controllers,
};

#[cfg(test)]
pub(crate) use schemas::doctor_parameter_contracts;
