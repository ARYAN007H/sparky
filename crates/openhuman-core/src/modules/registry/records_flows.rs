//! TinyFlows module: scheduled job runtime and workflow authoring through its bus contract.
use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

pub(crate) const TINYFLOWS: ModuleRecord = ModuleRecord {
    id: "tinyflows",
    description: "Scheduled jobs and workflow authoring through TinyFlows",
    bus_name: tinyflows_bus::BUS_NAME,
    object_path: tinyflows_bus::OBJECT_PATH,
    version: "0.8.27",
    release_url: "https://github.com/tinyhumansai/tinyflows/releases/tag/v0.8.27",
    // Verbatim from the published v0.8.26 checksum.toml.
    assets: &[
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyflows-module-0.8.27-macos-26-arm64.tar.gz",
            sha256: "99e05d3e7612f0f65315533e74bf3b08ff804102ff8439d51e9b19f522748063",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyflows-module-0.8.27-macos-26-x86_64.tar.gz",
            sha256: "0614266a74bd830f720c319e8cb8cc6442b86e992b3fea7af0f5b63c2ed02164",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyflows-module-0.8.27-macos-15-arm64.tar.gz",
            sha256: "60ba1f5ef4f6198f886e980eabe3400cc70ad2f765e797b1f16cb22d4998925a",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyflows-module-0.8.27-macos-15-x86_64.tar.gz",
            sha256: "9304f1b065c9def4a1e8f28c05a70697c5da49e48fdecd0594544bc831658196",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyflows-module-0.8.27-ubuntu-24.04-x86_64.tar.gz",
            sha256: "0b4345b8747c7f855039978873ad01f1f8b8c7a0356bfc73cc159cb269622843",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyflows-module-0.8.27-ubuntu-24.04-arm64.tar.gz",
            sha256: "af8bd91845c0d13b15d2c26ee0059d312bf4ec383d2bd50041bbf7e41ecaf834",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyflows-module-0.8.27-ubuntu-22.04-x86_64.tar.gz",
            sha256: "6d7b89d9d52f5b6024e421fd17d9a4391448e29708835c3380a022e9beccdafe",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyflows-module-0.8.27-ubuntu-22.04-arm64.tar.gz",
            sha256: "74fb1d0b12d87ae7830ade2f6e043707ee356a54346b560adb1fc5d66785de2f",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyflows-module-0.8.27-windows-2025-x86_64.zip",
            sha256: "b116e7eebaff050333d1412940d13a16cea711261dff8f7503589fef087d36b8",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyflows-module-0.8.27-windows-2022-x86_64.zip",
            sha256: "9b12b158dd57a5efc6b59e3871301e650e120d6969775318ed7a8d98b8ef94ed",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyflows-module-0.8.27-windows-11-arm64.zip",
            sha256: "7ea7cf80ee0f65f56485ce1e7066ec7591098ecbae259baff85a7ab53f2d3305",
        },
    ],
    load: LoadPolicy::Lazy,
    source: SourceSpec {
        submodule: "tinyflows",
        manifest: "crates/tinyflows-module/Cargo.toml",
        package: "tinyflows-module",
        lib_name: "tinyflows_module",
        features: &[],
    },
};
