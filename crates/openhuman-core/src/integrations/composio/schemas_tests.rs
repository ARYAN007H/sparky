use super::*;
use crate::core::all::{
    assert_schema_controller_parity, expect_schema_fields, expect_schema_identity,
    expect_schema_required_inputs,
};
use serde_json::json;

#[test]
fn composio_schema_contracts_and_registration_are_preserved() {
    assert_schema_controller_parity(
        &all_controller_schemas(),
        &all_registered_controllers(),
        "execute",
    );
    let keys = [
        "list_toolkits",
        "list_capabilities",
        "list_connections",
        "authorize",
        "delete_connection",
        "list_tools",
        "execute",
        "get_user_profile",
        "list_trigger_history",
        "get_user_scopes",
        "set_user_scopes",
        "list_available_triggers",
        "list_triggers",
        "enable_trigger",
        "disable_trigger",
    ];
    for function in keys {
        expect_schema_identity(&schemas(function), "composio", function);
    }
    let registered = all_registered_controllers();
    for function in ["list_triggers", "enable_trigger", "disable_trigger", "list_trigger_history"] {
        assert!(
            registered.iter().any(|controller| {
                controller.schema.namespace == "composio" && controller.schema.function == function
            }),
            "composio.{function} is not registered"
        );
    }
    let capabilities = schemas("list_capabilities");
    assert!(capabilities.inputs.is_empty());
    expect_schema_fields(&capabilities, &[("capabilities", Some(true))]);
    expect_schema_fields(&schemas("authorize"), &[("toolkit", Some(true))]);
    expect_schema_fields(
        &schemas("execute"),
        &[("tool", Some(true)), ("arguments", Some(false))],
    );

    expect_schema_fields(
        &schemas("list_available_triggers"),
        &[("toolkit", Some(true)), ("connection_id", Some(false))],
    );
    expect_schema_fields(&schemas("list_triggers"), &[("toolkit", Some(false))]);
    expect_schema_fields(
        &schemas("enable_trigger"),
        &[
            ("connection_id", Some(true)),
            ("slug", Some(true)),
            ("trigger_config", Some(false)),
        ],
    );
    expect_schema_fields(&schemas("disable_trigger"), &[("trigger_id", Some(true))]);
}

// ── read_required / read_required_non_empty / read_optional ────

#[test]
fn read_required_and_non_empty_helpers_preserve_errors_and_trimming() {
    let mut m = Map::new();
    m.insert("toolkit".into(), Value::String("gmail".into()));
    let v: String = read_required(&m, "toolkit").unwrap();
    assert_eq!(v, "gmail");
    assert!(read_required::<String>(&Map::new(), "toolkit")
        .unwrap_err()
        .contains("missing required param"));
    let wrong = Map::from_iter([("toolkit".into(), json!(42))]);
    assert!(read_required::<String>(&wrong, "toolkit")
        .unwrap_err()
        .contains("invalid 'toolkit'"));
    let mut m = Map::new();
    for blank in ["", "   "] {
        m.insert("toolkit".into(), Value::String(blank.into()));
        assert!(read_required_non_empty(&m, "toolkit")
            .unwrap_err()
            .contains("must not be empty"));
    }
    m.insert("toolkit".into(), Value::String("  gmail ".into()));
    assert_eq!(read_required_non_empty(&m, "toolkit").unwrap(), "gmail");
}

#[test]
fn read_optional_returns_none_on_missing_or_null() {
    let mut m = Map::new();
    assert_eq!(read_optional::<String>(&m, "k").unwrap(), None);
    m.insert("k".into(), Value::Null);
    assert_eq!(read_optional::<String>(&m, "k").unwrap(), None);
}

#[test]
fn read_optional_helpers_preserve_typed_values_and_errors() {
    let mut m = Map::new();
    assert_eq!(read_optional::<String>(&m, "k").unwrap(), None);
    m.insert("k".into(), Value::Null);
    assert_eq!(read_optional::<String>(&m, "k").unwrap(), None);
    m.insert("toolkits".into(), json!(["gmail", "notion"]));
    let v: Vec<String> = read_optional(&m, "toolkits").unwrap().unwrap();
    assert_eq!(v, vec!["gmail".to_string(), "notion".to_string()]);
    m.insert("toolkits".into(), Value::String("not-an-array".into()));
    let err = read_optional::<Vec<String>>(&m, "toolkits").unwrap_err();
    assert!(err.contains("invalid 'toolkits'"));
}
