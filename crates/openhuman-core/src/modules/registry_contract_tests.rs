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
    #[cfg(feature = "inbound-queues")]
    names.push((
        "tinyeventqueues",
        tinyeventqueues_bus::INTERFACE,
        tinyeventqueues_bus::OBJECT_PATH,
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
    feature = "inbound-queues"
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
        "tinyeventqueues",
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
                    "tinydocs" | "tinywallet" | "tinyvoice" | "tinyhosts" | "tinyeventqueues"
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
    assert_eq!(schedule::CONTRACT_VERSION, 8, "the host speaks contract v8");
    assert_eq!(component_for("tinyflows"), Some("tinyflows"));
}

/// The pins are copied from the published v0.9.2 `checksum.toml`.
#[test]
fn the_flows_record_pins_every_host_of_one_release() {
    let record = find("tinyflows").unwrap();
    assert_eq!(record.version, "0.9.2");
    assert_eq!(record.assets.len(), 11);
    let expected = [
        (
            "tinyflows-module-0.9.2-macos-26-arm64.tar.gz",
            "283d960f158eb237797d6a8159309e3f2c3635e9b5c93f43f265b4f5add15611",
        ),
        (
            "tinyflows-module-0.9.2-macos-26-x86_64.tar.gz",
            "eee906fa2c97ddd7fd16b8ccef6cd0fe95f14a3352ead66590d742b7c3b7a30f",
        ),
        (
            "tinyflows-module-0.9.2-macos-15-arm64.tar.gz",
            "c20a68c370ae6c6184917ad136b0bde55c7a91207717960ebe3fc71f33609c0b",
        ),
        (
            "tinyflows-module-0.9.2-macos-15-x86_64.tar.gz",
            "5d33229a1d4ea3b934b9a5199831d5d94c6725c51435da9e18f1493324cde728",
        ),
        (
            "tinyflows-module-0.9.2-ubuntu-24.04-x86_64.tar.gz",
            "4830d646cdd866399204eb53d43ef567dcb5bfd0ea4b0a99ea33b36fdb31cc20",
        ),
        (
            "tinyflows-module-0.9.2-ubuntu-24.04-arm64.tar.gz",
            "6606505283da68b5f403e8a68abdbb1b2cf4a5f8af546cc6d32fedaadb47470b",
        ),
        (
            "tinyflows-module-0.9.2-ubuntu-22.04-x86_64.tar.gz",
            "38dd32039169ac2647a16e76484d3285a36a6bdc21e66eacc3e04aee15972bc8",
        ),
        (
            "tinyflows-module-0.9.2-ubuntu-22.04-arm64.tar.gz",
            "84dabcfd8170081f79232c45a3f6662919d6b55e41018e6e8819fa0584546ac9",
        ),
        (
            "tinyflows-module-0.9.2-windows-2025-x86_64.zip",
            "6747f1016231087770522c9e92b623c093de7f3081fd96a0fa36eee08bdfab2f",
        ),
        (
            "tinyflows-module-0.9.2-windows-2022-x86_64.zip",
            "b9974418d6802f6f9c32646be17a3ee58b6a73e27ae119d89cce039e6e3e2d01",
        ),
        (
            "tinyflows-module-0.9.2-windows-11-arm64.zip",
            "94e5bcc612b6046a1c3c8cfebb5270f28ca5048eeee452d5a0d56a132d7e32f0",
        ),
    ];
    for (asset, (archive, sha256)) in record.assets.iter().zip(expected) {
        assert_eq!(asset.archive, archive);
        assert_eq!(asset.sha256, sha256, "{}", asset.archive);
    }
}
