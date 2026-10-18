use super::*;

#[test]
fn profile_capabilities_deny_the_registered_spawn_tool() {
    let capabilities = ProfileCapabilities::new(
        None,
        None,
        None,
        HashMap::new(),
        Vec::new(),
        None,
    );

    assert!(!capabilities.admits_capability("spawn_subagent"));
}
