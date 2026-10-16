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
}
