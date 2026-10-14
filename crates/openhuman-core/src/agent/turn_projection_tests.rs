use super::*;
use tinyagents_harness::turn_stop::{TurnStopKind, SUBAGENT_ITERATION_CAP_REASON};

const CLASSIFIED_NOTE: &str = "Stopping after 2 attempt(s): failure class \
    `uncertain_side_effect` still blocks operation `web_answer_tool` on \
    `https://example.com/private?q=secret`. Resolve this blocker before retrying.";

#[test]
fn a_classified_breaker_note_keeps_its_class_and_drops_its_scope() {
    let stop = breaker_stop(CLASSIFIED_NOTE);
    assert_eq!(stop.kind, TurnStopKind::Breaker);
    assert_eq!(stop.failure_class.as_deref(), Some("uncertain_side_effect"));
    assert_eq!(stop.operation.as_deref(), Some("web_answer_tool"));
    assert!(!stop.status_message().contains("example.com"));
}

#[test]
fn an_unclassified_breaker_note_falls_back_to_its_stop_reason_kind() {
    let stop = breaker_stop("The last 6 tool calls in a row failed. Last error: token sk-live-123 rejected");
    assert_eq!(stop.kind, TurnStopKind::Breaker);
    assert_eq!(stop.failure_class.as_deref(), Some("varied_failures"));
    assert_eq!(stop.status_message(), "stopped: breaker varied_failures");
}

#[test]
fn classify_prefers_breaker_then_wind_down_then_cap() {
    assert_eq!(classify_stop(None, false, false), None);
    assert_eq!(
        classify_stop(None, true, true).map(|s| s.kind),
        Some(TurnStopKind::WindDown)
    );
    assert_eq!(
        classify_stop(Some(CLASSIFIED_NOTE), true, true).map(|s| s.kind),
        Some(TurnStopKind::Breaker)
    );
}

#[test]
fn an_incomplete_reason_maps_to_cap_or_breaker() {
    assert_eq!(
        stop_from_incomplete(SUBAGENT_ITERATION_CAP_REASON).kind,
        TurnStopKind::IterationCap
    );
    assert_eq!(
        stop_from_incomplete(CLASSIFIED_NOTE).operation.as_deref(),
        Some("web_answer_tool")
    );
}
