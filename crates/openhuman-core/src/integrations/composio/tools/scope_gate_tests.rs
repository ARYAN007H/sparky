use super::*;

fn only(read: bool, write: bool, admin: bool) -> UserScopePref {
    UserScopePref { read, write, admin }
}

#[test]
fn a_curated_write_action_follows_the_write_flag() {
    let allowed = decide_visibility("gmail", "GMAIL_SEND_EMAIL", only(true, true, false));
    assert!(matches!(allowed, ToolDecision::Allow));
    let blocked = decide_visibility("gmail", "GMAIL_SEND_EMAIL", only(true, false, false));
    assert!(matches!(
        blocked,
        ToolDecision::BlockedByScope {
            scope: ToolScope::Write
        }
    ));
}

#[test]
fn an_action_outside_a_curated_catalog_is_not_curated() {
    let decision = decide_visibility("gmail", "GMAIL_NOT_A_REAL_ACTION", UserScopePref::default());
    assert!(matches!(decision, ToolDecision::NotCurated));
}

#[test]
fn an_uncatalogued_toolkit_falls_back_to_the_verb_heuristic() {
    let read = decide_visibility(
        "flowstestkit",
        "FLOWSTESTKIT_GET_THING",
        only(true, false, false),
    );
    assert!(matches!(read, ToolDecision::PassthroughCheckScope { .. }));
    let admin = decide_visibility(
        "flowstestkit",
        "FLOWSTESTKIT_DELETE_THING",
        only(true, true, false),
    );
    assert!(matches!(
        admin,
        ToolDecision::BlockedByScope {
            scope: ToolScope::Admin
        }
    ));
}

#[test]
fn unknown_slugs_default_to_write_for_the_sandbox_gate() {
    assert_eq!(resolve_action_scope_sync("  "), ToolScope::Write);
    assert!(action_mutates_external_state("   "));
}

#[test]
fn the_scope_error_names_the_toolkit_and_the_unlock_path() {
    let message = scope_error_message(
        "GMAIL_SEND_EMAIL",
        ToolScope::Write,
        only(true, false, false),
    );
    assert!(message.contains("Connections → gmail → write"), "{message}");
    assert!(message.contains("write=false"), "{message}");
}
