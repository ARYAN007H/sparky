use super::*;
use crate::core::all::{assert_schema_contract, ControllerSchemaContract as Contract, SchemaFieldContract as Field};
use serde_json::json;

#[test]
fn device_schema_wire_contracts_are_preserved() {
    let contracts = [
        Contract {
            namespace: "devices", function: "create_pairing", input_count: Some(1),
            input_order: Some(&["label"]), required_input_order: None,
            inputs: &[Field { name: "label", required: Some(false), type_debug: None }],
            output_fields: &["channel_id", "pairing_token", "core_pubkey"], outputs: None,
        },
        Contract {
            namespace: "devices", function: "list", input_count: Some(0),
            input_order: None, required_input_order: None, inputs: &[],
            output_fields: &[], outputs: Some(&["devices"]),
        },
        Contract {
            namespace: "devices", function: "revoke", input_count: Some(1),
            input_order: Some(&["channel_id"]), required_input_order: None,
            inputs: &[Field { name: "channel_id", required: Some(true), type_debug: None }],
            output_fields: &[], outputs: Some(&["success"]),
        },
    ];
    let names: Vec<_> = all_controller_schemas().into_iter().map(|schema| schema.function).collect();
    assert_eq!(names, contracts.iter().map(|contract| contract.function).collect::<Vec<_>>());
    for contract in &contracts {
        assert_schema_contract(&schemas(contract.function), contract);
    }
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
