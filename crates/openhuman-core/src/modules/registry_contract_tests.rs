//! Registry metadata against each module's bus contract constants.

use super::{component_for, find, ALL, COMPONENT_OF};

/// The interface and object path each contract crate publishes, keyed by the
/// module id the registry uses. This is written out from the contract
/// constants, not from the records, so a record that drifts from its contract
/// (or a contract that is renamed upstream) fails here rather than at the first
/// bus call. Records behind an optional contract crate are listed under the
/// same gate as the crate.
fn contract_names() -> Vec<(&'static str, &'static str, &'static str)> {
    #[allow(unused_mut)]
    let mut names = vec![
        (
            "tinycomputer",
            tinycomputer_bus::names::INTERFACE,
            tinycomputer_bus::names::OBJECT_PATH,
        ),
        (
            "tinysearch",
            tinysearch_bus::names::INTERFACE,
            tinysearch_bus::names::OBJECT_PATH,
        ),
        (
            "tinyjuice",
            tinyjuice_bus::names::BUS_NAME,
            tinyjuice_bus::names::OBJECT_PATH,
        ),
        (
            "tinymcp",
            tinymcp_bus::names::INTERFACE,
            tinymcp_bus::names::OBJECT_PATH,
        ),
        (
            "tinyconnectors",
            tinyconnectors_bus::names::INTERFACE,
            tinyconnectors_bus::names::OBJECT_PATH,
        ),
        (
            "tinychannels",
            tinychannels_bus::names::BUS_NAME,
            tinychannels_bus::names::OBJECT_PATH,
        ),
        // The Authoring identity the module registers under; the scheduler is
        // a second object on it (pinned by `the_flows_record_serves_the_schedule_object`).
        (
            "tinyflows",
            tinyflows_bus::BUS_NAME,
            tinyflows_bus::OBJECT_PATH,
        ),
    ];
    #[cfg(feature = "modules")]
    names.push(("tinybox", tinybox_bus::INTERFACE, tinybox_bus::OBJECT_PATH));
    #[cfg(feature = "documents")]
    names.push((
        "tinydocs",
        tinydocs_bus::names::BUS_NAME,
        tinydocs_bus::names::OBJECT_PATH,
    ));
    #[cfg(feature = "web3")]
    names.push((
        "tinywallet",
        tinywallet_bus::names::BUS_NAME,
        tinywallet_bus::names::OBJECT_PATH,
    ));
    #[cfg(feature = "voice")]
    names.push((
        "tinyvoice",
        tinyvoice_bus::names::BUS_NAME,
        tinyvoice_bus::names::OBJECT_PATH,
    ));
    #[cfg(feature = "hosting")]
    names.push((
        "tinyhosts",
        tinyhosts_bus::BUS_NAME,
        tinyhosts_bus::OBJECT_PATH,
    ));
    #[cfg(feature = "tinymemes")]
    names.push((
        "tinymemes",
        tinymemes_bus::STATE_INTERFACE,
        tinymemes_bus::STATE_PATH,
    ));
    names
}

#[test]
fn every_record_matches_its_contract_constants() {
    let contracts = contract_names();
    assert_eq!(contracts.len(), ALL.len(), "a record has no contract entry");
    for (id, bus_name, object_path) in contracts {
        let record = find(id).unwrap_or_else(|| panic!("{id} has no registry record"));
        assert_eq!(
            record.bus_name, bus_name,
            "{id} bus name drifted from its contract"
        );
        assert_eq!(
            record.object_path, object_path,
            "{id} object path drifted from its contract"
        );
    }
}

#[test]
fn a_drifted_contract_constant_is_reported_as_a_mismatch() {
    // The comparison must be able to fail: a fixture whose constant differs
    // from the record (as a renamed upstream interface would) is not equal.
    let record = find("tinymcp").unwrap();
    let drifted = format!("{}Renamed", tinymcp_bus::names::INTERFACE);
    assert_ne!(record.bus_name, drifted);
}

#[cfg(all(
    feature = "documents",
    feature = "web3",
    feature = "voice",
    feature = "hosting",
    feature = "tinymemes"
))]
#[test]
fn a_product_feature_build_carries_all_thirteen_records() {
    assert_eq!(ALL.len(), 13);
    for id in [
        "tinycomputer",
        "tinysearch",
        "tinydocs",
        "tinywallet",
        "tinyjuice",
        "tinyvoice",
        "tinymcp",
        "tinyconnectors",
        "tinybox",
        "tinychannels",
        "tinyflows",
        "tinyhosts",
        "tinymemes",
    ] {
        assert!(find(id).is_some(), "{id} missing from the registry");
    }
}

