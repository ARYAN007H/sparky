use super::*;

#[test]
fn summary_response_preserves_legacy_camel_case_rpc_shape() {
    let value = summary_value(tinywallet_bus::wire::X402DetailedSpendingSummary {
        session_total_atomic: 11,
        daily_total_atomic: 12,
        monthly_total_atomic: 13,
        session_count: 2,
        daily_count: 3,
        monthly_count: 4,
        reserved_atomic: 99,
        per_request_max_atomic: 100,
        daily_max_atomic: 200,
        monthly_max_atomic: 300,
    });

    assert_eq!(
        value,
        json!({
            "summary": {
                "sessionTotalAtomic": 11,
                "dailyTotalAtomic": 12,
                "monthlyTotalAtomic": 13,
                "sessionCount": 2,
                "dailyCount": 3,
                "monthlyCount": 4,
            },
            "budget": {
                "perRequestMaxAtomic": 100,
                "dailyMaxAtomic": 200,
                "monthlyMaxAtomic": 300,
            },
        })
    );
}
