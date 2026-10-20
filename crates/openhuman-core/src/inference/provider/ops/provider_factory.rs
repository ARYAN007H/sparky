/// Fixed id for the single inference backend (OpenHuman API).
pub const INFERENCE_BACKEND_ID: &str = "openhuman";

/// Information about a supported provider for display purposes.
pub struct ProviderInfo {
    pub name: &'static str,
    pub display_name: &'static str,
    pub aliases: &'static [&'static str],
    pub local: bool,
}

/// Return known providers for display (single backend path).
pub fn list_providers() -> Vec<ProviderInfo> {
    vec![ProviderInfo {
        name: INFERENCE_BACKEND_ID,
        display_name: "OpenHuman (backend)",
        aliases: &["backend", "openhuman-backend"],
        local: false,
    }]
}

// Legacy provider alias stubs (integrations / config); remote providers were removed.
