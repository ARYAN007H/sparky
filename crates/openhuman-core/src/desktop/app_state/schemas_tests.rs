use super::*;
use crate::core::all::{
    assert_schema_controller_parity, expect_schema_identity, ControllerHandler,
    ControllerParamContract,
};

#[test]
fn legacy_handlers_keep_their_identity_and_order() {
    let expected: [(&str, ControllerHandler); 2] = [
        ("snapshot", handle_snapshot),
        ("update_local_state", handle_update_local_state),
    ];
    let controllers = all_app_state_registered_controllers();
    assert_eq!(controllers.len(), expected.len());
    for ((function, handler), controller) in expected.iter().zip(&controllers) {
        assert_eq!(controller.schema.namespace, "app_state");
        assert_eq!(controller.schema.function, *function);
        assert!(std::ptr::fn_addr_eq(controller.handler, *handler));
    }
}

pub(super) fn parameter_contracts() -> Vec<ControllerParamContract> {
    use serde_json::json;

    vec![
        ControllerParamContract {
            method: "app_state.snapshot",
            required: &[],
            optional: &[],
            examples: vec![json!({})],
        },
        ControllerParamContract {
            method: "app_state.update_local_state",
            required: &[],
            optional: &["encryptionKey", "onboardingTasks", "keyringConsent"],
            examples: vec![
                json!({}),
                json!({
                    "encryptionKey": null,
                    "onboardingTasks": null,
                    "keyringConsent": null
                }),
                json!({
                    "encryptionKey": "  keep whitespace  ",
                    "onboardingTasks": {
                        "accessibilityPermissionGranted": false,
                        "enabledTools": [],
                        "connectedSources": [],
                        "updatedAtMs": null
                    },
                    "keyringConsent": {
                        "storageMode": "",
                        "consentedAtMs": null
                    }
                }),
            ],
        },
    ]
}

#[test]
fn app_state_schema_contracts_and_registration_are_preserved() {
    crate::core::all::expect_schema_namespace(&all_app_state_controller_schemas(), "app_state");
    assert_eq!(all_app_state_controller_schemas().len(), 2);
    assert_schema_controller_parity(
        &all_app_state_controller_schemas(),
        &all_app_state_registered_controllers(),
        "snapshot",
    );
    let s = app_state_schemas("snapshot");
    expect_schema_identity(&s, "app_state", "snapshot");
    assert!(s.inputs.is_empty());
    assert!(!s.outputs.is_empty());
    let s = app_state_schemas("update_local_state");
    expect_schema_identity(&s, "app_state", "update_local_state");
    assert_eq!(s.inputs.len(), 3);
    for input in &s.inputs {
        assert!(!input.required, "input '{}' should be optional", input.name);
    }
}

#[test]
fn deserialize_update_local_state_params_empty() {
    let params: UpdateLocalStateParams =
        serde_json::from_value(serde_json::Value::Object(Map::new())).unwrap();
    assert!(params.encryption_key.is_none());
    assert!(params.onboarding_tasks.is_none());
}

#[test]
fn deserialize_update_local_state_params_with_values() {
    let mut m = Map::new();
    // encryption_key is Option<Option<String>> — sending a string value sets Some(Some("..."))
    m.insert("encryptionKey".into(), serde_json::json!("my-key"));
    let params: UpdateLocalStateParams =
        serde_json::from_value(serde_json::Value::Object(m)).unwrap();
    assert!(params.encryption_key.is_some());
}

#[test]
fn deserialize_update_local_state_params_with_null_clears_value() {
    let mut m = Map::new();
    m.insert("encryptionKey".into(), serde_json::Value::Null);
    m.insert("onboardingTasks".into(), serde_json::Value::Null);
    let params: UpdateLocalStateParams =
        serde_json::from_value(serde_json::Value::Object(m)).unwrap();
    assert_eq!(params.encryption_key, Some(None));
    assert!(matches!(params.onboarding_tasks, Some(None)));
    assert_eq!(params.keyring_consent, Some(None));
}

#[test]
fn update_local_state_patch_preserves_camel_case_tristate_and_nested_defaults() {
    let missing: UpdateLocalStateParams = serde_json::from_value(serde_json::json!({})).unwrap();
    assert_eq!(missing.encryption_key, None);
    assert_eq!(missing.onboarding_tasks, None);
    assert_eq!(missing.keyring_consent, None);

    let params: UpdateLocalStateParams = serde_json::from_value(serde_json::json!({
        "encryptionKey": "  keep whitespace  ",
        "onboardingTasks": {
            "accessibilityPermissionGranted": false,
            "enabledTools": [],
            "connectedSources": [],
            "updatedAtMs": null,
            "unknownNestedField": true
        },
        "keyringConsent": {
            "storageMode": "",
            "consentedAtMs": 17,
            "unknownNestedField": true
        },
        "unknownOuterField": {"ignored": true},
        "encryption_key": "snake_case_is_not_an_alias"
    }))
    .unwrap();

    assert_eq!(
        params.encryption_key.as_deref(),
        Some(Some("  keep whitespace  "))
    );
    let tasks = params.onboarding_tasks.as_ref().unwrap().as_ref().unwrap();
    assert!(!tasks.accessibility_permission_granted);
    assert!(tasks.enabled_tools.is_empty());
    assert!(tasks.connected_sources.is_empty());
    assert_eq!(tasks.updated_at_ms, None);
    let consent = params.keyring_consent.as_ref().unwrap().as_ref().unwrap();
    assert_eq!(consent.storage_mode, "");
    assert_eq!(consent.consented_at_ms, Some(17));
}

#[test]
fn update_local_state_patch_rejects_wrong_nested_and_outer_types() {
    for value in [
        serde_json::json!({"encryptionKey": false}),
        serde_json::json!({"onboardingTasks": []}),
        serde_json::json!({"keyringConsent": {"consentedAtMs": "17"}}),
    ] {
        assert!(
            serde_json::from_value::<UpdateLocalStateParams>(value).is_err(),
            "malformed app_state patch types are rejected"
        );
    }
}
