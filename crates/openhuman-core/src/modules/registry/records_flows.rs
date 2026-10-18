//! TinyFlows module: scheduled job runtime and workflow authoring through its bus contract.
use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

pub(crate) const TINYFLOWS: ModuleRecord = ModuleRecord {
    id: "tinyflows",
    description: "Scheduled jobs and workflow authoring through TinyFlows",
    bus_name: tinyflows_bus::BUS_NAME,
    object_path: tinyflows_bus::OBJECT_PATH,
    version: "0.8.25",
    release_url: "https://github.com/tinyhumansai/tinyflows/releases/tag/v0.8.25",
    // Verbatim from the published v0.8.25 checksum.toml.
    assets: &[
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyflows-module-0.8.25-macos-26-arm64.tar.gz",
            sha256: "af7e4fd98885704f4745384b32f628128954389595269698bc26fa90be41ea68",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyflows-module-0.8.25-macos-26-x86_64.tar.gz",
            sha256: "d253d9ca4d95b806042b7b5bc4f98acfdb174c0512e3fb7035a46a7e0e07b39d",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyflows-module-0.8.25-macos-15-arm64.tar.gz",
            sha256: "2f6a938f3d12874db93d3b44eca3e7aa44418cd1c6d07a06813d3ff10eddacbe",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyflows-module-0.8.25-macos-15-x86_64.tar.gz",
            sha256: "7dafc35c4754e1198209f0fd182e3ab3e7e1aa2bb6cfdcfad1bae0649049aa7f",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyflows-module-0.8.25-ubuntu-24.04-x86_64.tar.gz",
            sha256: "1a713e76ff38034481a06cfc0dc9cee1c59af80af6338d9d4a4c8901fbab7c8f",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyflows-module-0.8.25-ubuntu-24.04-arm64.tar.gz",
            sha256: "b0f59ee5d2f7726f409a5e3d5cd48eea2f2a2f32fe82a070eb8758ee80ae2646",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyflows-module-0.8.25-ubuntu-22.04-x86_64.tar.gz",
            sha256: "4d8f500c83243894daea71a3051c1d549269cd2079bf114469c41324a7e3733d",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyflows-module-0.8.25-ubuntu-22.04-arm64.tar.gz",
            sha256: "3eadc6e3e71a6176354a695b33897b6eda61ba469f26dc51e27dd3e966cebda0",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyflows-module-0.8.25-windows-2025-x86_64.zip",
            sha256: "9cf9b8bf35b4e7134a57dc5e03d14966c9ff468bfec16fcac8b22285dcbc2e40",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyflows-module-0.8.25-windows-2022-x86_64.zip",
            sha256: "bb5c8d97030aa53a05c3baf60bb1e3a6a8631be6add72a03823052b795fb1286",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyflows-module-0.8.25-windows-11-arm64.zip",
            sha256: "712cf7bc89b3d0a1a02beff3d792990047cd946a23433a3e6f54b35545963ad8",
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
