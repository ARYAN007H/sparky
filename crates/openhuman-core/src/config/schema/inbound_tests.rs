use super::*;

#[test]
fn defaults_route_through_the_shipped_queues_in_storage() {
    let c = InboundConfig::default();
    assert!(c.enabled);
    assert_eq!(c.queues_path, None);
    assert_eq!(c.pipeline_path, None);
    assert_eq!(c.queue_url, None);
    assert_eq!(c.triage_timeout_ms, DEFAULT_TRIAGE_TIMEOUT_MS);
}

#[test]
fn a_partial_section_keeps_the_other_defaults() {
    let c: InboundConfig =
        toml::from_str("enabled = false\nqueues_path = \"/etc/oh/queues.json\"").unwrap();
    assert!(!c.enabled);
    assert_eq!(c.queues_path, Some(PathBuf::from("/etc/oh/queues.json")));
    assert_eq!(c.triage_timeout_ms, DEFAULT_TRIAGE_TIMEOUT_MS);
}
