use super::*;
use tempfile::TempDir;

use crate::config::Config;
fn test_config_in(tmp: &TempDir) -> Config {
    let mut cfg = Config::default();
    cfg.workspace_dir = tmp.path().to_path_buf();
    cfg
}

#[test]
fn model_targets_come_from_the_callers_configured_cloud_providers() {
    let config = Config {
        cloud_providers: vec![crate::config::schema::CloudProviderCreds {
            id: "p_custom_test".into(),
            slug: "custom-test".into(),
            label: "Custom Test Provider".into(),
            endpoint: "https://inference.example.test/v1".into(),
            auth_style: crate::config::schema::AuthStyle::None,
            default_model: None,
        }],
        ..Config::default()
    };

    let report = run_models(&config, false).expect("doctor model listing should succeed");

    assert_eq!(report.entries.len(), 1);
    assert_eq!(report.entries[0].provider, "custom-test");
    assert_eq!(report.entries[0].outcome, ModelProbeOutcome::Skipped);

    let other_config = Config {
        cloud_providers: vec![crate::config::schema::CloudProviderCreds {
            id: "p_other_test".into(),
            slug: "other-test".into(),
            label: "Other Test Provider".into(),
            endpoint: "https://other.example.test/v1".into(),
            auth_style: crate::config::schema::AuthStyle::None,
            default_model: None,
        }],
        ..Config::default()
    };
    let other_report = run_models(&other_config, false)
        .expect("doctor model listing should use each call's config");
    assert_eq!(other_report.entries.len(), 1);
    assert_eq!(other_report.entries[0].provider, "other-test");
}

#[test]
fn model_targets_without_configured_providers_keep_empty_error() {
    let err = run_models(&Config::default(), false)
        .expect_err("empty provider configuration should retain the existing error");
    assert_eq!(err.to_string(), "No providers available for model probing");
}

#[test]
fn config_validation_warns_no_channels() {
    let config = Config::default();
    let mut items = vec![];
    check_config_semantics(&config, &mut items);
    let ch_item = items.iter().find(|i| i.message.contains("channel"));
    assert!(ch_item.is_some());
    assert_eq!(ch_item.unwrap().severity, Severity::Warn);
}

#[test]
fn truncate_for_display_short() {
    let s = "hello";
    assert_eq!(truncate_for_display(s, 10), s);
}

#[test]
fn truncate_for_display_long() {
    let s = "abcdefghijklmnopqrstuvwxyz";
    let truncated = truncate_for_display(s, 5);
    assert!(truncated.starts_with("abcde"));
    assert!(truncated.ends_with("..."));
}

#[test]
fn embedding_provider_validation_accepts_standard_values() {
    assert_eq!(embedding_provider_validation_error("none"), None);
    assert_eq!(embedding_provider_validation_error("openai"), None);
    assert_eq!(
        embedding_provider_validation_error("custom:https://example.com"),
        None
    );
}

#[test]
fn embedding_provider_validation_rejects_empty_custom_url() {
    let err = embedding_provider_validation_error("custom:   ").expect("should fail");
    assert!(err.contains("non-empty URL"), "{err}");
}

#[test]
fn embedding_provider_validation_rejects_non_http_scheme() {
    let err = embedding_provider_validation_error("custom:file:///tmp/model").expect("should fail");
    assert!(err.contains("http/https"), "{err}");
}

#[test]
fn embedding_provider_validation_rejects_malformed_url() {
    let err = embedding_provider_validation_error("custom:not a url").expect("should fail");
    assert!(err.contains("invalid custom provider URL"), "{err}");
}

#[test]
fn model_matches_accepts_exact_and_tagged_variants() {
    assert!(model_matches("bge-m3", "bge-m3"));
    assert!(model_matches("bge-m3:latest", "bge-m3"));
    assert!(model_matches("bge-m3", "bge-m3:latest"));
    assert!(model_matches("bge-m3:v1.0", "bge-m3"));
}

#[test]
fn model_matches_rejects_different_base_models() {
    assert!(!model_matches("nomic-embed-text:latest", "bge-m3"));
    assert!(!model_matches("bge-m3:latest", "nomic-embed-text"));
    assert!(!model_matches("bge-m3:latest", "bge-m3:v1.0"));
}
