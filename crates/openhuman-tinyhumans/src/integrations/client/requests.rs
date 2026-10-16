//! Outbound request plumbing for [`IntegrationClient`]: route guards, egress
//! disclosure/enforcement, budget gating, and the JSON verb methods
//! (`post`/`get`/`patch`/`delete`/`upload_multipart`).

use super::construct::IntegrationClient;
use crate::client::reject_privileged_backend_path;
use openhuman_embed::__host::security::approval::ApprovalTurnContext;

pub(super) fn managed_budget_applies_to_path(path: &str) -> bool {
    path != "/agent-integrations/pricing" && path.starts_with("/agent-integrations/")
}

/// Build the egress descriptor for a managed-backend round-trip. The query
/// string is stripped so only the endpoint (the "to where") is described, never
/// any data carried in the query.
pub(super) fn backend_egress_descriptor(
    path: &str,
) -> openhuman_embed::__host::egress::EgressDescriptor {
    let endpoint = path.split('?').next().unwrap_or(path);
    openhuman_embed::__host::egress::EgressDescriptor::integration(endpoint)
}

/// Egress spine (privacy epic S2, #4436): disclose an OpenHuman managed-backend
/// round-trip before it leaves the device. Fire-and-forget — never fails the
/// caller.
pub(super) fn emit_backend_egress(approval: &ApprovalTurnContext, path: &str) {
    openhuman_embed::__host::egress::emit_external_transfer(
        approval.chat.as_ref(),
        backend_egress_descriptor(path),
    );
}

/// Local-only enforcement (privacy epic S7, #4441): refuse a managed-backend
/// round-trip that ships user data when the live policy is `LocalOnly`. Returns
/// `Ok(())` for control-plane paths (session / team / billing / integration
/// connection-management + catalog) even under `LocalOnly` — blocking those
/// would break sign-in and the Connections UI for no privacy benefit. See
/// [`is_control_plane`](openhuman_embed::__host::egress). Call before
/// [`emit_backend_egress`] so a blocked call is neither disclosed nor sent.
pub(super) fn enforce_backend_egress(path: &str) -> anyhow::Result<()> {
    openhuman_embed::__host::egress::enforce_egress(&backend_egress_descriptor(path))
}

impl IntegrationClient {
    /// Send one JSON round-trip over the shared transport, classifying a
    /// transport failure (including "no transport") for this call.
    pub(super) async fn send_json_request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&serde_json::Value>,
        method_name: &str,
        url: &str,
    ) -> anyhow::Result<serde_json::Value> {
        self.http
            .send_json(method, path, body)
            .await
            .map_err(|error| self.map_transport_error(error, method_name, path, url))
    }

    pub(super) async fn ensure_budget_available(&self, path: &str) -> anyhow::Result<()> {
        if !managed_budget_applies_to_path(path) {
            return Ok(());
        }
        if let Some(config) = &self.budget_config {
            if super::budget_gate::managed_tool_budget_exhausted(config).await {
                anyhow::bail!(
                    "Managed cloud tools are disabled because your OpenHuman AI credits are exhausted. Add credits or route the task to user-supplied providers."
                );
            }
        }
        Ok(())
    }

    /// POST JSON to a backend endpoint and parse the response `data` field.
    pub async fn post<T: serde::de::DeserializeOwned>(
        &self,
        approval: &ApprovalTurnContext,
        path: &str,
        body: &serde_json::Value,
    ) -> anyhow::Result<T> {
        self.request_json(approval, reqwest::Method::POST, path, Some(body))
            .await
    }

    /// GET from a backend endpoint and parse the response `data` field.
    pub async fn get<T: serde::de::DeserializeOwned>(
        &self,
        approval: &ApprovalTurnContext,
        path: &str,
    ) -> anyhow::Result<T> {
        self.request_json(approval, reqwest::Method::GET, path, None)
            .await
    }

    pub(super) async fn request_json<T: serde::de::DeserializeOwned>(
        &self,
        approval: &ApprovalTurnContext,
        method: reqwest::Method,
        path: &str,
        body: Option<&serde_json::Value>,
    ) -> anyhow::Result<T> {
        self.validate_credential_endpoint()?;
        reject_privileged_backend_path(method.as_str(), path)?;
        enforce_backend_egress(path)?;
        emit_backend_egress(approval, path);
        self.ensure_budget_available(path).await?;
        let url = openhuman_embed::__host::util::url::join_url(&self.backend_url, path);
        let method_name = method.as_str().to_ascii_lowercase();
        tracing::debug!("[integrations] {} {}", method.as_str(), url);
        let value = self
            .send_json_request(method, path, body, &method_name, &url)
            .await?;
        Self::parse_envelope(&method_name, path, &url, value)
    }

    /// PATCH JSON to a backend endpoint and parse the response `data` field.
    /// Mirrors [`Self::post`] (auth header, 401 → session-expiry, error
    /// classification).
    pub async fn patch<T: serde::de::DeserializeOwned>(
        &self,
        approval: &ApprovalTurnContext,
        path: &str,
        body: &serde_json::Value,
    ) -> anyhow::Result<T> {
        self.request_json(approval, reqwest::Method::PATCH, path, Some(body))
            .await
    }

    /// DELETE a backend resource and parse the response `data` field.
    /// Mirrors [`Self::post`] (auth header, 401 → session-expiry, error
    /// classification).
    pub async fn delete<T: serde::de::DeserializeOwned>(
        &self,
        approval: &ApprovalTurnContext,
        path: &str,
    ) -> anyhow::Result<T> {
        self.request_json(approval, reqwest::Method::DELETE, path, None)
            .await
    }

    /// POST a `multipart/form-data` body to a backend endpoint and parse the
    /// response `data` field. Mirrors [`Self::post`] URL building, Bearer
    /// auth, 401 → session-expiry handling and error classification; the
    /// content type is set by reqwest from the form boundary.
    pub async fn upload_multipart<T: serde::de::DeserializeOwned>(
        &self,
        approval: &ApprovalTurnContext,
        path: &str,
        form: reqwest::multipart::Form,
    ) -> anyhow::Result<T> {
        self.validate_credential_endpoint()?;
        reject_privileged_backend_path("POST", path)?;
        enforce_backend_egress(path)?;
        emit_backend_egress(approval, path);
        self.ensure_budget_available(path).await?;
        let url = openhuman_embed::__host::util::url::join_url(&self.backend_url, path);
        tracing::debug!("[integrations] POST(multipart) {}", url);

        let value = self
            .http
            .send_multipart(path, form)
            .await
            .map_err(|error| self.map_transport_error(error, "post_multipart", path, &url))?;
        // The transport unwraps successful `{success,data}` responses. Preserve
        // compatibility with endpoints that return their payload directly,
        // while still recognizing a `success:false` envelope.
        if value.get("success") == Some(&serde_json::Value::Bool(false)) {
            return Self::parse_envelope("post_multipart", path, &url, value);
        }
        Ok(serde_json::from_value(value)?)
    }
}
