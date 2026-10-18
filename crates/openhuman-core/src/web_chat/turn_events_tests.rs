use super::*;
use crate::core::runtime::DispatchCtx;
use crate::web_chat::WebChannelEvent;
use tinyagents_harness::turn_runner::{QueueDisposition, ScopeKey};

fn request(id: &str, disposition: QueueDisposition) -> TurnRequest<QueuedPayload> {
    TurnRequest {
        scope: ScopeKey {
            agent_id: String::new(),
            profile_id: None,
        },
        thread_id: format!("events-thread-{id}"),
        request_id: id.to_owned(),
        text: "what is up".to_owned(),
        payload: QueuedPayload {
            ctx: Some(DispatchCtx::ambient()),
            client_id: "starter".to_owned(),
            ..QueuedPayload::default()
        },
        host_authored: false,
        disposition,
        deadline: Some(Duration::from_secs(42)),
        idle_timeout: Some(Duration::from_secs(7)),
    }
}

fn events() -> (Arc<Shared>, WebEvents) {
    let shared = Arc::new(Shared::default());
    (shared.clone(), WebEvents::new(shared))
}

async fn events_for(
    rx: &mut tokio::sync::broadcast::Receiver<WebChannelEvent>,
    request_id: &str,
    count: usize,
) -> Vec<WebChannelEvent> {
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut seen = Vec::new();
        while seen.len() < count {
            match rx.recv().await {
                Ok(event) if event.request_id == request_id => seen.push(event),
                Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(error) => panic!("bus closed: {error}"),
            }
        }
        seen
    })
    .await
    .expect("events within timeout")
}

fn done_result(text: &str) -> super::super::types::WebChatTaskResult {
    super::super::types::WebChatTaskResult {
        full_response: text.to_owned(),
        citations: Vec::new(),
        usage: None,
        workspace_dir: std::path::PathBuf::from("/tmp/turn-events"),
        timing: Some(tinychannels_bus::web::TurnTimingSnapshot {
            total_ms: Some(1_000),
            ..Default::default()
        }),
    }
}

#[test]
fn the_runners_guard_failures_classify_as_a_graceful_turn_timeout() {
    for guard in [RUNNER_DEADLINE, RUNNER_IDLE] {
        let (class, message) = WebFailures.classify(guard);
        assert_eq!(class, "turn_timeout", "{guard}");
        assert!(!message.contains("openhuman_turn_wall_clock_timeout"));
    }
    let (class, _) = WebFailures.classify("OpenAI API error (429): slow down");
    assert_ne!(class, "turn_timeout");
}

#[test]
fn a_guard_failure_reports_the_window_that_fired() {
    let req = request("window", QueueDisposition::Interrupt);
    assert!(host_error(RUNNER_DEADLINE, Some(&req)).contains("42s"));
    assert!(host_error(RUNNER_IDLE, Some(&req)).contains("7s"));
    assert_eq!(host_error("boom", Some(&req)), "boom");
}

#[tokio::test]
async fn a_completed_turn_announces_one_chat_done_with_its_timing() {
    let mut rx = crate::web_chat::subscribe_web_channel_events();
    let (shared, sink) = events();
    let req = request("events-done", QueueDisposition::Parallel);
    shared.with("events-done", |slot| {
        slot.result = Some(done_result("all done"))
    });
    sink.terminal(
        &req,
        TerminalEvent::Completed {
            request_id: "events-done".into(),
            reply_id: "reply".into(),
        },
    )
    .await;
    let done = events_for(&mut rx, "events-done", 1).await.remove(0);
    assert_eq!(done.event, "chat_done");
    assert_eq!(done.client_id, "starter");
    assert_eq!(done.full_response.as_deref(), Some("all done"));
    assert_eq!(done.timing.expect("timing").total_ms, Some(1_000));
    assert_eq!(done.seq, None, "terminal events sit outside the seq stream");
    assert!(shared.take("events-done").is_none(), "the slot is released");
}

#[tokio::test]
async fn a_failed_turn_announces_a_classified_chat_error() {
    let mut rx = crate::web_chat::subscribe_web_channel_events();
    let (_shared, sink) = events();
    let req = request("events-failed", QueueDisposition::Interrupt);
    sink.terminal(
        &req,
        TerminalEvent::Failed {
            request_id: "events-failed".into(),
            message: "ignored".into(),
            class: "ignored".into(),
            error: RUNNER_IDLE.into(),
        },
    )
    .await;
    let error = events_for(&mut rx, "events-failed", 1).await.remove(0);
    assert_eq!(error.event, "chat_error");
    assert_eq!(error.error_type.as_deref(), Some("turn_timeout"));
    assert_eq!(error.error_retryable, Some(true));
    assert!(error.copy_key.is_some());
}

