use super::*;

#[test]
fn starts_where_the_pre_turn_step_put_it_and_never_past_the_turn() {
    assert_eq!(InPromptFrom::new(6, 20).get(), 6);
    assert_eq!(InPromptFrom::new(30, 20).get(), 20);
}

#[test]
fn a_fold_moves_it_two_turns_per_new_user_message() {
    let window = InPromptFrom::new(6, 20);
    assert_eq!(
        window.advance_past(["first question", "second question"]),
        10
    );
    assert_eq!(window.get(), 10);
    // Folding the same messages again (a retried or extended compaction)
    // moves nothing.
    assert_eq!(window.advance_past(["second question"]), 10);
    assert_eq!(window.advance_past(Vec::<String>::new()), 10);
    assert_eq!(window.advance_past(["third question"]), 12);
}

#[test]
fn it_stops_at_the_turn_being_answered() {
    let window = InPromptFrom::new(16, 20);
    assert_eq!(window.advance_past(["a", "b", "c", "d", "e"]), 20);
}
