use super::*;
use crate::core::TypeSchema;

#[test]
fn raw_doctor_platform_registry_matches_its_independent_contract() {
    let entries: Vec<_> = registry()
        .iter()
        .filter(|row| row.controller.schema.namespace == "doctor")
        .collect();
    let contracts = crate::platform::doctor::doctor_parameter_contracts();
    let methods = ["doctor.report", "doctor.models"];

    assert_eq!(
        entries.len(),
        methods.len(),
        "complete raw Doctor inventory"
    );
    assert_eq!(
        entries
            .iter()
            .map(|row| row.controller.schema.method_name())
            .collect::<Vec<_>>(),
        methods,
        "Doctor registration order is a wire contract"
    );
    assert!(entries.iter().all(|row| row.group == DomainGroup::Platform));
    assert!(internal_registry()
        .iter()
        .all(|row| row.controller.schema.namespace != "doctor"));
    assert_eq!(
        entries
            .iter()
            .map(|row| row.controller.rpc_method_name())
            .collect::<Vec<_>>(),
        ["openhuman.doctor_report", "openhuman.doctor_models"]
    );

    for (entry, output_type) in entries.iter().zip(["DoctorReport", "ModelProbeReport"]) {
        let outputs = &entry.controller.schema.outputs;
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].name, "report");
        assert!(outputs[0].required);
        assert!(matches!(
            &outputs[0].ty,
            TypeSchema::Ref(name) if *name == output_type
        ));
    }

    let entries: Vec<_> = entries
        .into_iter()
        .map(|row| row.controller.clone())
        .collect();
    crate::core::all::verify_parameter_contracts(&entries, &contracts)
        .expect("raw Doctor entries use their declared schemas and runtime decoders");
}

#[test]
fn raw_doctor_decoders_ignore_unknown_fields_and_keep_boolean_types_strict() {
    use serde_json::json;

    let entries: Vec<_> = registry()
        .iter()
        .filter(|row| row.controller.schema.namespace == "doctor")
        .map(|row| row.controller.clone())
        .collect();
    let report = entries
        .iter()
        .find(|entry| entry.schema.method_name() == "doctor.report")
        .expect("raw doctor.report entry");
    report
        .decode_params(Map::from_iter([
            ("use_cache".into(), json!("not a boolean")),
            ("nested_extra".into(), json!({"ignored":[1, false]})),
        ]))
        .expect("doctor.report ignores the entire parameter map");

    let models = entries
        .iter()
        .find(|entry| entry.schema.method_name() == "doctor.models")
        .expect("raw doctor.models entry");
    models
        .decode_params(Map::from_iter([
            ("use_cache".into(), json!(false)),
            ("extra_scalar".into(), json!(true)),
            ("nested_extra".into(), json!({"ignored":true})),
        ]))
        .expect("doctor.models ignores undeclared fields");

    for invalid in [
        json!("false"),
        json!(1),
        json!([]),
        json!({"enabled":false}),
    ] {
        let error = models
            .decode_params(Map::from_iter([("use_cache".into(), invalid)]))
            .expect_err("use_cache accepts only JSON booleans or null");
        assert!(
            error.starts_with("invalid 'use_cache':"),
            "invalid model flag keeps its parser prefix: {error}"
        );
    }
}
