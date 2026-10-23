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
    version: "0.2.0",
    release_url: "https://github.com/tinyhumansai/tinyeventqueues/releases/tag/v0.2.0",
    assets: &[
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyeventqueues-0.2.0-macos-15-arm64.tar.gz",
            sha256: "8ae7cd5118defd8b3c06b69e19e2614ac29a2a7aa6995a6caf9a751a49f3ad5d",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyeventqueues-0.2.0-macos-15-x86_64.tar.gz",
            sha256: "7c8f64b2007d58d404df67f732ce7476956a6ba4e0b7f8c68da0dbb1547d51b0",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyeventqueues-0.2.0-macos-26-arm64.tar.gz",
            sha256: "241723a0a77e25b28489d5ffac8624880c25304a781381de2f3fe0e7806002e2",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyeventqueues-0.2.0-macos-26-x86_64.tar.gz",
            sha256: "491a9dd000a7c3cb1e1e9d3ab0717683d044f20f3f11716b7a7aa6be405356cb",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyeventqueues-0.2.0-ubuntu-22.04-arm64.tar.gz",
            sha256: "f22b592d870b98db909750987c6fb9ee3a3ae22394d04584dd40bd8205affe4e",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyeventqueues-0.2.0-ubuntu-22.04-x86_64.tar.gz",
            sha256: "6335812a8fa40183f237cf298544d0cd08e56a3ed8f6eb5b99464599e6e82113",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyeventqueues-0.2.0-ubuntu-24.04-arm64.tar.gz",
            sha256: "be74637d3c0bb1a16523e6f638a7182bb86d8079149cfd7c095569a8705ba3dc",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyeventqueues-0.2.0-ubuntu-24.04-x86_64.tar.gz",
            sha256: "ceea76fc2a5871a4a8e3a80f32bdea476c5672f34568ad085ea1a39ac4264d14",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyeventqueues-0.2.0-windows-11-arm64.zip",
            sha256: "3c0b601712a25e9990192886ee2f28672fb5a4653445527be44b500d21dcc5c5",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyeventqueues-0.2.0-windows-2022-x86_64.zip",
            sha256: "d08b91c04382ddbab5d95773b51f28fc57f50f6f66f848d17eeb61d68c509f34",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyeventqueues-0.2.0-windows-2025-x86_64.zip",
            sha256: "e76e7b5b39a2c98e014f094a42c94a35fd548cf0f5e2db715233fee1910c8109",
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
