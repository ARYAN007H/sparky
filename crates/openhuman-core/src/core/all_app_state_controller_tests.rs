use super::*;
use crate::core::TypeSchema;

#[test]
fn raw_app_state_desktop_registry_matches_its_independent_contract() {
    let entries: Vec<_> = registry()
        .iter()
        .filter(|row| row.controller.schema.namespace == "app_state")
        .collect();
    let methods = ["app_state.snapshot", "app_state.update_local_state"];
    let contracts = crate::desktop::app_state::app_state_parameter_contracts();

    assert_eq!(
        entries.len(),
        methods.len(),
        "complete raw app_state inventory"
    );
    assert_eq!(
        entries
            .iter()
            .map(|row| row.controller.schema.method_name())
            .collect::<Vec<_>>(),
        methods,
        "app_state registration order is a wire contract"
    );
    assert!(entries.iter().all(|row| row.group == DomainGroup::Desktop));
    assert!(internal_registry()
        .iter()
        .all(|row| row.controller.schema.namespace != "app_state"));
    assert_eq!(
        entries
            .iter()
            .map(|row| row.controller.rpc_method_name())
            .collect::<Vec<_>>(),
        [
            "openhuman.app_state_snapshot",
            "openhuman.app_state_update_local_state"
        ]
    );
    for entry in &entries {
        let outputs = &entry.controller.schema.outputs;
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].name, "result");
        assert!(outputs[0].required);
        assert!(matches!(&outputs[0].ty, TypeSchema::Json));
    }

    let entries: Vec<_> = entries
        .into_iter()
        .map(|row| row.controller.clone())
        .collect();
    crate::core::all::verify_parameter_contracts(&entries, &contracts)
        .expect("raw app_state entries use their declared schemas and runtime decoders");
}

#[test]
fn raw_app_state_decoders_ignore_unknown_outer_fields() {
    use serde_json::json;

    let entries: Vec<_> = registry()
        .iter()
        .filter(|row| row.controller.schema.namespace == "app_state")
        .map(|row| row.controller.clone())
        .collect();
    let snapshot = entries
        .iter()
        .find(|entry| entry.schema.method_name() == "app_state.snapshot")
        .expect("raw snapshot entry");
    snapshot
        .decode_params(Map::from_iter([
            ("unknown_scalar".into(), json!(17)),
            ("unknown_nested".into(), json!({"ignored": [true, null]})),
        ]))
        .expect("snapshot ignores its entire parameter map");

    let update = entries
        .iter()
        .find(|entry| entry.schema.method_name() == "app_state.update_local_state")
        .expect("raw update entry");
    update
        .decode_params(Map::from_iter([
            ("encryptionKey".into(), json!("unchanged")),
            ("ignored_scalar".into(), json!(false)),
            ("ignored_nested".into(), json!({"extra": 1})),
        ]))
        .expect("update ignores undeclared outer keys");
}
