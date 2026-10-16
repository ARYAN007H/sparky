//! `write_pause_checkpoint` — the pause-persistence contract (#5928).
//!
//! Every failure branch returns `None`, and that `None` is what stops the
//! runner reporting a resumable `AwaitingUser` for a pause it never saved.
//! Before this, all three failures were logged at `warn` and then dropped, so
//! the orchestrator relayed a `task_id` and asked the user to answer a question
//! whose answer had nowhere to go.

// ── task_id is untrusted on its way into a path (tinysweeper, #5951) ────────
//
// `continue_subagent` takes `task_id` from its tool arguments — model-authored
// — and feeds it back through `SubagentRunOptions::task_id`, so a re-paused
// child writes a checkpoint under a name the model chose. Joined unchecked,
// `../` walks the write out of the checkpoint directory entirely.

#[test]
fn a_traversing_task_id_is_refused_rather_than_written() {
    for bad in [
        "../../etc/cron.d/pwn",
        "a/b",
        "a\\b",
        "..",
        ".",
        "",
        "with space",
        "sub-\u{00e9}",
    ] {
        assert!(
            !super::is_safe_task_id(bad),
            "{bad:?} must be rejected as a checkpoint filename"
        );
    }
}

#[test]
fn the_ids_this_system_actually_mints_are_accepted() {
    // The guard is an allow-list, so it has to admit every shape in real use —
    // otherwise it would break resumption instead of hardening it.
    for good in [
        "sub-2b9d1f4e-0c3a-4f10-9c2e-6a7b8c9d0e1f",
        "subsess-2b9d1f4e-0c3a-4f10-9c2e-6a7b8c9d0e1f",
        "task-fleet-b",
        "t1",
        "t_steer",
    ] {
        assert!(
            super::is_safe_task_id(good),
            "{good:?} is a real id shape and must be accepted"
        );
    }
}

#[test]
fn a_legacy_checkpoint_with_toolkit_override_still_parses() {
    // Checkpoints written before the `toolkit` spawn argument was retired
    // carry a `toolkit_override` key. Resume must still read them.
    let mut value = serde_json::to_value(checkpoint_data("task-legacy")).expect("serialise");
    value
        .as_object_mut()
        .expect("checkpoint is an object")
        .insert("toolkit_override".into(), serde_json::json!("gmail"));
    let parsed: SubagentCheckpointData =
        serde_json::from_value(value).expect("legacy checkpoint parses");
    assert_eq!(parsed.task_id, "task-legacy");
    assert_eq!(parsed.question, "Which region?");
}
