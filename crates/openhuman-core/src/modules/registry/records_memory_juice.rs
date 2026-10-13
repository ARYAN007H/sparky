//! Registry record for the `tinyjuice` module.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

pub(crate) const TINYJUICE: ModuleRecord = ModuleRecord {
    id: "tinyjuice",
    description: "Content-aware tool-output compression and recoverable caching",
    bus_name: tinyjuice_bus::names::BUS_NAME,
    object_path: tinyjuice_bus::names::OBJECT_PATH,
    version: "0.9.2",
    release_url: "https://github.com/tinyhumansai/tinyjuice/releases/tag/v0.9.2",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyjuice-module-0.9.2-ubuntu-24.04-x86_64.tar.gz",
            sha256: "cbf499d69a6ec0421abf85b5d6a6127b36c494a9f6b9b827c8dc2b909b803650",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyjuice-module-0.9.2-ubuntu-24.04-arm64.tar.gz",
            sha256: "b164106ec92d3fcb31f364ab5bae514f70c40795cb44ba66f74500c4477cd464",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyjuice-module-0.9.2-ubuntu-22.04-x86_64.tar.gz",
            sha256: "c7943caf014f463285370355dc51c04a923a1d86d75294d9ba4564faec193f82",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyjuice-module-0.9.2-ubuntu-22.04-arm64.tar.gz",
            sha256: "1784cf0e38dd0e0b710fd5434dd23d4280f6492b8f39e58a3dd6ca0a9d2b7913",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyjuice-module-0.9.2-macos-26-arm64.tar.gz",
            sha256: "c7509e1c13a86248d43e2616f3394fd08146a3707554e979c025057bf7d7561a",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyjuice-module-0.9.2-macos-26-x86_64.tar.gz",
            sha256: "03037e91677e13dd166078e382053dff111a5579097bde75a867564312833197",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyjuice-module-0.9.2-macos-15-arm64.tar.gz",
            sha256: "243581b01a847a921ec6289e9efe3958c7c4e082cf15e271462e8d75d3f93756",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyjuice-module-0.9.2-macos-15-x86_64.tar.gz",
            sha256: "ec4b8a870e32b5a9249cf1c0fcbad3a72ed718063bb425bcc48bd47a12908978",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyjuice-module-0.9.2-windows-2025-x86_64.zip",
            sha256: "ba82fee144b7997109266a28fa4c35513db1e97c5cbdf9e9740823ae55370d64",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyjuice-module-0.9.2-windows-2022-x86_64.zip",
            sha256: "eb58b8418f9fcf6f1cf1b1e84a9eca01a7bcb358b56b3f9347aa644be02e2eff",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyjuice-module-0.9.2-windows-11-arm64.zip",
            sha256: "1b4be0bd07d8a88859cb3da582bf564f0debf5a1c0bb27acfafcba08947bb894",
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
