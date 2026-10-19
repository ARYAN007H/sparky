use super::*;

#[test]
fn workspace_binding_clears_a_previous_turns_workspace() {
    let previous = tinytools::WorkspaceDescriptor::new("/previous/thread");

    assert_eq!(WorkspaceBinding::Clear.update(), Some(None));
    assert!(previous.root.ends_with("/previous/thread"));
}

#[test]
fn workspace_binding_can_preserve_or_replace_the_session_workspace() {
    let previous = tinytools::WorkspaceDescriptor::new("/configured");
    let requested = tinytools::WorkspaceDescriptor::new("/thread");

    assert_eq!(WorkspaceBinding::Preserve.update(), None);
    assert_eq!(
        WorkspaceBinding::Set(requested.clone()).update(),
        Some(Some(requested))
    );
}
