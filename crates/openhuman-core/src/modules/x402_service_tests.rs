//! Budget and service-identity behavior for the Wallet x402 adapter.

use super::*;

#[test]
fn budget_defaults_and_environment_overrides_match_the_host_contract() {
    let defaults = budget_from(|_| None);
    assert_eq!(defaults, (1_000_000, 10_000_000, 100_000_000));

    let configured = budget_from(|name| match name {
        "OPENHUMAN_X402_PER_REQUEST_MAX" => Some("2500".to_string()),
        "OPENHUMAN_X402_DAILY_MAX" => Some("not-a-number".to_string()),
        "OPENHUMAN_X402_MONTHLY_MAX" => Some("9000000".to_string()),
        _ => None,
    });
    assert_eq!(configured, (2_500, 10_000_000, 9_000_000));
}

#[test]
fn service_reuse_requires_the_same_workspace_and_budgets() {
    let config = X402ConfigResponse {
        workspace_dir: "/workspace/one".to_string(),
        session_id: "x402-session".to_string(),
        per_request_max_atomic: 1,
        daily_max_atomic: 2,
        monthly_max_atomic: 3,
    };
    assert!(same_budget(&config, (1, 2, 3)));
    assert!(!same_budget(&config, (1, 2, 4)));
}
