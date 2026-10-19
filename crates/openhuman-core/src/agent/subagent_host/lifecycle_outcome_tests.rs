use super::*;

#[test]
fn classified_stop_extension_round_trips_without_the_private_summary() {
    let cause = tinyagents_harness::no_progress::ClassifiedStopCause {
        class: "uncertain_side_effect".into(),
        operation: "web_answer_tool".into(),
        summary: "private URL https://example.com/private?q=secret".into(),
    };
    let extension = encode_classified_stop_cause(&cause);
    let mut neutral = SubagentOutcome::cancelled("task");
    neutral
        .extensions
        .insert(CLASSIFIED_STOP_CAUSE_EXTENSION.into(), extension);

    let serialized = serde_json::to_string(&neutral).expect("neutral outcome serializes");
    assert!(!serialized.contains("example.com"));
    assert!(!serialized.contains("private URL"));
    let reloaded: SubagentOutcome =
        serde_json::from_str(&serialized).expect("neutral outcome reloads");
    let reloaded_extension = reloaded
        .extensions
        .get(CLASSIFIED_STOP_CAUSE_EXTENSION)
        .expect("namespaced stop cause extension");
    let restored = decode_classified_stop_cause(reloaded_extension, "persisted reason")
        .expect("typed cause reloads");

    assert_eq!(restored.class, "uncertain_side_effect");
    assert_eq!(restored.operation, "web_answer_tool");
    assert_eq!(restored.summary, "persisted reason");
}
