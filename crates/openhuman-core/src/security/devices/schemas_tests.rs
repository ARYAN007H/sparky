use super::*;
use crate::core::all::{
    assert_schema_controller_parity, expect_schema_fields, expect_schema_identity,
    expect_schema_input_order, expect_schema_output_fields, expect_schema_output_names,
    ControllerHandler, ControllerParamContract,
};
use serde_json::json;

#[test]
fn legacy_handlers_keep_their_identity_and_order() {
    let expected: [(&str, ControllerHandler); 3] = [
        ("create_pairing", handle_create_pairing),
        ("list", handle_list),
        ("revoke", handle_revoke),
    ];
    let controllers = all_registered_controllers();
    assert_eq!(controllers.len(), expected.len());
    for ((function, handler), controller) in expected.iter().zip(&controllers) {
        assert_eq!(controller.schema.namespace, "devices");
        assert_eq!(controller.schema.function, *function);
        assert!(std::ptr::fn_addr_eq(controller.handler, *handler));
    }
}

pub(super) fn parameter_contracts() -> Vec<ControllerParamContract> {
    vec![
        ControllerParamContract {
            method: "devices.create_pairing",
            required: &[],
            optional: &["label"],
            examples: vec![
                json!({}),
                json!({"label": null}),
                json!({"label": ""}),
                json!({"label": "  preserve label  "}),
            ],
        },
        ControllerParamContract {
            method: "devices.list",
            required: &[],
            optional: &[],
            examples: vec![json!({})],
        },
        ControllerParamContract {
            method: "devices.revoke",
            required: &["channel_id"],
            optional: &[],
            examples: vec![
                json!({"channel_id": ""}),
                json!({"channel_id": "  raw channel id  "}),
            ],
        },
    ]
}

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
fn create_pairing_parser_preserves_optional_labels_and_strict_type_errors() {
    for (params, expected) in [
        (Map::new(), None),
        (Map::from_iter([("label".into(), json!(null))]), None),
        (
            Map::from_iter([("label".into(), json!(""))]),
            Some(String::new()),
        ),
        (
            Map::from_iter([("label".into(), json!("  preserve label  "))]),
            Some("  preserve label  ".to_owned()),
        ),
    ] {
        assert_eq!(parse_create_pairing(params).unwrap(), expected);
    }

    for (value, kind) in [
        (json!(true), "bool"),
        (json!(7), "number"),
        (json!([]), "array"),
        (json!({}), "object"),
    ] {
        assert_eq!(
            parse_create_pairing(Map::from_iter([("label".into(), value)])),
            Err(format!("invalid 'label': expected string, got {kind}"))
        );
    }
}

#[test]
fn list_parser_ignores_the_entire_parameter_map() {
    parse_list(Map::from_iter([
        ("unexpected".into(), json!({"nested": [true, null]})),
        ("channel_id".into(), json!(false)),
    ]))
    .expect("devices.list ignores all parameters");
}

#[test]
fn revoke_parser_requires_and_preserves_a_string_channel_id() {
    assert_eq!(
        parse_revoke(Map::new()).unwrap_err(),
        "missing required param 'channel_id'"
    );
    for (value, serde_error) in [
        (json!(null), "invalid type: null, expected a string"),
        (json!(true), "invalid type: boolean `true`, expected a string"),
        (json!(7), "invalid type: integer `7`, expected a string"),
        (json!([]), "invalid type: sequence, expected a string"),
        (json!({}), "invalid type: map, expected a string"),
    ] {
        assert_eq!(
            parse_revoke(Map::from_iter([("channel_id".into(), value)])),
            Err(format!("invalid 'channel_id': {serde_error}"))
        );
    }
    for value in ["", "ordinary-channel", "  raw channel id  "] {
        assert_eq!(
            parse_revoke(Map::from_iter([("channel_id".into(), json!(value))])).unwrap(),
            value
        );
    }
}
