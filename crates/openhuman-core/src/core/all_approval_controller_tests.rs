use super::*;
use crate::core::TypeSchema;
use serde_json::json;

#[test]
fn raw_approval_security_registry_matches_its_independent_contract() {
    let grouped: Vec<_> = registry()
        .iter()
        .filter(|row| row.controller.schema.namespace == "approval")
        .collect();
    let methods = [
        "approval.list_pending",
        "approval.list_recent_decisions",
        "approval.decide",
        "approval.get_gate_state",
        "approval.preauthorize_flow",
    ];
    let rpc_methods = [
        "openhuman.approval_list_pending",
        "openhuman.approval_list_recent_decisions",
        "openhuman.approval_decide",
        "openhuman.approval_get_gate_state",
        "openhuman.approval_preauthorize_flow",
    ];
    let contracts = crate::security::approval::approval_parameter_contracts();

    assert_eq!(grouped.len(), methods.len(), "complete raw approval inventory");
    assert_eq!(
        grouped
            .iter()
            .map(|row| row.controller.schema.method_name())
            .collect::<Vec<_>>(),
        methods,
        "independent approval method inventory and order"
    );
    assert!(grouped.iter().all(|row| row.group == DomainGroup::Security));
    assert!(internal_registry()
        .iter()
        .all(|row| row.controller.schema.namespace != "approval"));
    assert_eq!(
        grouped
            .iter()
            .map(|row| row.controller.rpc_method_name())
            .collect::<Vec<_>>(),
        rpc_methods
    );

    let entries: Vec<_> = grouped
        .iter()
        .map(|row| row.controller.clone())
        .collect();
    assert_eq!(
        contracts.iter().map(|row| row.method).collect::<Vec<_>>(),
        methods,
        "independent fixtures enumerate every method in registration order"
    );
    crate::core::all::verify_parameter_contracts(&entries, &contracts)
        .expect("raw approval entries use their declared schemas and runtime decoders");

    let schemas: Vec<_> = entries.iter().map(|entry| &entry.schema).collect();
    for (schema, expected) in schemas.iter().zip([
        ["agent_id"].as_slice(),
        ["limit"].as_slice(),
        ["request_id", "decision", "agent_id"].as_slice(),
        [].as_slice(),
        ["flow_id", "tool_names"].as_slice(),
    ]) {
        assert_eq!(
            schema.inputs.iter().map(|field| field.name).collect::<Vec<_>>(),
            expected,
            "{}.{} input order",
            schema.namespace,
            schema.function
        );
    }
    assert_input(
        schemas[0],
        "agent_id",
        false,
        |ty| matches!(ty, TypeSchema::Option(inner) if matches!(inner.as_ref(), TypeSchema::String)),
    );
    assert_eq!(schemas[1].inputs.len(), 1);
    assert_input(schemas[1], "limit", false, |ty| {
        matches!(ty, TypeSchema::Option(inner) if matches!(inner.as_ref(), TypeSchema::U64))
    });
    assert_eq!(schemas[2].inputs.len(), 3);
    assert_input(schemas[2], "request_id", true, |ty| matches!(ty, TypeSchema::String));
    assert_input(schemas[2], "decision", true, |ty| matches!(ty, TypeSchema::String));
    assert_input(schemas[2], "agent_id", false, |ty| {
        matches!(ty, TypeSchema::Option(inner) if matches!(inner.as_ref(), TypeSchema::String))
    });
    assert!(schemas[3].inputs.is_empty());
    assert_eq!(schemas[4].inputs.len(), 2);
    assert_input(schemas[4], "flow_id", true, |ty| matches!(ty, TypeSchema::String));
    assert_input(schemas[4], "tool_names", true, |ty| {
        matches!(ty, TypeSchema::Array(inner) if matches!(inner.as_ref(), TypeSchema::String))
    });

    let outputs = [
        ("pending", "PendingApproval", true),
        ("decisions", "ApprovalAuditEntry", true),
        ("decided", "PendingApproval", true),
        ("state", "ApprovalGateBootState", true),
        ("result", "FlowPreauthorizationResult", true),
    ];
    for (schema, (name, reference, required)) in schemas.iter().zip(outputs) {
        assert_eq!(schema.outputs.len(), 1, "{}.{} output count", schema.namespace, schema.function);
        let output = &schema.outputs[0];
        assert_eq!(output.name, name);
        assert_eq!(output.required, required);
        assert!(matches!(&output.ty, TypeSchema::Ref(actual) if *actual == reference));
    }

    for (entry, extra) in entries.iter().zip([
        json!({"agent_id": "  raw agent  ", "unknown": {"nested": [1, null]}}),
        json!({"limit": 0, "unknown": true}),
        json!({"request_id": " raw id ", "decision": "deny", "extra": [false]}),
        json!({"anything": {"ignored": true}}),
        json!({"flow_id": " raw flow ", "tool_names": [" ", "tool"], "extra": null}),
    ]) {
        entry
            .decode_params(extra.as_object().unwrap().clone())
            .expect("actual bound decoder ignores undeclared parameter keys");
    }
    entries[0]
        .decode_params(Map::from_iter([("agent_id".into(), json!(false))]))
        .expect_err("bound schema decoder rejects wrong agent_id type");
    entries[1]
        .decode_params(Map::from_iter([("limit".into(), json!(-1))]))
        .expect_err("bound schema decoder rejects negative limit");
    entries[2]
        .decode_params(Map::from_iter([
            ("request_id".into(), json!("id")),
            ("decision".into(), json!("unsupported")),
        ]))
        .expect_err("bound schema decoder enforces string wire types");
    entries[4]
        .decode_params(Map::from_iter([
            ("flow_id".into(), json!("flow")),
            ("tool_names".into(), json!([1])),
        ]))
        .expect_err("bound schema decoder enforces array item wire types");
}

fn assert_input(
    schema: &crate::core::ControllerSchema,
    name: &str,
    required: bool,
    type_matches: impl FnOnce(&TypeSchema) -> bool,
) {
    let field = schema
        .inputs
        .iter()
        .find(|field| field.name == name)
        .unwrap_or_else(|| panic!("{}.{} missing input {name}", schema.namespace, schema.function));
    assert_eq!(field.required, required, "{}.{} {name}", schema.namespace, schema.function);
    assert!(
        type_matches(&field.ty),
        "{}.{} input {name} has wrong type",
        schema.namespace,
        schema.function
    );
}
