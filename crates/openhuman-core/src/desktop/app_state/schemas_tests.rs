use super::*;
use crate::core::all::{assert_schema_controller_parity, expect_schema_identity};

#[test]
fn app_state_schema_contracts_and_registration_are_preserved() {
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
}