#[test]
fn every_record_maps_to_a_real_component() {
    for record in ALL {
        let component = component_for(record.id)
            .unwrap_or_else(|| panic!("module '{}' has no component mapping", record.id));
        assert!(
            crate::core::runtime::COMPONENT_NAMES.contains(&component),
            "'{component}' is not a [components] name"
        );
    }
    for (id, _) in COMPONENT_OF {
        assert!(
            ALL.iter().any(|r| r.id == *id)
                || !cfg!(feature = "modules")
                || matches!(
                    *id,
                    "tinydocs" | "tinywallet" | "tinyvoice" | "tinyhosts" | "tinymemes"
                ),
            "mapping for unknown module '{id}'"
        );
    }
}

/// The scheduler is a second interface on the same module, and the host calls
/// it by the contract's own names: the record carries the Authoring identity,
/// and the Schedule object and callback interfaces are distinct from it and from
/// each other.
#[test]
fn the_flows_record_serves_the_schedule_object() {
    use tinyflows_bus::schedule;
    let record = find("tinyflows").expect("tinyflows is registered");
    assert_eq!(record.bus_name, schedule::BUS_NAME);
    assert_ne!(record.object_path, schedule::OBJECT_PATH);
    assert_ne!(schedule::INTERFACE, schedule::CALLBACK_INTERFACE);
    assert_ne!(schedule::INTERFACE, schedule::DOCUMENT_INTERFACE);
    assert_eq!(schedule::CONTRACT_VERSION, 6, "the host speaks contract v6");
    assert_eq!(component_for("tinyflows"), Some("tinyflows"));
}

/// The pins are copied from the published v0.8.27 `checksum.toml`.
#[test]
fn the_flows_record_pins_every_host_of_one_release() {
    let record = find("tinyflows").unwrap();
    assert_eq!(record.version, "0.8.27");
    assert_eq!(record.assets.len(), 11);
    let expected = [
        (
            "tinyflows-module-0.8.27-macos-26-arm64.tar.gz",
            "99e05d3e7612f0f65315533e74bf3b08ff804102ff8439d51e9b19f522748063",
        ),
        (
            "tinyflows-module-0.8.27-macos-26-x86_64.tar.gz",
            "0614266a74bd830f720c319e8cb8cc6442b86e992b3fea7af0f5b63c2ed02164",
        ),
        (
            "tinyflows-module-0.8.27-macos-15-arm64.tar.gz",
            "60ba1f5ef4f6198f886e980eabe3400cc70ad2f765e797b1f16cb22d4998925a",
        ),
        (
            "tinyflows-module-0.8.27-macos-15-x86_64.tar.gz",
            "9304f1b065c9def4a1e8f28c05a70697c5da49e48fdecd0594544bc831658196",
        ),
        (
            "tinyflows-module-0.8.27-ubuntu-24.04-x86_64.tar.gz",
            "0b4345b8747c7f855039978873ad01f1f8b8c7a0356bfc73cc159cb269622843",
        ),
        (
            "tinyflows-module-0.8.27-ubuntu-24.04-arm64.tar.gz",
            "af8bd91845c0d13b15d2c26ee0059d312bf4ec383d2bd50041bbf7e41ecaf834",
        ),
        (
            "tinyflows-module-0.8.27-ubuntu-22.04-x86_64.tar.gz",
            "6d7b89d9d52f5b6024e421fd17d9a4391448e29708835c3380a022e9beccdafe",
        ),
        (
            "tinyflows-module-0.8.27-ubuntu-22.04-arm64.tar.gz",
            "74fb1d0b12d87ae7830ade2f6e043707ee356a54346b560adb1fc5d66785de2f",
        ),
        (
            "tinyflows-module-0.8.27-windows-2025-x86_64.zip",
            "b116e7eebaff050333d1412940d13a16cea711261dff8f7503589fef087d36b8",
        ),
        (
            "tinyflows-module-0.8.27-windows-2022-x86_64.zip",
            "9b12b158dd57a5efc6b59e3871301e650e120d6969775318ed7a8d98b8ef94ed",
        ),
        (
            "tinyflows-module-0.8.27-windows-11-arm64.zip",
            "7ea7cf80ee0f65f56485ce1e7066ec7591098ecbae259baff85a7ab53f2d3305",
        ),
    ];
    for (asset, (archive, sha256)) in record.assets.iter().zip(expected) {
        assert_eq!(asset.archive, archive);
        assert_eq!(asset.sha256, sha256, "{}", asset.archive);
    }
}
