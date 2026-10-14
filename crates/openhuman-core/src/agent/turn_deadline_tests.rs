use super::*;
use tinyagents_harness::turn_deadline::parse_backstop;
use std::time::Instant;

#[test]
fn parse_backstop_defaults_and_disables() {
    assert_eq!(parse_backstop(None), Some(Duration::from_secs(900)));
    assert_eq!(
        parse_backstop(Some("garbage")),
        Some(Duration::from_secs(900))
    );
    assert_eq!(
        parse_backstop(Some(" 120 ")),
        Some(Duration::from_secs(120))
    );
    assert_eq!(parse_backstop(Some("0")), None);
}

/// The web backstop defaults to the harness turn ceiling, not the harness's
/// 900s, and keeps the env override and the `0` opt-out.
#[test]
fn web_backstop_defaults_to_an_hour_and_keeps_the_override() {
    let hour = Some(Duration::from_secs(DEFAULT_WEB_TURN_BACKSTOP_SECS));
    assert_eq!(DEFAULT_WEB_TURN_BACKSTOP_SECS, 3_600);
    assert_eq!(parse_web_backstop(None), hour);
    assert_eq!(parse_web_backstop(Some("garbage")), hour);
    assert_eq!(
        parse_web_backstop(Some(" 120 ")),
        Some(Duration::from_secs(120))
    );
    assert_eq!(parse_web_backstop(Some("0")), None);
    let deadline = TurnDeadline::starting_now(hour.unwrap());
    assert_eq!(deadline.wind_down_after(), Duration::from_secs(3_480));
    assert_eq!(deadline.hard_stop_after(), Duration::from_secs(3_540));
}

#[tokio::test]
async fn task_local_scope_is_visible_inside_and_absent_outside() {
    assert!(current().is_none());
    let deadline = TurnDeadline::starting_now(Duration::from_secs(60));
    let seen = with_turn_deadline(Some(deadline), async { current() }).await;
    assert_eq!(seen, Some(deadline));
    assert!(current().is_none());
    assert!(with_turn_deadline(None, async { current() })
        .await
        .is_none());
}

/// The grounded close gets what is left of the backstop minus the commit
/// reserve, and nothing once the reserve itself is reached (#7716).
#[test]
fn close_budget_is_backstop_remainder_minus_reserve() {
    let now = Instant::now();
    let backstop = Duration::from_secs(900);
    let at = |elapsed: u64| TurnDeadline::new(now - Duration::from_secs(elapsed), backstop);
    // The wind-down point of a default turn leaves 120s, 110s of it for the close.
    assert_eq!(close_budget(&at(780), now), Duration::from_secs(110));
    assert_eq!(close_budget(&at(889), now), Duration::from_secs(1));
    assert_eq!(close_budget(&at(890), now), Duration::ZERO);
    assert_eq!(close_budget(&at(950), now), Duration::ZERO);
}
