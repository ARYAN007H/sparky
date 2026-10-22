//! `finalize_committed_turn` reports the iteration cap only when the turn hit it.

use tinyagents_runtime::{CommitReceipt, ResumeMode, SessionTurnOutcome, TranscriptTurnOptions};

use crate::agent::session_host::runtime_session::post_commit::{
    finalize_committed_turn, TurnIdentity,
};
use crate::agent::session_host::runtime_session::OpenHumanSessionState;

/// Commits one turn whose outcome is `interrupted` and returns the
/// `last_turn_hit_cap` it leaves on the session.
async fn hit_cap_after_interrupted_turn(sidecar_hit_cap: bool) -> bool {
    let context = crate::agent::tinyagents::host::OpenHumanRunContext::new();
    context.session_sidecar.lock().unwrap().hit_cap = sidecar_hit_cap;
    let receipt = CommitReceipt {
        outcome: SessionTurnOutcome {
            history: Vec::new(),
            output: Some("Which city do you mean?".into()),
            // The driver marks a turn interrupted on an early-exit tool
            // (`ask_user_clarification`) as well as on the cap.
            interrupted: true,
        },
        options: TranscriptTurnOptions {
            request_id: None,
            thread_id: None,
            stream: true,
            resume: ResumeMode::Never,
            context,
        },
        transcript: None,
    };
    let state = std::sync::Mutex::new(OpenHumanSessionState::default());
    finalize_committed_turn(
        &state,
        &[],
        TurnIdentity {
            session_id: "post-commit-hit-cap".into(),
            agent_id: "orchestrator".into(),
            channel: "web".into(),
        },
        receipt,
    )
    .await;
    let hit_cap = state.lock().unwrap().last_turn_hit_cap;
    hit_cap
}

#[tokio::test]
async fn a_clarification_early_exit_is_not_reported_as_hitting_the_cap() {
    assert!(
        !hit_cap_after_interrupted_turn(false).await,
        "a turn that ended by asking the user a question was reported as an iteration-cap pause"
    );
}

#[tokio::test]
async fn a_turn_that_hit_the_cap_is_reported_as_hitting_it() {
    assert!(hit_cap_after_interrupted_turn(true).await);
}
