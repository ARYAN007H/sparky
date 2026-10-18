use super::*;
use crate::core::all::{assert_schema_contract, ControllerSchemaContract, SchemaFieldContract};

#[test]
fn credential_schema_wire_contracts_are_preserved() {
    // These fixed rows replace independent schema assertions that were
    // previously scattered across one test per method. The key/function pairs
    // and field expectations are copied from those prior assertions.
    let contracts: &[(&str, ControllerSchemaContract)] = &[
        (
            "auth_set_credential",
            ControllerSchemaContract {
                namespace: "auth",
                function: "set_credential",
                input_count: None,
                input_order: None,
                required_input_order: Some(&["token"]),
                inputs: &["kind", "userId", "user"].map(|name| SchemaFieldContract {
                    name,
                    required: None,
                    type_debug: None,
                }),
                output_fields: &[],
                outputs: None,
            },
        ),
        (
            "auth_clear_credential",
            ControllerSchemaContract {
                namespace: "auth",
                function: "clear_credential",
                input_count: None,
                input_order: None,
                required_input_order: Some(&[]),
                inputs: &[SchemaFieldContract {
                    name: "kind",
                    required: Some(false),
                    type_debug: None,
                }],
                output_fields: &[],
                outputs: None,
            },
        ),
        (
            "auth_list_provider_credentials",
            ControllerSchemaContract {
                namespace: "auth",
                function: "list_provider_credentials",
                input_count: None,
                input_order: None,
                required_input_order: None,
                inputs: &[SchemaFieldContract {
                    name: "provider",
                    required: Some(false),
                    type_debug: None,
                }],
                output_fields: &[],
                outputs: None,
            },
        ),
    ];
    let key_contracts = [
        ("auth_get_state", "get_state"),
        ("auth_get_session_token", "get_session_token"),
        (
            "auth_store_provider_credentials",
            "store_provider_credentials",
        ),
        (
            "auth_remove_provider_credentials",
            "remove_provider_credentials",
        ),
    ];
    for (key, expected) in contracts {
        assert_schema_contract(&schemas(key), expected);
    }
    for (key, expected_function) in key_contracts {
        let schema = schemas(key);
        assert_eq!(schema.namespace, "auth", "key `{key}` namespace");
        assert_eq!(schema.function, expected_function, "key `{key}` function");
    }
}

// ── Param-deserialization helper ───────────────────────────────

#[test]
fn deserialize_params_parses_set_credential_request() {
    let mut m = Map::new();
    m.insert("token".into(), Value::String("abc".into()));
    let parsed: crate::security::credentials::SetCredentialRequest = deserialize_params(m).unwrap();
    assert_eq!(parsed.token, "abc");
    assert!(parsed.kind.is_none());
    assert!(parsed.user_id.is_none());
    assert!(parsed.user.is_none());
}

#[test]
fn deserialize_params_honours_userid_camel_and_snake_case() {
    for key in ["userId", "user_id"] {
        let mut m = Map::new();
        m.insert("token".into(), Value::String("abc".into()));
        m.insert(key.into(), Value::String("u1".into()));
        m.insert("kind".into(), Value::String("session".into()));
        let parsed: crate::security::credentials::SetCredentialRequest =
            deserialize_params(m).unwrap();
        assert_eq!(parsed.user_id.as_deref(), Some("u1"), "{key}");
        assert_eq!(parsed.kind.as_deref(), Some("session"));
    }
}

#[test]
fn deserialize_params_reports_missing_required_fields() {
    // `token` is required — an empty object must fail.
    let err = deserialize_params::<crate::security::credentials::SetCredentialRequest>(Map::new())
        .unwrap_err();
    assert!(err.contains("invalid params"));
}

#[test]
fn deserialize_params_parses_clear_credential_kind() {
    let parsed: AuthClearCredentialParams = deserialize_params(Map::new()).unwrap();
    assert!(parsed.kind.is_none());
    let mut m = Map::new();
    m.insert("kind".into(), Value::String("api-key".into()));
    let parsed: AuthClearCredentialParams = deserialize_params(m).unwrap();
    assert_eq!(parsed.kind.as_deref(), Some("api-key"));
}

#[test]
fn deserialize_params_parses_optional_provider_filter() {
    // Empty object is legal (provider is optional).
    let parsed: AuthListProviderCredentialsParams = deserialize_params(Map::new()).unwrap();
    assert!(parsed.provider.is_none());

    let mut m = Map::new();
    m.insert("provider".into(), Value::String("openai".into()));
    let parsed: AuthListProviderCredentialsParams = deserialize_params(m).unwrap();
    assert_eq!(parsed.provider.as_deref(), Some("openai"));
}

// ── RPC-outcome serializer ─────────────────────────────────────

#[test]
fn to_json_emits_logs_and_result_envelope() {
    let outcome = Outcome::single_log(serde_json::json!({"ok": true}), "my-log");
    let v = to_json(outcome).unwrap();
    // `into_cli_compatible_json` wraps Outcome as `{logs, result}`.
    assert!(v.get("logs").is_some(), "expected a `logs` field: {v}");
    assert!(
        v.get("result").is_some(),
        "expected a `result` envelope for the data: {v}"
    );
    assert_eq!(v["logs"][0], "my-log");
    assert_eq!(v["result"]["ok"], true);
}
