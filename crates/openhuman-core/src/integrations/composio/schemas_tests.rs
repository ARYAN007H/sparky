use super::*;
use serde_json::json;

#[test]
fn all_schemas_use_composio_namespace_and_have_descriptions() {
    for s in all_controller_schemas() {
        assert_eq!(s.namespace, "composio", "function {}", s.function);
        assert!(!s.description.is_empty());
        assert!(
            !s.outputs.is_empty(),
            "function {} has no outputs",
            s.function
        );
    }
}

#[test]
fn every_known_schema_key_resolves() {
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
    ];
    for k in keys {
        let s = schemas(k);
        assert_eq!(s.namespace, "composio");
        assert_ne!(s.function, "unknown", "key `{k}` fell through");
    }
}

#[test]
fn list_capabilities_schema_has_matrix_output() {
    let s = schemas("list_capabilities");
    assert_eq!(s.namespace, "composio");
    assert_eq!(s.function, "list_capabilities");
    assert!(s.inputs.is_empty());
    assert!(s
        .outputs
        .iter()
        .any(|f| f.name == "capabilities" && f.required));
}

#[test]
fn authorize_schema_requires_toolkit() {
    let s = schemas("authorize");
    let tk = s.inputs.iter().find(|f| f.name == "toolkit").unwrap();
    assert!(tk.required);
}

#[test]
fn execute_schema_requires_tool_and_accepts_optional_arguments() {
    let s = schemas("execute");
    assert!(s.inputs.iter().any(|f| f.name == "tool" && f.required));
    let args = s.inputs.iter().find(|f| f.name == "arguments");
    assert!(args.is_some());
    assert!(!args.unwrap().required);
}

// ── read_required / read_required_non_empty / read_optional ────

#[test]
fn read_required_parses_string_value() {
    let mut m = Map::new();
    m.insert("toolkit".into(), Value::String("gmail".into()));
    let v: String = read_required(&m, "toolkit").unwrap();
    assert_eq!(v, "gmail");
}

#[test]
fn read_required_errors_when_missing() {
    let m = Map::new();
    let err = read_required::<String>(&m, "toolkit").unwrap_err();
    assert!(err.contains("missing required param"));
}

#[test]
fn read_required_errors_when_wrong_type() {
    let mut m = Map::new();
    m.insert("toolkit".into(), json!(42));
    let err = read_required::<String>(&m, "toolkit").unwrap_err();
    assert!(err.contains("invalid 'toolkit'"));
}

#[test]
fn read_required_non_empty_rejects_blank_and_whitespace() {
    let mut m = Map::new();
    m.insert("toolkit".into(), Value::String("".into()));
    assert!(read_required_non_empty(&m, "toolkit")
        .unwrap_err()
        .contains("must not be empty"));
    m.insert("toolkit".into(), Value::String("   ".into()));
    assert!(read_required_non_empty(&m, "toolkit")
        .unwrap_err()
        .contains("must not be empty"));
}

