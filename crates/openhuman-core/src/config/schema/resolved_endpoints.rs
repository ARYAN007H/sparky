//! Runtime-resolved service endpoints that a host fills in after a config load.
//!
//! The config file holds what the *user* chose. Some values the core needs are
//! not the user's to choose: the control-plane origin a host talks to, the
//! socket and tracing-ingestion URLs it derives, the attribution headers it
//! stamps on its requests, the product it reports as. A host that has such
//! values registers a config enricher
//! ([`register_enricher`](crate::config::register_enricher)); the loader runs it
//! on every load and it writes them here.
//!
//! The core reads these fields instead of owning the values. Like
//! [`EphemeralRoute`](super::ephemeral_route::EphemeralRoute) the field is
//! `#[serde(skip)]`: it never reaches `config.toml`, and nothing a caller
//! saves can persist it. With no enricher registered every field is empty and
//! the core behaves as it does with no hosted connection.

/// Endpoints and identity resolved by a registered config enricher. Every
/// field is `None`/empty until an enricher fills it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolvedEndpoints {
    /// The origin of the host's control plane (REST API base URL).
    pub control_plane_url: Option<String>,
    /// The origin inference requests go to, when it differs from the control
    /// plane.
    pub inference_url: Option<String>,
    /// The realtime socket URL.
    pub socket_url: Option<String>,
    /// Where tracing/observability events are pushed.
    pub tracing_ingest_url: Option<String>,
    /// Headers stamped on every request to the control plane, `(name, value)`.
    pub attribution_headers: Vec<(String, String)>,
    /// The product identity the host reports (the `x-sdk-name` value).
    pub product_identity: Option<String>,
}

impl ResolvedEndpoints {
    /// Whether an enricher has filled nothing in.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The control-plane origin, or the `BACKEND_UNAVAILABLE:` sentinel
    /// `core::observability` demotes when no enricher resolved one, for
    /// callers on the JSON-RPC `String` error channel.
    pub fn require_control_plane(&self) -> Result<&str, String> {
        self.control_plane_url.as_deref().ok_or_else(|| {
            format!(
                "{} no control plane resolved",
                crate::core::observability::BACKEND_UNAVAILABLE_PREFIX
            )
        })
    }

    /// The inference origin: the resolved one, else the user's explicit
    /// `api_url` override (normalized), else `None`.
    pub fn inference_or_configured(&self, api_url: &Option<String>) -> Option<String> {
        self.inference_url.clone().or_else(|| {
            api_url
                .as_deref()
                .map(str::trim)
                .filter(|u| !u.is_empty())
                .map(crate::util::url::normalize_api_base_url)
        })
    }

    /// The attribution headers as a `reqwest` header map; invalid pairs are
    /// skipped.
    pub fn attribution_header_map(&self) -> reqwest::header::HeaderMap {
        let mut map = reqwest::header::HeaderMap::new();
        for (name, value) in &self.attribution_headers {
            if let (Ok(n), Ok(v)) = (
                reqwest::header::HeaderName::from_bytes(name.as_bytes()),
                reqwest::header::HeaderValue::from_str(value),
            ) {
                map.insert(n, v);
            }
        }
        map
    }
}
