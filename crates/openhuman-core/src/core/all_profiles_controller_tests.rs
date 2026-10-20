use super::*;

#[test]
fn raw_profiles_operator_registry_matches_its_independent_contract() {
    let entries: Vec<_> = registry()
        .iter()
        .filter(|row| row.controller.schema.namespace == "profiles")
        .collect();
    let contracts = crate::profiles::schemas::profile_parameter_contracts();
    assert_eq!(entries.len(), 7, "complete raw Profiles inventory");
    assert_eq!(
        entries
            .iter()
            .map(|row| row.controller.schema.method_name())
            .collect::<Vec<_>>(),
        contracts
            .iter()
            .map(|contract| contract.method.to_owned())
            .collect::<Vec<_>>(),
        "Profiles registration order is a wire contract"
    );
    assert!(entries.iter().all(|row| row.group == DomainGroup::Operator));
    assert!(internal_registry()
        .iter()
        .all(|row| row.controller.schema.namespace != "profiles"));
    assert_eq!(
        entries
            .iter()
            .map(|row| row.controller.rpc_method_name())
            .collect::<Vec<_>>(),
        [
            "openhuman.profiles_provision",
            "openhuman.profiles_deprovision",
            "openhuman.profiles_list",
            "openhuman.profiles_status",
            "openhuman.profiles_set_credential",
            "openhuman.profiles_clear_credential",
            "openhuman.profiles_release",
        ]
    );
    let entries = entries
        .into_iter()
        .map(|row| row.controller.clone())
        .collect::<Vec<_>>();
    verify_parameter_contracts(&entries, &contracts)
        .expect("raw Profiles entries use their declared schemas and runtime decoders");
}