#[test]
fn read_required_non_empty_trims_value() {
    let mut m = Map::new();
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
fn read_optional_parses_typed_value() {
    let mut m = Map::new();
    m.insert("toolkits".into(), json!(["gmail", "notion"]));
    let v: Vec<String> = read_optional(&m, "toolkits").unwrap().unwrap();
    assert_eq!(v, vec!["gmail".to_string(), "notion".to_string()]);
}

#[test]
fn read_optional_errors_on_type_mismatch() {
    let mut m = Map::new();
    m.insert("toolkits".into(), Value::String("not-an-array".into()));
    let err = read_optional::<Vec<String>>(&m, "toolkits").unwrap_err();
    assert!(err.contains("invalid 'toolkits'"));
}

#[test]
fn to_json_wraps_outcome() {
    let v = to_json(Outcome::single_log(json!({"x": 1}), "note")).unwrap();
    assert!(v.get("logs").is_some() || v.get("result").is_some() || v.get("x").is_some());
}

// ── Trigger management schema coverage ──────────────────────────────

#[test]
fn trigger_management_schemas_resolve() {
    for k in [
        "list_available_triggers",
        "list_triggers",
        "enable_trigger",
        "disable_trigger",
    ] {
        let s = schemas(k);
        assert_eq!(s.namespace, "composio");
        assert_ne!(s.function, "unknown", "key `{k}` fell through");
        assert!(!s.description.is_empty());
        assert!(!s.outputs.is_empty());
    }
}

#[test]
fn list_available_triggers_schema_input_shape() {
    let s = schemas("list_available_triggers");
    assert!(s.inputs.iter().any(|f| f.name == "toolkit" && f.required));
    let conn = s.inputs.iter().find(|f| f.name == "connection_id").unwrap();
    assert!(!conn.required);
}

#[test]
fn list_triggers_schema_input_shape() {
    let s = schemas("list_triggers");
    let tk = s.inputs.iter().find(|f| f.name == "toolkit").unwrap();
    assert!(!tk.required);
}

#[test]
fn enable_trigger_schema_input_shape() {
    let s = schemas("enable_trigger");
    assert!(s
        .inputs
        .iter()
        .any(|f| f.name == "connection_id" && f.required));
    assert!(s.inputs.iter().any(|f| f.name == "slug" && f.required));
    let cfg = s
        .inputs
        .iter()
        .find(|f| f.name == "trigger_config")
        .unwrap();
    assert!(!cfg.required);
}

#[test]
fn disable_trigger_schema_input_shape() {
    let s = schemas("disable_trigger");
    assert!(s
        .inputs
        .iter()
        .any(|f| f.name == "trigger_id" && f.required));
}

#[test]
fn trigger_management_controllers_are_all_registered() {
    let registered = all_registered_controllers();
    for k in [
        "list_available_triggers",
        "list_triggers",
        "enable_trigger",
        "disable_trigger",
    ] {
        assert!(
            registered.iter().any(|c| c.schema.function == k),
            "controller {k} not registered"
        );
    }
}

#[test]
fn controller_schema_catalog_covers_each_declared_function_shape() {
    // Pin required inputs rather than exact counts so additive optional params
    // remain backwards-compatible. This was previously checked by a host
    // reachability suite despite being a schema contract owned by this domain.
    let expected: [(&str, &[&str], &str); 22] = [
        ("list_toolkits", &[], "toolkits"),
        ("list_capabilities", &[], "capabilities"),
        ("list_agent_ready_toolkits", &[], "toolkits"),
        ("list_connections", &[], "connections"),
        ("authorize", &["toolkit"], "connectUrl"),
        ("delete_connection", &["connection_id"], "deleted"),
        ("list_tools", &["toolkits"], "tools"),
        ("execute", &["tool", "connection_id"], "result"),
        ("list_github_repos", &["connection_id"], "result"),
        ("create_trigger", &["slug", "connection_id"], "result"),
        ("get_user_profile", &["connection_id"], "profile"),
        ("refresh_all_identities", &[], "report"),
        ("list_trigger_history", &["limit"], "result"),
        ("get_user_scopes", &["toolkit"], "pref"),
        ("set_user_scopes", &["toolkit", "read", "write"], "pref"),
        ("list_available_triggers", &["toolkit"], "triggers"),
        ("list_triggers", &["toolkit"], "triggers"),
        ("enable_trigger", &["connection_id", "slug"], "result"),
        ("disable_trigger", &["trigger_id"], "deleted"),
        ("get_mode", &[], "mode"),
        ("set_api_key", &["api_key"], "result"),
        ("clear_api_key", &[], "result"),
    ];

    for (function, required_inputs, first_output) in expected {
        let schema = schemas(function);
        assert_eq!(schema.namespace, "composio");
        assert_eq!(schema.function, function);
        let input_names: Vec<&str> = schema.inputs.iter().map(|field| field.name).collect();
        for required in required_inputs {
            assert!(
                input_names.contains(required),
                "{function} must declare input `{required}` (got {input_names:?})"
            );
        }
        assert_eq!(schema.outputs[0].name, first_output, "{function}");
        assert!(!schema.description.is_empty());
    }
}
