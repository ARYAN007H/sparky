//! Host projection of harness turn-stop facts.
//!
//! The vocabulary (`TurnStop`, `TurnStopKind`, the breaker-note parsing) is
//! owned by `tinyagents_harness::turn_stop`. OpenHuman keeps only the product
//! mapping of a stop note without a failure class to its stop-reason key
//! ([`StopReasonKind`](crate::agent::session_host::stop_summary::StopReasonKind)).

use tinyagents_harness::turn_stop::TurnStop;

/// The stop-reason key for a breaker note that names no failure class.
pub fn stop_fallback_class(note: &str) -> &'static str {
    crate::agent::session_host::stop_summary::StopReasonKind::from_stop_note(note).key()
}

/// [`TurnStop::breaker`] with the OpenHuman stop-reason fallback.
pub fn breaker_stop(note: &str) -> TurnStop {
    crate::agent::turn_projection::breaker_stop(note, stop_fallback_class)
}

/// [`TurnStop::classify`] with the OpenHuman stop-reason fallback.
pub fn classify_stop(
    breaker_halt: Option<&str>,
    wind_down: bool,
    hit_cap: bool,
) -> Option<TurnStop> {
    TurnStop::classify(breaker_halt, wind_down, hit_cap, stop_fallback_class)
}

/// [`TurnStop::from_incomplete_reason`] with the OpenHuman stop-reason fallback.
pub fn stop_from_incomplete(reason: &str) -> TurnStop {
    crate::agent::turn_projection::stop_from_incomplete(reason, stop_fallback_class)
}

#[cfg(test)]
#[path = "turn_projection_tests.rs"]
mod tests;
