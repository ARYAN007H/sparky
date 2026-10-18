use super::*;

fn iteration_started() -> AgentProgress {
    AgentProgress::IterationStarted {
        iteration: 1,
        max_iterations: 50,
    }
}

fn model_call_completed(subagent_task_id: Option<&str>) -> AgentProgress {
    AgentProgress::ModelCallCompleted {
        model: "m".into(),
        provider_id: "managed".into(),
        subagent_task_id: subagent_task_id.map(str::to_string),
        input: None,
        output: None,
        iteration: 1,
        input_tokens: 0,
        output_tokens: 0,
        cached_input_tokens: 0,
        cache_creation_tokens: 0,
        reasoning_tokens: 0,
        cost_usd: None,
    }
}

fn tool_started(call_id: &str) -> AgentProgress {
    AgentProgress::ToolCallStarted {
        call_id: call_id.into(),
        tool_name: "shell".into(),
        arguments: serde_json::json!({}),
        iteration: 1,
        display_label: None,
        display_detail: None,
    }
}

fn tool_completed(call_id: &str) -> AgentProgress {
    AgentProgress::ToolCallCompleted {
        call_id: call_id.into(),
        tool_name: "shell".into(),
        success: true,
        output_chars: 0,
        output: String::new(),
        arguments: None,
        elapsed_ms: 0,
        iteration: 1,
        failure: None,
        display_label: None,
        display_detail: None,
        structured: None,
    }
}

fn text(delta: &str) -> AgentProgress {
    AgentProgress::TextDelta {
        delta: delta.into(),
        iteration: 1,
    }
}

use std::time::Duration;

#[test]
fn parse_idle_window_defaults_and_disables() {
    assert_eq!(parse_idle_window(None), Duration::from_secs(900));
    assert_eq!(parse_idle_window(Some("x")), Duration::from_secs(900));
    assert_eq!(parse_idle_window(Some(" 60 ")), Duration::from_secs(60));
    // Disabled stands for a window no turn reaches.
    assert!(parse_idle_window(Some("0")) > Duration::from_secs(365 * 24 * 3600));
}

fn signal(tracker: &mut WorkTracker, event: &AgentProgress) -> Option<&'static str> {
    tracker.observe(event).map(|signal| match signal {
        TurnSignal::WorkStarted => "started",
        TurnSignal::WorkFinished => "finished",
        TurnSignal::Progress(_) => "progress",
    })
}

/// A provider retrying a failed call is silent for minutes while the turn is
/// healthy: an open model call is work in flight until it completes.
#[test]
fn an_open_model_call_is_work_until_it_completes() {
    let mut tracker = WorkTracker::default();
    assert_eq!(signal(&mut tracker, &iteration_started()), Some("started"));
    assert_eq!(signal(&mut tracker, &text("hi")), None);
    assert_eq!(
        signal(&mut tracker, &model_call_completed(None)),
        Some("finished")
    );
}

#[test]
fn a_subagent_model_call_does_not_close_the_parent_call() {
    let mut tracker = WorkTracker::default();
    signal(&mut tracker, &iteration_started());
    assert_eq!(
        signal(&mut tracker, &model_call_completed(Some("task-1"))),
        None
    );
}

/// A long, silent shell call is in flight, not idle, and parallel calls stay
/// open until the last one completes.
#[test]
fn open_tool_calls_are_work_until_the_last_one_completes() {
    let mut tracker = WorkTracker::default();
    assert_eq!(signal(&mut tracker, &tool_started("a")), Some("started"));
    assert_eq!(signal(&mut tracker, &tool_started("b")), None);
    assert_eq!(signal(&mut tracker, &tool_completed("a")), None);
    assert_eq!(signal(&mut tracker, &tool_completed("b")), Some("finished"));
}

/// The model call that asked for a tool returned: a tool starting hands the
/// open work over without a gap.
#[test]
fn a_tool_start_closes_the_model_call_that_asked_for_it() {
    let mut tracker = WorkTracker::default();
    signal(&mut tracker, &iteration_started());
    assert_eq!(signal(&mut tracker, &tool_started("a")), None);
    assert_eq!(signal(&mut tracker, &tool_completed("a")), Some("finished"));
}

/// A lost `ToolCallCompleted` does not leave the call open: the next model
/// call proves the tool round finished.
#[test]
fn the_next_model_call_closes_a_tool_whose_completion_was_lost() {
    let mut tracker = WorkTracker::default();
    signal(&mut tracker, &tool_started("a"));
    // `tool_completed("a")` is dropped under backpressure.
    assert_eq!(signal(&mut tracker, &iteration_started()), None);
    assert_eq!(
        signal(&mut tracker, &model_call_completed(None)),
        Some("finished")
    );
}

#[test]
fn turn_completed_closes_everything_still_open() {
    let mut tracker = WorkTracker::default();
    signal(&mut tracker, &iteration_started());
    signal(&mut tracker, &tool_started("a"));
    assert_eq!(
        signal(&mut tracker, &AgentProgress::turn_completed(1)),
        Some("finished")
    );
}

#[test]
fn stuck_point_names_where_the_turn_went_quiet() {
    let activity = TurnActivity::default();
    activity.observe(&AgentProgress::TurnContent {
        input: Some("hi".into()),
        output: None,
    });
    assert_eq!(activity.stuck_point(), StuckPoint::BeforeRun);

    activity.observe(&iteration_started());
    activity.observe(&model_call_completed(None));
    activity.observe(&AgentProgress::TurnCostUpdated {
        model: "m".into(),
        iteration: 1,
        input_tokens: 0,
        output_tokens: 0,
        cached_input_tokens: 0,
        total_usd: 0.0,
    });
    assert_eq!(
        activity.stuck_point(),
        StuckPoint::BetweenSteps {
            after: "the model replied".into()
        },
        "a cost tick is bookkeeping, not a step"
    );

    activity.observe(&tool_started("a"));
    activity.observe(&tool_completed("a"));
    assert_eq!(
        activity.stuck_point(),
        StuckPoint::BetweenSteps {
            after: "the `shell` tool finished".into()
        }
    );

    activity.observe(&AgentProgress::TurnCompleted {
        iterations: 1,
        stop: None,
    });
    assert_eq!(activity.stuck_point(), StuckPoint::AfterRun);
}

#[test]
fn the_turn_context_is_what_the_turn_recorded() {
    let activity = TurnActivity::default();
    assert!(activity.turn_context().0.is_none());
    let config = Arc::new(crate::config::Config::default());
    activity.remember_turn(config.clone(), Some("de".into()));
    let (seen, locale) = activity.turn_context();
    assert!(Arc::ptr_eq(&seen.expect("config recorded"), &config));
    assert_eq!(locale.as_deref(), Some("de"));
}

