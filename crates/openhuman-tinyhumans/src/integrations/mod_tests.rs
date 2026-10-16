//! Tests for the integrations module root (types + client construction).

use super::*;
#[test]
fn backend_response_deserializes() {
    let json = r#"{"success": true, "data": {"foo": 42}}"#;
    let resp: BackendResponse<serde_json::Value> = serde_json::from_str(json).unwrap();
    assert!(resp.success);
    assert_eq!(resp.data.unwrap()["foo"], 42);
}

#[test]
fn backend_response_without_data() {
    let json = r#"{"success": true}"#;
    let resp: BackendResponse<serde_json::Value> = serde_json::from_str(json).unwrap();
    assert!(resp.success);
    assert!(resp.data.is_none());
}

#[test]
fn integration_pricing_defaults_on_missing_fields() {
    let json = r#"{"integrations": {}}"#;
    let pricing: IntegrationPricing = serde_json::from_str(json).unwrap();
    assert!(pricing.integrations.google_places.is_none());
    assert!(pricing.integrations.parallel.is_none());
    assert!(pricing.integrations.tinyfish.is_none());
}

#[test]
fn integration_pricing_ignores_retired_apify_and_twilio_entries() {
    // An older backend still reporting the retired integrations must not
    // break pricing deserialization.
    let json = r#"{"integrations": {
        "apify": {"available": true, "pricing": {}},
        "twilio": {"available": true, "pricing": {}},
        "parallel": {"available": true, "pricing": {}}
    }}"#;
    let pricing: IntegrationPricing = serde_json::from_str(json).unwrap();
    assert!(pricing.integrations.parallel.unwrap().available);
}

#[test]
fn build_client_returns_none_when_no_auth_token() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let config = openhuman_embed::__host::config::Config {
        workspace_dir: tmp.path().join("workspace"),
        action_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("config.toml"),
        ..openhuman_embed::__host::config::Config::default()
    };
    assert!(build_client(&config).is_none());
}
