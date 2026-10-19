use super::*;

#[test]
fn workspace_binding_clears_a_previous_turns_workspace() {
    let previous = tinytools::WorkspaceDescriptor::new("/previous/thread");

    assert_eq!(WorkspaceBinding::Clear.apply(Some(previous)), None);
}

#[test]
fn workspace_binding_can_preserve_or_replace_the_session_workspace() {
    let previous = tinytools::WorkspaceDescriptor::new("/configured");
    let requested = tinytools::WorkspaceDescriptor::new("/thread");

    assert_eq!(
        WorkspaceBinding::Preserve.apply(Some(previous.clone())),
        Some(previous)
    );
    assert_eq!(
        WorkspaceBinding::Set(requested.clone()).apply(None),
        Some(requested)
    );
}
