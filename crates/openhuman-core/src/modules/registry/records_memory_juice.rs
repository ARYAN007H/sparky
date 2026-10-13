//! Registry record for the `tinyjuice` module.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

pub(crate) const TINYJUICE: ModuleRecord = ModuleRecord {
    id: "tinyjuice",
    description: "Content-aware tool-output compression and recoverable caching",
    bus_name: tinyjuice_bus::names::BUS_NAME,
    object_path: tinyjuice_bus::names::OBJECT_PATH,
    version: "0.10.0",
    release_url: "https://github.com/tinyhumansai/tinyjuice/releases/tag/v0.10.0",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyjuice-module-0.10.0-ubuntu-24.04-x86_64.tar.gz",
            sha256: "a40704bd1086e29b1238639e9f1cab4f14a04bb70ba7f3b88d9c748d3715ff36",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyjuice-module-0.10.0-ubuntu-24.04-arm64.tar.gz",
            sha256: "fdd8f34817309563d6c2c1dfc46dca7c35c2311e9315e36ab95b63a86e33fccf",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyjuice-module-0.10.0-ubuntu-22.04-x86_64.tar.gz",
            sha256: "c1fc3353e881c09f83a63d00d13a5319c9bed1c8bdd33ba29f80019b274b0ce1",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyjuice-module-0.10.0-ubuntu-22.04-arm64.tar.gz",
            sha256: "922caaa47086fa53b7fbc443c0a9d4c80376d3740afcd91036befb8c9f495176",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyjuice-module-0.10.0-macos-26-arm64.tar.gz",
            sha256: "321470839fbb97774d0d11b03b5ff9c23ced58d505c3af9f9840ecb5bfc6648a",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyjuice-module-0.10.0-macos-26-x86_64.tar.gz",
            sha256: "7e12c2bc2f0d2041db9749e9013f14cc1021521010e9cc1c3649c59b57fad205",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyjuice-module-0.10.0-macos-15-arm64.tar.gz",
            sha256: "7e2acc0e0461e5ad7a201b883c312e410596ad15e232733861c33198b92308c1",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyjuice-module-0.10.0-macos-15-x86_64.tar.gz",
            sha256: "a472ca39813308efbbde49957a2e7169a0d6c04a6a4001cda08b3c573ff22664",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyjuice-module-0.10.0-windows-2025-x86_64.zip",
            sha256: "80c057701bd13fa1c3bf5e379af78addb8c4d72ed06152c792c93be6c313f880",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyjuice-module-0.10.0-windows-2022-x86_64.zip",
            sha256: "8db4a74d71b5cd203b783cd869f375b5c6d91b391dff221a379c836d7bac93ae",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyjuice-module-0.10.0-windows-11-arm64.zip",
            sha256: "72fca45f566da829edf11e47e086949f1d14a61b25b46eeae081ceabb5dea188",
        },
    ],
    load: LoadPolicy::Lazy,
    source: SourceSpec {
        submodule: "tinyjuice",
        manifest: "crates/tinyjuice-module/Cargo.toml",
        package: "tinyjuice-module",
        lib_name: "tinyjuice_module",
        features: &[],
    },
};
