use super::*;

#[test]
fn absent_provider_keeps_default_absent_and_concrete_model_ids_verbatim() {
    assert_eq!(
        crate::inference::registry::provider_default_model("missing-provider"),
        None
    );
    let config = Config::default();
    assert_eq!(
        resolve_provider_model_hint("missing-provider", "vendor/model-id", &config),
        "vendor/model-id"
    );
}

#[test]
fn abstract_hint_markers_do_not_become_raw_model_ids() {
    assert!(is_abstract_model("missing-provider", "hint:unknown"));
    assert!(!is_raw_model("missing-provider", "hint:unknown"));
    assert!(is_raw_model("missing-provider", "vendor/model-id"));
    assert_eq!(role_for_provider_model("missing-provider", "hint:coding"), "coding");
    assert_eq!(role_for_provider_model("missing-provider", "hint:unknown"), "chat");
}
