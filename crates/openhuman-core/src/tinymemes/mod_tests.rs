use super::*;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::threads::store::ConversationMessage;

/// Answers every remix with a fixed response, after an optional delay.
struct ScriptedRemixer {
    response: Result<RemixResponse, String>,
    delay: Duration,
    calls: AtomicUsize,
    last: std::sync::Mutex<Option<RemixRequest>>,
}

impl ScriptedRemixer {
    fn new(response: Result<RemixResponse, String>) -> Self {
        Self {
            response,
            delay: Duration::ZERO,
            calls: AtomicUsize::new(0),
            last: std::sync::Mutex::new(None),
        }
    }
}

#[async_trait]
impl Remixer for ScriptedRemixer {
    async fn remix(&self, request: RemixRequest) -> Result<RemixResponse, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        *self.last.lock().unwrap() = Some(request);
        tokio::time::sleep(self.delay).await;
        self.response.clone()
    }
}

fn remixed_response(reply: &str) -> RemixResponse {
    RemixResponse {
        reply: reply.to_owned(),
        remixed: true,
        skipped: None,
        score: Some(7),
        wants_more_memes: false,
    }
}

fn skipped_response(reply: &str, reason: SkipReason) -> RemixResponse {
    RemixResponse {
        reply: reply.to_owned(),
        remixed: false,
        skipped: Some(reason),
        score: None,
        wants_more_memes: false,
    }
}

fn turn(request_id: &str) -> TurnInfo<'_> {
    TurnInfo {
        request_id,
        bucket: 7,
        turn_ms: 1234,
    }
}

fn msg(id: &str, sender: &str, content: &str) -> ConversationMessage {
    ConversationMessage {
        id: id.to_owned(),
        content: content.to_owned(),
        message_type: "text".to_owned(),
        extra_metadata: serde_json::Value::Null,
        sender: sender.to_owned(),
        created_at: "2026-10-09T00:00:00Z".to_owned(),
    }
}

fn thread() -> Vec<ConversationMessage> {
    vec![
        msg("u1", "user", "bhai build fail ho gaya"),
        msg("r1", "agent", "Arre yaar, moye moye"),
    ]
}

fn store(dir: &std::path::Path) -> Arc<remixed::RemixedIds> {
    remixed::for_workspace(dir).unwrap()
}

async fn run(
    remixer: &ScriptedRemixer,
    ids: &Arc<remixed::RemixedIds>,
    request_id: &str,
    budget: Duration,
) -> Option<String> {
    remix_with(
        remixer,
        ids,
        &thread(),
        &turn(request_id),
        Instant::now(),
        "phir se fail",
        "Build failed again.",
        budget,
    )
    .await
}

#[test]
fn the_budget_defaults_to_twenty_seconds() {
    if std::env::var(TIMEOUT_ENV).is_err() {
        assert_eq!(budget(), DEFAULT_TIMEOUT);
    }
}

#[test]
fn only_the_treatment_arm_passes_the_gate() {
    let t = turn("req");
    assert!(!gate(Arm::Disabled, &t));
    assert!(!gate(Arm::Control, &t));
    assert!(gate(Arm::Treatment, &t));
}

#[tokio::test]
async fn a_remixed_reply_is_delivered_and_remembered() {
    let dir = tempfile::tempdir().unwrap();
    let ids = store(dir.path());
    let remixer = ScriptedRemixer::new(Ok(remixed_response("Arre yaar, build phir fail!")));
    let out = run(&remixer, &ids, "req-ok", Duration::from_secs(5)).await;
    assert_eq!(out.as_deref(), Some("Arre yaar, build phir fail!"));
    assert!(ids.contains(&crate::threads::store::run_reply_message_id("req-ok")));

    // The module saw the thread with the earlier remixed reply flagged.
    let request = remixer.last.lock().unwrap().clone().unwrap();
    assert!(request.enabled && !request.deliver);
    assert_eq!(request.reply, "Build failed again.");
    assert_eq!(request.timeout_ms, 5000);
    assert_eq!(request.conversation.last().unwrap().text, "phir se fail");
}

#[tokio::test]
async fn an_unchanged_reply_sends_the_original() {
    let dir = tempfile::tempdir().unwrap();
    let ids = store(dir.path());
    for reason in [
        SkipReason::BelowThreshold,
        SkipReason::Failed,
        SkipReason::Timeout,
    ] {
        let remixer = ScriptedRemixer::new(Ok(skipped_response("Build failed again.", reason)));
        assert_eq!(
            run(&remixer, &ids, "req-skip", Duration::from_secs(5)).await,
            None
        );
    }
    assert!(!ids.contains(&crate::threads::store::run_reply_message_id("req-skip")));
}

#[tokio::test]
async fn a_remix_that_returns_the_same_text_is_not_a_change() {
    let dir = tempfile::tempdir().unwrap();
    let ids = store(dir.path());
    let remixer = ScriptedRemixer::new(Ok(remixed_response("Build failed again. ")));
    assert_eq!(
        run(&remixer, &ids, "req-same", Duration::from_secs(5)).await,
        None
    );
    assert!(!ids.contains(&crate::threads::store::run_reply_message_id("req-same")));
}

#[tokio::test]
async fn a_module_error_sends_the_original() {
    let dir = tempfile::tempdir().unwrap();
    let ids = store(dir.path());
    let remixer = ScriptedRemixer::new(Err("module unavailable".into()));
    assert_eq!(
        run(&remixer, &ids, "req-err", Duration::from_secs(5)).await,
        None
    );
}

#[tokio::test]
async fn a_slow_remix_times_out_to_the_original() {
    let dir = tempfile::tempdir().unwrap();
    let ids = store(dir.path());
    let mut remixer = ScriptedRemixer::new(Ok(remixed_response("too late")));
    remixer.delay = Duration::from_secs(30);
    let out = run(&remixer, &ids, "req-slow", Duration::from_millis(50)).await;
    assert_eq!(out, None);
}

#[tokio::test]
async fn a_disabled_flag_leaves_the_task_reply_alone() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = Config::default();
    config.config_path = dir.path().join("config.toml");
    config.workspace_dir = dir.path().join("workspace");
    let mut reply = String::from("Build failed again.");
    // Disabled arm: no module is touched and nothing changes.
    let out = remix_final_reply(
        Arm::Disabled,
        &config,
        "thread",
        "req",
        "msg",
        &reply,
        Duration::from_millis(5),
    )
    .await;
    assert_eq!(out, None);
    remix_task_reply(
        &config,
        "thread",
        "req",
        "msg",
        &mut reply,
        "placeholder",
        Duration::ZERO,
    )
    .await;
    assert_eq!(reply, "Build failed again.");
}

#[test]
fn the_turn_line_reports_the_outcome_without_user_content() {
    let t = turn("req-1");
    let none = turn_line(&t, 5, "error", None);
    assert!(none.contains("outcome=error") && none.contains("score=-1"));
    let some = turn_line(&t, 9, "remixed", Some(&remixed_response("secret text")));
    assert!(some.contains("score=7") && some.contains("request_id=req-1"));
    assert!(!some.contains("secret text"));
    let skipped = turn_line(
        &t,
        9,
        "unchanged",
        Some(&skipped_response("x", SkipReason::BelowThreshold)),
    );
    assert!(skipped.contains("skipped=below_threshold"));
}

#[test]
fn flag_values_enable_remixing() {
    assert!(flag_enabled("on"));
    assert!(flag_enabled("ab:30"));
    assert!(!flag_enabled("off"));
}
