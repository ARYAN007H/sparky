use super::*;
use crate::core::all::{
    assert_schema_controller_parity, expect_schema_fields, expect_schema_identity,
    expect_schema_input_order, expect_schema_output_fields, expect_schema_output_names,
};
use serde_json::json;

#[test]
fn device_schema_wire_contracts_are_preserved() {
    assert_schema_controller_parity(
        &all_controller_schemas(),
        &all_registered_controllers(),
        "create_pairing",
    );
    let contracts = ["create_pairing", "list", "revoke"];
    let names: Vec<_> = all_controller_schemas()
        .into_iter()
        .map(|schema| schema.function)
        .collect();
    assert_eq!(names, contracts);

    let create = schemas("create_pairing");
    expect_schema_identity(&create, "devices", "create_pairing");
    expect_schema_input_order(&create, &["label"]);
    expect_schema_fields(&create, &[("label", Some(false))]);
    expect_schema_output_fields(&create, &["channel_id", "pairing_token", "core_pubkey"]);

    let list = schemas("list");
    expect_schema_identity(&list, "devices", "list");
    assert!(list.inputs.is_empty());
    expect_schema_output_names(&list, &["devices"]);

    let revoke = schemas("revoke");
    expect_schema_identity(&revoke, "devices", "revoke");
    expect_schema_input_order(&revoke, &["channel_id"]);
    expect_schema_fields(&revoke, &[("channel_id", Some(true))]);
    expect_schema_output_names(&revoke, &["success"]);
}

#[test]
fn read_required_errors_when_key_missing() {
    let params = Map::new();
    let err = read_required::<String>(&params, "channel_id").unwrap_err();
    assert!(err.contains("missing required param 'channel_id'"));
}

#[test]
fn read_optional_string_absent_key_is_none() {
    let result = read_optional_string(&Map::new(), "label").unwrap();
    assert!(result.is_none());
}

#[test]
fn read_optional_string_present_value_returned() {
    let mut params = Map::new();
    params.insert("label".into(), json!("iPhone 15"));
    let result = read_optional_string(&params, "label").unwrap();
    assert_eq!(result, Some("iPhone 15".to_string()));
}

#[test]
fn type_name_covers_all_variants() {
    assert_eq!(type_name(&Value::Null), "null");
    assert_eq!(type_name(&json!(true)), "bool");
    assert_eq!(type_name(&json!(1)), "number");
    assert_eq!(type_name(&json!("s")), "string");
    assert_eq!(type_name(&json!([])), "array");
    assert_eq!(type_name(&json!({})), "object");
}
