use super::*;
use tinybus::broker::Broker;
use tinybus::module::ModuleHost;

/// Every registry id this crate can link, in registration order.
const ALL_IDS: [&str; 11] = [
    "tinybox",
    "tinychannels",
    "tinycomputer",
    "tinyconnectors",
    "tinydocs",
    "tinyhosts",
    "tinyjuice",
    "tinymcp",
    "tinysearch",
    "tinyvoice",
    "tinywallet",
];

fn enabled_ids() -> Vec<&'static str> {
    [
        (cfg!(feature = "tinybox"), "tinybox"),
        (cfg!(feature = "tinychannels"), "tinychannels"),
        (cfg!(feature = "tinycomputer"), "tinycomputer"),
        (cfg!(feature = "tinyconnectors"), "tinyconnectors"),
        (cfg!(feature = "tinydocs"), "tinydocs"),
        (cfg!(feature = "tinyhosts"), "tinyhosts"),
        (cfg!(feature = "tinyjuice"), "tinyjuice"),
        (cfg!(feature = "tinymcp"), "tinymcp"),
        (cfg!(feature = "tinysearch"), "tinysearch"),
        (cfg!(feature = "tinyvoice"), "tinyvoice"),
        (cfg!(feature = "tinywallet"), "tinywallet"),
    ]
    .into_iter()
    .filter_map(|(on, id)| on.then_some(id))
    .collect()
}

#[test]
fn every_enabled_feature_registers_exactly_its_own_id() {
    let ids: Vec<&str> = linked_modules().iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, enabled_ids());
    assert!(ids.iter().all(|id| ALL_IDS.contains(id)));
}

#[test]
fn install_is_idempotent() {
    install();
    install();
}

#[test]
fn install_makes_every_enabled_factory_visible_to_the_core() {
    install();
    for id in enabled_ids() {
        assert!(
            openhuman_rpc::embed::modules::is_linked_registered(id),
            "{id} was not registered"
        );
    }
}

/// Each factory yields a module whose manifest names its own registry id and
/// whose descriptor passes admission: the same check a released library gets,
/// with the executable's digest as attestation.
#[test]
fn every_enabled_factory_builds_a_module_that_names_its_own_id() {
    for (id, factory) in linked_modules() {
        let module = factory().unwrap_or_else(|error| panic!("{id}: factory failed: {error}"));
        assert_eq!(module.manifest.module.name, id, "{id}: manifest name");
        assert!(
            module
                .manifest
                .bus_name
                .as_str()
                .starts_with("ai.tinyhumans"),
            "{id}: bus name {}",
            module.manifest.bus_name.as_str()
        );
    }
}

/// Linking several modules must not make them share state: each is admitted by
/// its own host, and a module whose required interfaces the bare host does not
/// serve is refused for exactly that reason, never for a duplicate symbol or a
/// descriptor fault.
#[test]
fn every_enabled_module_is_admitted_or_refused_only_for_its_host_interfaces() {
    for (id, factory) in linked_modules() {
        let module = factory().unwrap();
        let host = ModuleHost::new(Broker::new());
        match host.attach_linked_with_config(module, serde_json::json!({})) {
            Ok(info) => assert_eq!(info.manifest.module.name, id),
            Err(error) => {
                let message = error.to_string();
                assert!(
                    message.contains("required interface has no provider"),
                    "{id}: unexpected admission refusal: {message}"
                );
            }
        }
    }
}

/// Two modules can sit in one host: the descriptors and manifests of different
/// modules do not collide on a bus name or object path.
#[test]
fn enabled_modules_claim_distinct_bus_names_and_object_paths() {
    let mut bus_names = std::collections::BTreeSet::new();
    let mut paths = std::collections::BTreeSet::new();
    for (id, factory) in linked_modules() {
        let module = factory().unwrap();
        assert!(
            bus_names.insert(module.manifest.bus_name.as_str().to_string()),
            "{id}: duplicate bus name"
        );
        assert!(
            paths.insert(module.manifest.object_path.as_str().to_string()),
            "{id}: duplicate object path"
        );
    }
}
