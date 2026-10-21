use super::*;
use crate::core::TypeSchema;

#[test]
fn raw_devices_security_registry_matches_its_independent_contract() {
    let entries: Vec<_> = registry()
        .iter()
        .filter(|row| row.controller.schema.namespace == "devices")
        .collect();
    let methods = [
        "devices.create_pairing",
        "devices.list",
        "devices.revoke",
    ];
    let contracts = crate::security::devices::devices_parameter_contracts();

    assert_eq!(entries.len(), methods.len(), "complete raw devices inventory");
    assert_eq!(
        entries
            .iter()
            .map(|row| row.controller.schema.method_name())
            .collect::<Vec<_>>(),
        methods,
        "device registration order is a wire contract"
    );
    assert!(entries
        .iter()
        .all(|row| row.group == DomainGroup::Security));
    assert!(internal_registry()
        .iter()
        .all(|row| row.controller.schema.namespace != "devices"));
    assert_eq!(
        entries
            .iter()
            .map(|row| row.controller.rpc_method_name())
            .collect::<Vec<_>>(),
        [
            "openhuman.devices_create_pairing",
            "openhuman.devices_list",
            "openhuman.devices_revoke"
        ]
    );

    let create = &entries[0].controller.schema;
    assert_eq!(create.inputs.len(), 1);
    assert_eq!(create.inputs[0].name, "label");
    assert!(!create.inputs[0].required);
    assert!(matches!(
        &create.inputs[0].ty,
        TypeSchema::Option(inner) if matches!(inner.as_ref(), TypeSchema::String)
    ));
    assert_eq!(
        create
            .outputs
            .iter()
            .map(|field| field.name)
            .collect::<Vec<_>>(),
        ["channel_id", "pairing_token", "core_pubkey", "rpc_url", "expires_at"]
    );
    assert_eq!(create.outputs.iter().filter(|field| field.required).count(), 4);

    let list = &entries[1].controller.schema;
    assert!(list.inputs.is_empty());
    assert_eq!(list.outputs.len(), 1);
    assert_eq!(list.outputs[0].name, "devices");
    assert!(list.outputs[0].required);
    assert!(matches!(
        &list.outputs[0].ty,
        TypeSchema::Array(inner) if matches!(inner.as_ref(), TypeSchema::Ref("PairedDevice"))
    ));

    let revoke = &entries[2].controller.schema;
    assert_eq!(revoke.inputs.len(), 1);
    assert_eq!(revoke.inputs[0].name, "channel_id");
    assert!(revoke.inputs[0].required);
    assert!(matches!(&revoke.inputs[0].ty, TypeSchema::String));
    assert_eq!(revoke.outputs.len(), 1);
    assert_eq!(revoke.outputs[0].name, "success");
    assert!(revoke.outputs[0].required);
    assert!(matches!(&revoke.outputs[0].ty, TypeSchema::Bool));

    let entries: Vec<_> = entries
        .into_iter()
        .map(|row| row.controller.clone())
        .collect();
    crate::core::all::verify_parameter_contracts(&entries, &contracts)
        .expect("raw devices entries use their declared schemas and runtime decoders");
}

#[test]
fn raw_devices_decoders_ignore_extra_fields_and_list_parameters() {
    use serde_json::json;

    let entries: Vec<_> = registry()
        .iter()
        .filter(|row| row.controller.schema.namespace == "devices")
        .map(|row| row.controller.clone())
        .collect();

    entries[0]
        .decode_params(Map::from_iter([
            ("label".into(), json!("  preserve label  ")),
            ("unknown_scalar".into(), json!(true)),
            ("unknown_nested".into(), json!({"ignored": [1, null]})),
        ]))
        .expect("create_pairing ignores undeclared fields");
    entries[1]
        .decode_params(Map::from_iter([
            ("anything".into(), json!({"ignored": true})),
            ("label".into(), json!(false)),
        ]))
        .expect("list ignores its entire parameter map");
    entries[2]
        .decode_params(Map::from_iter([
            ("channel_id".into(), json!("  raw channel id  ")),
            ("extra".into(), json!(false)),
        ]))
        .expect("revoke ignores undeclared fields");
}
