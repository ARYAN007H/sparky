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

#[test]
fn newly_synthesized_packed_tool_keeps_its_use_skill_route_visible() {
    let mut visible = std::collections::HashSet::new();
    let mut deferred = std::collections::HashSet::new();
    let mut permanent = std::collections::HashSet::new();
    let previous = std::collections::HashSet::new();
    let next = ["packed_delegate".to_string()].into_iter().collect();

    reconcile_surface_names(
        &mut visible,
        &mut deferred,
        &mut permanent,
        &previous,
        &next,
        true,
        None,
        |visible| {
            if visible.remove("packed_delegate") {
                visible.insert("use_skill".to_string());
            }
        },
    );

    assert!(visible.contains("use_skill"));
    assert!(!visible.contains("packed_delegate"));
}
