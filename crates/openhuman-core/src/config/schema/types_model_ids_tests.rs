use super::WORKLOAD_ROLES;

#[test]
fn workload_roles_follow_the_native_workload_route_order() {
    let native = tinyinference_llm::model::workload::WORKLOAD_ROUTE_TIERS
        .iter()
        .map(|route| route.strip_prefix("hint:").expect("native hint route"))
        .collect::<Vec<_>>();
    assert_eq!(native, WORKLOAD_ROLES);
}
