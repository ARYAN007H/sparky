mod builder;
mod definition;
mod draft;
mod run;

pub(super) use builder::*;
pub(super) use definition::*;
pub(super) use draft::*;
pub(super) use run::*;

use crate::core::Outcome;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

pub(super) fn read_required<T: DeserializeOwned>(
    params: &Map<String, Value>,
    key: &str,
) -> Result<T, String> {
    let value = params
        .get(key)
        .cloned()
        .ok_or_else(|| format!("missing required param '{key}'"))?;
    serde_json::from_value(value).map_err(|e| format!("invalid '{key}': {e}"))
}

pub(super) fn to_json<T: serde::Serialize>(outcome: Outcome<T>) -> Result<Value, String> {
    outcome.into_cli_compatible_json()
}
