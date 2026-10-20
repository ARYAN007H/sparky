use super::*;

#[test]
fn profile_capabilities_deny_the_registered_spawn_tool() {
    let capabilities = ProfileCapabilities::new(None, None, None, HashMap::new(), Vec::new(), None);

    for name in tinyagents_definition::SPAWN_TOOL_NAMES
        .iter()
        .copied()
        .chain(["continue_subagent"])
    {
        assert!(
            !capabilities.admits_capability(name),
            "profiles must not admit spawn tool {name}"
        );
    }
}
