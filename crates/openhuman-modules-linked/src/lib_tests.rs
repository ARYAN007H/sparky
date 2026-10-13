use super::*;

#[test]
fn every_enabled_feature_registers_exactly_its_own_id() {
    let ids: Vec<&str> = linked_modules().iter().map(|(id, _)| *id).collect();
    let expected: Vec<&str> = [
        cfg!(feature = "tinychannels").then_some("tinychannels"),
        cfg!(feature = "tinydocs").then_some("tinydocs"),
        cfg!(feature = "tinyjuice").then_some("tinyjuice"),
    ]
    .into_iter()
    .flatten()
    .collect();
    assert_eq!(ids, expected);
}

#[test]
fn install_is_idempotent() {
    install();
    install();
}

#[cfg(feature = "tinychannels")]
#[test]
fn install_makes_the_factory_visible_to_the_core() {
    install();
    assert!(openhuman_rpc::embed::modules::is_linked_registered(
        "tinychannels"
    ));
}

#[cfg(feature = "tinyjuice")]
#[test]
fn tinyjuice_factory_is_visible_to_the_core() {
    install();
    assert!(openhuman_rpc::embed::modules::is_linked_registered(
        "tinyjuice"
    ));
}
