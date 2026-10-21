use super::*;
use crate::core::all::assert_schema_controller_parity;

#[test]
fn legacy_handlers_keep_their_identity_and_order() {
    use crate::core::all::ControllerHandler;

    let handlers: [(&str, ControllerHandler); 2] =
        [("report", handle_report), ("models", handle_models)];
    let controllers = all_registered_controllers();
    assert_eq!(controllers.len(), handlers.len());
    assert_eq!(all_controller_schemas().len(), handlers.len());
    assert_schema_controller_parity(&all_controller_schemas(), &controllers, "report");

    for ((function, expected_handler), controller) in handlers.iter().zip(&controllers) {
        assert_eq!(controller.schema.namespace, "doctor");
        assert_eq!(controller.schema.function, *function);
        assert!(std::ptr::fn_addr_eq(controller.handler, *expected_handler));
    }
}

#[test]
fn report_schema() {
    let s = schemas("report");
    assert_eq!(s.namespace, "doctor");
    assert_eq!(s.function, "report");
    assert!(s.inputs.is_empty());
}

#[test]
fn models_schema_has_optional_use_cache() {
    let s = schemas("models");
    assert_eq!(s.function, "models");
    let use_cache = s.inputs.iter().find(|f| f.name == "use_cache");
    assert!(use_cache.is_some_and(|f| !f.required));
}

#[test]
fn optional_cache_flag_preserves_missing_null_and_boolean_values() {
    for (case, input, expected) in [
        ("missing", None, None),
        ("null", Some(Value::Null), None),
        ("true", Some(Value::Bool(true)), Some(true)),
        ("false", Some(Value::Bool(false)), Some(false)),
    ] {
        let mut params = Map::new();
        if let Some(input) = input {
            params.insert("use_cache".into(), input);
        }
        assert_eq!(
            read_optional::<bool>(&params, "use_cache").unwrap(),
            expected,
            "{case}"
        );
    }
}

#[test]
fn read_optional_errors_on_wrong_type() {
    for value in [
        Value::String("true".into()),
        Value::Number(1.into()),
        Value::Array(vec![]),
        Value::Object(Map::new()),
    ] {
        let params = Map::from_iter([("use_cache".into(), value)]);
        let error = read_optional::<bool>(&params, "use_cache").unwrap_err();
        assert!(
            error.starts_with("invalid 'use_cache':"),
            "wrong type error keeps its field prefix: {error}"
        );
    }
}

pub(crate) fn doctor_parameter_contracts() -> Vec<crate::core::all::ControllerParamContract> {
    use serde_json::json;

    vec![
        crate::core::all::ControllerParamContract {
            method: "doctor.report",
            required: &[],
            optional: &[],
            examples: vec![json!({})],
        },
        crate::core::all::ControllerParamContract {
            method: "doctor.models",
            required: &[],
            optional: &["use_cache"],
            examples: vec![
                json!({}),
                json!({"use_cache":null}),
                json!({"use_cache":true}),
                json!({"use_cache":false}),
            ],
        },
    ]
}
