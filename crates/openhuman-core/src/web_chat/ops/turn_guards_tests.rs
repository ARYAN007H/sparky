use super::*;

fn result(text: &str) -> WebChatTaskResult {
    WebChatTaskResult {
        full_response: text.to_string(),
        citations: Vec::new(),
        usage: None,
        workspace_dir: std::path::PathBuf::from("/tmp/turn-guards"),
        timing: None,
    }
}

/// The guarded turn sees the backstop it runs under, so the session turn can
/// hand the harness the deadline it must wind down before.
#[tokio::test]
async fn guarded_turn_sees_the_backstop_deadline() {
    let backstop = Duration::from_secs(900);
    let seen = std::sync::Arc::new(std::sync::Mutex::new(None));
    let seen_in_turn = seen.clone();
    let res = drive_turn_with_deadline(Some(backstop), None, None, async move {
        *seen_in_turn.lock().unwrap() = crate::agent::turn_deadline::current();
        Ok(result("answer"))
    })
    .await
    .expect("turn completes");
    assert_eq!(res.full_response, "answer");
    let deadline = seen.lock().unwrap().expect("deadline scoped on the turn");
    assert_eq!(deadline.backstop(), backstop);
    assert_eq!(deadline.wind_down_after(), Duration::from_secs(780));
    assert!(deadline.wind_down_after() < deadline.hard_stop_after());
    assert!(deadline.hard_stop_after() < backstop);
}

#[tokio::test]
async fn disabled_backstop_scopes_no_deadline() {
    let res = drive_turn_with_deadline(None, None, None, async {
        assert!(crate::agent::turn_deadline::current().is_none());
        Ok(result("ok"))
    })
    .await;
    assert!(res.is_ok());
}

/// If the backstop fires anyway, the caller gets the typed `turn_timeout`
/// error, which is classified to user-facing copy, not silence.
#[tokio::test(start_paused = true)]
async fn backstop_firing_yields_a_classified_turn_timeout_error() {
    let err = drive_turn_with_deadline(Some(Duration::from_secs(900)), None, None, async {
        std::future::pending::<()>().await;
        Ok(result("never"))
    })
    .await
    .expect_err("backstop fires");
    assert!(super::super::super::web_errors::is_outer_backstop_timeout(
        &err
    ));
    let classified = super::super::super::web_errors::classify_inference_error(&err);
    assert_eq!(classified.error_type, "turn_timeout");
    assert!(!classified.message.trim().is_empty());
}

fn text_delta() -> crate::agent::progress::AgentProgress {
    crate::agent::progress::AgentProgress::TextDelta {
        delta: "x".into(),
        iteration: 1,
    }
}

const IDLE: Option<Duration> = Some(Duration::from_secs(900));
const CEILING: Option<Duration> = Some(Duration::from_secs(3_600));

/// A turn that keeps emitting progress runs past the old 900s backstop: the
/// idle guard restarts on every event and the backstop is now the 3600s
/// ceiling.
#[tokio::test(start_paused = true)]
async fn a_turn_that_keeps_progressing_outlives_the_idle_window() {
    let res = drive_turn_with_deadline(CEILING, IDLE, None, async {
        let activity = super::super::super::turn_activity::current().expect("activity scoped");
        for _ in 0..90 {
            tokio::time::sleep(Duration::from_secs(17)).await;
            activity.observe(&text_delta());
        }
        Ok(result("done after 25 minutes"))
    })
    .await
    .expect("idle guard must not fire on a progressing turn");
    assert_eq!(res.full_response, "done after 25 minutes");
}

/// A turn that goes silent with nothing in flight ends after the idle window,
/// long before the ceiling, with the classified `turn_timeout` error.
#[tokio::test(start_paused = true)]
async fn a_silent_turn_ends_after_the_idle_window() {
    let started = tokio::time::Instant::now();
    let err = drive_turn_with_deadline(CEILING, IDLE, None, async {
        std::future::pending::<()>().await;
        Ok(result("never"))
    })
    .await
    .expect_err("idle guard fires");
    let waited = started.elapsed();
    assert!(waited >= Duration::from_secs(900) && waited < Duration::from_secs(3_600));
    assert!(err.contains("no progress for"), "{err}");
    assert!(super::super::super::web_errors::is_outer_backstop_timeout(
        &err
    ));
    let classified = super::super::super::web_errors::classify_inference_error(&err);
    assert_eq!(classified.error_type, "turn_timeout");
}

/// A turn that stays busy forever is still bounded by the fixed ceiling.
#[tokio::test(start_paused = true)]
async fn a_turn_busy_past_the_ceiling_hits_the_backstop() {
    let err = drive_turn_with_deadline(CEILING, IDLE, None, async {
        let activity = super::super::super::turn_activity::current().expect("activity scoped");
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            activity.observe(&text_delta());
        }
    })
    .await
    .expect_err("backstop fires");
    assert!(err.contains("3600s wall-clock budget"), "{err}");
}
