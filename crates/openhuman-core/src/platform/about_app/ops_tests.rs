use super::*;
use crate::config::schema::components::ComponentsConfig;

fn off(name: &str) -> ComponentSet {
    let mut c = ComponentsConfig::default();
    assert!(c.set(name, false));
    ComponentSet::from_config(&c)
}

#[test]
fn capability_of_a_disabled_component_shows_disabled() {
    let cap = super::super::lookup("voice.ptt").expect("voice.ptt");
    assert_ne!(cap.status, CapabilityStatus::Disabled);
    let marked = apply_components(cap, &off("voice"));
    assert_eq!(marked.status, CapabilityStatus::Disabled);
    assert_eq!(marked.id, cap.id);
    // Another component being off leaves it alone.
    assert_eq!(apply_components(cap, &off("wallet")).status, cap.status);
    assert_eq!(
        apply_components(cap, &ComponentSet::all()).status,
        cap.status
    );
}

#[test]
fn domain_mapping_names_real_components() {
    for cap in super::super::all_capabilities() {
        if let Some(name) = component_for_domain(cap.domain) {
            assert!(
                crate::core::runtime::COMPONENT_NAMES.contains(&name),
                "{name}"
            );
        }
    }
}
