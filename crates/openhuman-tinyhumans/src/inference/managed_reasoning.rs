//! OpenHuman's provider-neutral hint for a managed call that disables reasoning.

use serde_json::Value;
use tinyinference_llm::model::ModelRequest;

pub(crate) const REASONING_OFF_METADATA_KEY: &str = "openhuman_reasoning_off";

pub(crate) fn without_reasoning(mut request: ModelRequest) -> ModelRequest {
    if !request.metadata.is_object() {
        request.metadata = Value::Object(serde_json::Map::new());
    }
    if let Some(map) = request.metadata.as_object_mut() {
        map.insert(REASONING_OFF_METADATA_KEY.to_string(), Value::Bool(true));
    }
    request
}

#[cfg(test)]
#[path = "managed_reasoning_tests.rs"]
mod tests;
