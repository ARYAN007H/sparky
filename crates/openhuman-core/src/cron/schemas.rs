use crate::core::all::{ControllerFuture, RegisteredController};
use crate::core::runtime::DispatchCtx;
use crate::core::Outcome;
use crate::core::{ControllerSchema, FieldSchema, TypeSchema};
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
use tinyflows_bus::schedule::types::{CronJobPatch, DeliveryConfig};

mod declarations;
mod handlers;

pub use declarations::{all_controller_schemas, all_registered_controllers, schemas};

#[cfg(test)]
#[path = "schemas_tests.rs"]
mod tests;
