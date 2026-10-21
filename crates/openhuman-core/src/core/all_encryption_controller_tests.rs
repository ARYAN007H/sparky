use super::*;

#[test]
fn raw_encryption_security_registry_matches_its_independent_contract() {
    let entries: Vec<_> = registry()
        .iter()
        .filter(|row| {
            matches!(
                row.controller.schema.method_name().as_str(),
                "encrypt.secret" | "decrypt.secret"
            )
        })
        .collect();
    let contracts = crate::security::encryption::encryption_parameter_contracts();

    assert_eq!(entries.len(), 2, "complete raw encryption inventory");
    assert_eq!(
        entries
            .iter()
            .map(|row| row.controller.schema.method_name())
            .collect::<Vec<_>>(),
        ["encrypt.secret", "decrypt.secret"],
        "encryption registration order is a wire contract"
    );
    assert!(entries.iter().all(|row| {
        row.group == DomainGroup::Security
            && ["encrypt", "decrypt"].contains(&row.controller.schema.namespace)
    }));
    assert!(internal_registry().iter().all(|row| {
        !matches!(
            row.controller.schema.method_name().as_str(),
            "encrypt.secret" | "decrypt.secret"
        )
    }));
    assert_eq!(
        entries
            .iter()
            .map(|row| row.controller.rpc_method_name())
            .collect::<Vec<_>>(),
        ["openhuman.encrypt_secret", "openhuman.decrypt_secret"]
    );

    let entries = entries
        .into_iter()
        .map(|row| row.controller.clone())
        .collect::<Vec<_>>();
    verify_parameter_contracts(&entries, &contracts)
        .expect("raw encryption entries use their declared schemas and runtime decoders");
}

#[test]
fn bound_encryption_decoders_preserve_required_strings_and_ignore_extra_fields() {
    use serde_json::json;

    let entries = registry()
        .iter()
        .filter(|row| {
            matches!(
                row.controller.schema.method_name().as_str(),
                "encrypt.secret" | "decrypt.secret"
            )
        })
        .map(|row| row.controller.clone())
        .collect::<Vec<_>>();

    for (method, field) in [
        ("encrypt.secret", "plaintext"),
        ("decrypt.secret", "ciphertext"),
    ] {
        let entry = entries
            .iter()
            .find(|entry| entry.schema.method_name() == method)
            .expect("raw encryption entry exists");
        let mut params = Map::from_iter([(field.into(), json!("unchanged"))]);
        params.insert("extra_scalar".into(), json!(true));
        params.insert("extra_nested".into(), json!({"ignored":true}));
        entry
            .decode_params(params)
            .unwrap_or_else(|error| panic!("{method} decoder rejected extra fields: {error}"));

        assert_eq!(
            entry.decode_params(Map::new()).unwrap_err(),
            format!("missing required param '{field}'"),
            "{method} missing-field error"
        );
        for value in [json!(null), json!(true), json!(42)] {
            let params = Map::from_iter([(field.into(), value)]);
            let error = entry
                .decode_params(params)
                .expect_err("non-string secret input is rejected");
            assert!(
                error.starts_with(&format!("invalid '{field}':")),
                "{method} invalid-field prefix: {error}"
            );
        }
    }
}
