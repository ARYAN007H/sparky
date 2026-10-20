use super::*;
use crate::core::all::assert_schema_controller_parity;

#[test]
fn doctor_schema_catalog_matches_registered_controllers() {
    assert_schema_controller_parity(
        &all_controller_schemas(),
        &all_registered_controllers(),
        "report",
    );
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
        ("boolean", Some(Value::Bool(true)), Some(true)),
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
    let mut m = Map::new();
    m.insert("use_cache".into(), Value::String("yes".into()));
    let err = read_optional::<bool>(&m, "use_cache").unwrap_err();
    assert!(err.contains("invalid"));
}
