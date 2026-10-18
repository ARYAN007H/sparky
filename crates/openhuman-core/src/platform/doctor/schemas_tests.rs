use super::*;

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
fn read_optional_returns_none_for_missing() {
    let m = Map::new();
    let result: Option<bool> = read_optional(&m, "use_cache").unwrap();
    assert!(result.is_none());
}

#[test]
fn read_optional_returns_none_for_null() {
    let mut m = Map::new();
    m.insert("use_cache".into(), Value::Null);
    let result: Option<bool> = read_optional(&m, "use_cache").unwrap();
    assert!(result.is_none());
}

#[test]
fn read_optional_returns_some_for_value() {
    let mut m = Map::new();
    m.insert("use_cache".into(), Value::Bool(true));
    let result: Option<bool> = read_optional(&m, "use_cache").unwrap();
    assert_eq!(result, Some(true));
}

#[test]
fn read_optional_errors_on_wrong_type() {
    let mut m = Map::new();
    m.insert("use_cache".into(), Value::String("yes".into()));
    let err = read_optional::<bool>(&m, "use_cache").unwrap_err();
    assert!(err.contains("invalid"));
}
