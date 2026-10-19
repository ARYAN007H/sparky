use super::*;
use crate::core::all::{
    assert_schema_controller_parity, expect_schema_fields, expect_schema_identity,
};

#[test]
fn encryption_schema_contracts_and_registration_are_preserved() {
    assert_eq!(all_controller_schemas().len(), 2);
    assert_eq!(all_registered_controllers().len(), 2);
    assert_schema_controller_parity(
        &all_controller_schemas(),
        &all_registered_controllers(),
        "secret",
    );
    for (key, namespace, function, input) in [
        ("encrypt_secret", "encrypt", "secret", "plaintext"),
        ("decrypt_secret", "decrypt", "secret", "ciphertext"),
    ] {
        let schema = schemas(key);
        expect_schema_identity(&schema, namespace, function);
        assert_eq!(schema.inputs.len(), 1);
        expect_schema_fields(&schema, &[(input, Some(true))]);
    }
}

#[test]
fn read_required_parses_values_and_preserves_error_categories() {
    let mut m = Map::new();
    m.insert("key".into(), Value::String("value".into()));
    let result: String = read_required(&m, "key").unwrap();
    assert_eq!(result, "value");
    assert!(read_required::<String>(&Map::new(), "key")
        .unwrap_err()
        .contains("missing required param"));
    let wrong = Map::from_iter([("key".into(), Value::Bool(true))]);
    assert!(read_required::<String>(&wrong, "key")
        .unwrap_err()
        .contains("invalid"));
}
