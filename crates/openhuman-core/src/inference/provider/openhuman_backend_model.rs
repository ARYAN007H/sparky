//! Provider-neutral pieces of the managed provider that stay in the core: the
//! label wire errors are stamped with, and the `without_reasoning` request hint.
//!
//! The managed model itself is built by the chat-model provider registered under
//! [`PROVIDER_OPENHUMAN`](super::factory::PROVIDER_OPENHUMAN)
//! ([`crate::inference::registry`]); it lives with that provider.
//!
use serde_json::Value;
use tinyinference_llm::model::ModelRequest;

pub const PROVIDER_LABEL: &str = "OpenHuman";

/// Request-metadata key a caller sets (to `true`) to ask for no reasoning on
/// a call — see [`without_reasoning`].
pub const REASONING_OFF_METADATA_KEY: &str = "openhuman_reasoning_off";

/// Mark `request` as not needing the model to reason first. Provider-neutral:
/// `ModelRequest.metadata` never reaches the wire, so a BYOK or local provider
/// ignores it. The managed provider turns it into `reasoning: { enabled:
/// false }`, which the backend forwards to OpenRouter only. For short structured
/// helper calls (follow-up suggestions) reasoning adds seconds and more tokens
/// than the answer itself, for no quality gain.
pub(crate) fn without_reasoning(mut request: ModelRequest) -> ModelRequest {
    if !request.metadata.is_object() {
        request.metadata = Value::Object(serde_json::Map::new());
    }
    if let Some(map) = request.metadata.as_object_mut() {
        map.insert(REASONING_OFF_METADATA_KEY.to_string(), Value::Bool(true));
    }
    request
}
