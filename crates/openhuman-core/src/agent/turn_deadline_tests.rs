use super::*;

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
