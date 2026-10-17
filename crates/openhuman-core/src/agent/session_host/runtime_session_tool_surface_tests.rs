use super::*;

#[test]
fn permanent_tools_survive_packed_and_deferred_filters_but_revocation_wins() {
    let mut visible = [
        "packed_permanent".to_string(),
        "deferred_permanent".to_string(),
        "ordinary".to_string(),
        "revoked_permanent".to_string(),
    ]
    .into_iter()
    .collect::<std::collections::HashSet<_>>();
    let mut deferred = std::collections::HashSet::new();
    let mut permanent = [
        "packed_permanent".to_string(),
        "deferred_permanent".to_string(),
        "revoked_permanent".to_string(),
    ]
    .into_iter()
    .collect::<std::collections::HashSet<_>>();
    let previous = ["revoked_permanent".to_string()]
        .into_iter()
        .collect::<std::collections::HashSet<_>>();
    let next = std::collections::HashSet::new();
    let discovery_deferred = ["deferred_permanent".to_string(), "ordinary".to_string()]
        .into_iter()
        .collect();

    reconcile_surface_names(
        &mut visible,
        &mut deferred,
        &mut permanent,
        &previous,
        &next,
        false,
        Some(discovery_deferred),
        |visible| {
            visible.remove("packed_permanent");
        },
    );

    assert!(visible.contains("packed_permanent"));
    assert!(visible.contains("deferred_permanent"));
    assert!(!visible.contains("ordinary"));
    assert!(!visible.contains("revoked_permanent"));
    assert!(!deferred.contains("deferred_permanent"));
    assert!(deferred.contains("ordinary"));
    assert!(!permanent.contains("revoked_permanent"));
}
