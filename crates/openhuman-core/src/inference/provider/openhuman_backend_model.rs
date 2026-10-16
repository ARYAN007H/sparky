//! Provider-neutral pieces of the managed provider that stay in the core: the
//! label wire errors are stamped with, and the `without_reasoning` request hint.
//!
//! The managed model itself is built by the chat-model provider registered under
//! [`PROVIDER_OPENHUMAN`](super::factory::PROVIDER_OPENHUMAN)
//! ([`crate::inference::registry`]); it lives with that provider.
//!
//! The two `is_*_managed_*` endpoint guards below are transitional: they remain
//! only until the last core callers (integrations, voice, socket URL, backend
//! client) move onto the provider crate's own copies.

use serde_json::Value;
use tinyinference_llm::model::ModelRequest;

pub const PROVIDER_LABEL: &str = "OpenHuman";

fn is_loopback_host(url: &url::Url) -> bool {
    match url.host() {
        Some(url::Host::Domain("localhost")) => true,
        Some(url::Host::Ipv4(address)) => address.is_loopback(),
        Some(url::Host::Ipv6(address)) => address.is_loopback(),
        _ => false,
    }
}

/// Whether `endpoint` is safe to carry the TinyHumans API key as a bearer.
///
/// `https://` always qualifies; plain `http://` only for loopback, matching
/// `openhuman_embed::turn::is_safe_endpoint_for_bearer`'s allowance for local
/// testing against a dev server. Anything else — a plaintext non-loopback
/// endpoint — would put the key on the wire in the clear (CWE-319), so
/// [`OpenHumanBackendModel::resolve_bearer`] refuses before it gets there.
/// Deliberately narrow to the managed-key bearer path: `normalize_api_base_url`
/// itself must stay permissive, because a library host's BYOK/local `api_url`
/// can legitimately be plain HTTP.
pub(crate) fn is_safe_endpoint_for_managed_bearer(endpoint: &str) -> bool {
    let Ok(url) = url::Url::parse(endpoint) else {
        return false;
    };
    if url.scheme() == "https" {
        return true;
    }
    if url.scheme() != "http" {
        return false;
    }
    is_loopback_host(&url)
}

/// API keys belong to TinyHumans. A loopback endpoint is permitted for local
/// development; an arbitrary TLS host is not a trusted key recipient.
pub(crate) fn is_managed_endpoint_for_api_key(endpoint: &str) -> bool {
    let Ok(url) = url::Url::parse(endpoint) else {
        return false;
    };
    if !is_safe_endpoint_for_managed_bearer(endpoint) {
        return false;
    }
    matches!(
        url.host_str(),
        Some("api.tinyhumans.ai" | "staging-api.tinyhumans.ai")
    ) && url.scheme() == "https"
        || is_loopback_host(&url)
}

/// The managed OpenHuman backend as a crate [`ChatModel`]. Holds the backend
/// connection settings (for JWT + base-URL resolution) and the default model id
}

/// Request-metadata key a caller sets (to `true`) to ask for no reasoning on
/// a call — see [`without_reasoning`].
pub const REASONING_OFF_METADATA_KEY: &str = "openhuman_reasoning_off";

/// Mark `request` as not needing the model to reason first. Provider-neutral:
/// `ModelRequest.metadata` never reaches the wire, so a BYOK or local provider
/// ignores it. The managed backend model turns it into `reasoning: { enabled:
/// false }` ([`apply_reasoning_hint`]), which the backend forwards to OpenRouter
/// only. For short structured helper calls (follow-up suggestions) reasoning
/// adds seconds and more tokens than the answer itself, for no quality gain.
pub(crate) fn without_reasoning(mut request: ModelRequest) -> ModelRequest {
    if !request.metadata.is_object() {
        request.metadata = Value::Object(serde_json::Map::new());
    }
