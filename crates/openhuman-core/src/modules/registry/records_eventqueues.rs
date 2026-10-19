//! Registry record for the `tinyeventqueues` module.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

/// The `tinyeventqueues` module: durable inbound event queues. It persists,
/// deduplicates, filters and paces inbound events (Composio triggers, agent
/// webhooks) on the host's storage, loaded on first ingest, and calls the
/// host back (`Deliver`, `ClassifySalience`) to run triage.
pub(crate) const TINYEVENTQUEUES: ModuleRecord = ModuleRecord {
    id: "tinyeventqueues",
    description: "Durable inbound event queues: dedupe, filtering, pacing and retry before triage",
    bus_name: tinyeventqueues_bus::INTERFACE,
    object_path: tinyeventqueues_bus::OBJECT_PATH,
    version: "0.1.1",
    release_url: "https://github.com/tinyhumansai/tinyeventqueues/releases/tag/v0.1.1",
    assets: &[
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyeventqueues-0.1.1-macos-15-arm64.tar.gz",
            sha256: "0000000000000000000000000000000000000000000000000000000000000000",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyeventqueues-0.1.1-macos-15-x86_64.tar.gz",
            sha256: "0000000000000000000000000000000000000000000000000000000000000000",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyeventqueues-0.1.1-macos-26-arm64.tar.gz",
            sha256: "0000000000000000000000000000000000000000000000000000000000000000",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyeventqueues-0.1.1-macos-26-x86_64.tar.gz",
            sha256: "0000000000000000000000000000000000000000000000000000000000000000",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyeventqueues-0.1.1-ubuntu-22.04-arm64.tar.gz",
            sha256: "0000000000000000000000000000000000000000000000000000000000000000",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyeventqueues-0.1.1-ubuntu-22.04-x86_64.tar.gz",
            sha256: "0000000000000000000000000000000000000000000000000000000000000000",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyeventqueues-0.1.1-ubuntu-24.04-arm64.tar.gz",
            sha256: "0000000000000000000000000000000000000000000000000000000000000000",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyeventqueues-0.1.1-ubuntu-24.04-x86_64.tar.gz",
            sha256: "0000000000000000000000000000000000000000000000000000000000000000",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyeventqueues-0.1.1-windows-11-arm64.zip",
            sha256: "0000000000000000000000000000000000000000000000000000000000000000",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyeventqueues-0.1.1-windows-2022-x86_64.zip",
            sha256: "0000000000000000000000000000000000000000000000000000000000000000",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyeventqueues-0.1.1-windows-2025-x86_64.zip",
            sha256: "0000000000000000000000000000000000000000000000000000000000000000",
        },
    ],
    load: LoadPolicy::Lazy,
    source: SourceSpec {
        submodule: "tinyeventqueues",
        manifest: "crates/tinyeventqueues/Cargo.toml",
        package: "tinyeventqueues",
        lib_name: "tinyeventqueues",
        features: &[],
    },
};
