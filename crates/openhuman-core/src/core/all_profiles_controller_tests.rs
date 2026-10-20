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

fn raw_profiles_entries() -> Vec<ControllerEntry> {
    registry()
        .iter()
        .filter(|row| row.controller.schema.namespace == "profiles")
        .map(|row| row.controller.clone())
        .collect()
}

#[test]
fn bound_profile_decoders_keep_permissive_unknown_field_behavior() {
    use serde_json::json;

    let entries = raw_profiles_entries();
    for (method, params) in [
        ("profiles.provision", json!({"user_id":"u1", "extra":true})),
        (
            "profiles.deprovision",
            json!({"profile_id":"p1", "extra":true}),
        ),
        ("profiles.list", json!({"entirely_ignored":{"extra":true}})),
        ("profiles.status", json!({"profile_id":"p1", "extra":true})),
        (
            "profiles.set_credential",
            json!({
                "profile_id":"p1",
                "kind":"api_key",
                "token":"fixture-only-token",
                "extra":true
            }),
        ),
        (
            "profiles.clear_credential",
            json!({"profile_id":"p1", "extra":true}),
        ),
        ("profiles.release", json!({"profile_id":"p1", "extra":true})),
    ] {
        let params = params.as_object().expect("fixture object").clone();
        entries
            .iter()
            .find(|entry| entry.schema.method_name() == method)
            .unwrap_or_else(|| panic!("missing raw {method} entry"))
            .decode_params(params)
            .unwrap_or_else(|error| panic!("{method} decoder rejected permissive map: {error}"));
    }
}

#[tokio::test]
async fn bound_profile_controller_parses_before_the_saas_guard() {
    let provision = raw_profiles_entries()
        .into_iter()
        .find(|entry| entry.schema.method_name() == "profiles.provision")
        .expect("raw provision entry");
    let parse_error = provision
        .invoke(DispatchCtx::process_default(), Map::new())
        .await
        .expect_err("missing user id fails parsing");
    assert!(parse_error.starts_with("invalid params:"), "{parse_error}");

    let params = serde_json::from_value(serde_json::json!({"user_id":"user-1"}))
        .expect("valid provision params");
    let guard_error = provision
        .invoke(DispatchCtx::process_default(), params)
        .await
        .expect_err("valid params reach the existing SaaS guard");
    assert!(guard_error.contains("SaaS"), "{guard_error}");
}