#[tokio::test]
async fn a_stopped_turn_announces_cancelled_to_the_client_that_stopped_it() {
    let mut rx = crate::web_chat::subscribe_web_channel_events();
    let (shared, sink) = events();
    let req = request("events-stop", QueueDisposition::Interrupt);
    shared.note_cancel(
        "events-stop",
        CancelNote {
            user_stop: true,
            client_id: Some("stopper".into()),
            superseded_by: None,
        },
    );
    sink.terminal(
        &req,
        TerminalEvent::Cancelled {
            request_id: "events-stop".into(),
        },
    )
    .await;
    let seen = events_for(&mut rx, "events-stop", 2).await;
    assert_eq!(seen[0].event, "chat_error");
    assert_eq!(seen[0].error_type.as_deref(), Some("cancelled"));
    assert_eq!(seen[0].message.as_deref(), Some("Cancelled"));
    assert_eq!(seen[1].event, "chat_cancelled");
    assert_eq!(seen[1].cancel_reason.as_deref(), Some("user_stop"));
    assert!(seen.iter().all(|event| event.client_id == "stopper"));
}

#[tokio::test]
async fn a_superseded_turn_names_its_replacement() {
    let mut rx = crate::web_chat::subscribe_web_channel_events();
    let (shared, sink) = events();
    let req = request("events-old", QueueDisposition::Interrupt);
    shared.note_cancel(
        "events-old",
        CancelNote {
            superseded_by: Some("events-new".into()),
            client_id: Some("interrupter".into()),
            ..CancelNote::default()
        },
    );
    sink.terminal(
        &req,
        TerminalEvent::Cancelled {
            request_id: "events-old".into(),
        },
    )
    .await;
    let seen = events_for(&mut rx, "events-old", 2).await;
    assert_eq!(
        seen[0].message.as_deref(),
        Some("Cancelled by newer request")
    );
    assert_eq!(seen[1].cancel_reason.as_deref(), Some("superseded"));
    assert_eq!(seen[1].superseded_by.as_deref(), Some("events-new"));
    assert_eq!(seen[1].client_id, "interrupter");
}

#[tokio::test]
async fn a_queued_message_dropped_before_it_started_is_silent() {
    let mut rx = crate::web_chat::subscribe_web_channel_events();
    let (shared, sink) = events();
    let queued = request("events-queued", QueueDisposition::Followup);
    shared.mark_queued("events-queued");
    sink.terminal(
        &queued,
        TerminalEvent::Cancelled {
            request_id: "events-queued".into(),
        },
    )
    .await;
    // A later terminal for a sibling proves the queued one published nothing.
    let sibling = request("events-sibling", QueueDisposition::Interrupt);
    sink.terminal(
        &sibling,
        TerminalEvent::Cancelled {
            request_id: "events-sibling".into(),
        },
    )
    .await;
    let first = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match rx.recv().await {
                Ok(event)
                    if event.request_id == "events-queued"
                        || event.request_id == "events-sibling" =>
                {
                    return event.request_id;
                }
                Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(error) => panic!("bus closed: {error}"),
            }
        }
    })
    .await
    .expect("sibling announces");
    assert_eq!(first, "events-sibling");
}

#[tokio::test]
async fn the_reply_store_has_nothing_to_write_for_a_turn_without_a_result() {
    let shared = Arc::new(Shared::default());
    let store = WebReplies::new(shared);
    let req = request("replies-none", QueueDisposition::Interrupt);
    let result = TurnResult {
        reply: "x".into(),
        data: serde_json::Value::Null,
    };
    store
        .persist(&req, "reply", &result)
        .await
        .expect("a no-op");
}

#[tokio::test]
async fn a_turn_stopped_before_its_config_resolved_cannot_be_closed() {
    let shared = Arc::new(Shared::default());
    let summarizer = WebSummarizer::new(shared.clone());
    let req = request("close-none", QueueDisposition::Interrupt);
    assert!(
        summarizer
            .grounded_close(&req, RUNNER_DEADLINE)
            .await
            .is_none()
    );
    assert!(summarizer.grounded_close(&req, RUNNER_IDLE).await.is_none());
}

#[tokio::test]
async fn a_closed_turn_keeps_the_reply_for_the_store_and_the_terminal() {
    let tmp = tempfile::tempdir().expect("workspace");
    let config = crate::config::Config {
        workspace_dir: tmp.path().join("workspace"),
        action_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("config.toml"),
        ..Default::default()
    };
    let shared = Arc::new(Shared::default());
    shared.with("close-ok", |slot| {
        slot.activity
            .remember_turn(Arc::new(config), Some("de".into()));
    });
    let summarizer = WebSummarizer::new(shared.clone());
    let req = request("close-ok", QueueDisposition::Interrupt);
    let closed = summarizer
        .grounded_close(&req, RUNNER_IDLE)
        .await
        .expect("a turn with a config is closed");
    assert!(
        closed.reply.contains("stopped this turn"),
        "{}",
        closed.reply
    );
    let kept = shared
        .with("close-ok", |slot| slot.result.clone())
        .expect("reply kept for the terminal");
    assert_eq!(kept.full_response, closed.reply);
}

#[test]
fn sentry_policy_is_a_pure_predicate_over_the_formatted_error() {
    assert!(sentry_suppression_reason(&turn_timeout_error_message(5)).is_some());
    assert!(sentry_suppression_reason("provider exploded").is_none());
    assert_eq!(timeout_bound_tag("provider exploded"), "none");
}
