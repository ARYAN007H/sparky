//! TinyFlows module: scheduled job runtime and workflow authoring through its bus contract.
use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

pub(crate) const TINYFLOWS: ModuleRecord = ModuleRecord {
    id: "tinyflows",
    description: "Scheduled jobs, workflow Catalog/Runs execution, and authoring through TinyFlows",
    bus_name: tinyflows_bus::BUS_NAME,
    object_path: tinyflows_bus::OBJECT_PATH,
    version: "0.9.3",
    release_url: "https://github.com/tinyhumansai/tinyflows/releases/tag/v0.9.3",
    // Verbatim from the published v0.9.2 checksum.toml.
    assets: &[
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyflows-module-0.9.3-macos-26-arm64.tar.gz",
            sha256: "60983ebe770cf4295751d7b12e0ec315c64c8554f9a5473fdea30e973d520919",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyflows-module-0.9.3-macos-26-x86_64.tar.gz",
            sha256: "b66ae430b923b0a75c0ff7e218d360d6aee46c96b2aabf93a3518e0d904122dd",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyflows-module-0.9.3-macos-15-arm64.tar.gz",
            sha256: "265e1d63011119ea294d34d5c87eb522f7b73b3e128e6dd84faa4d7083fc629b",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyflows-module-0.9.3-macos-15-x86_64.tar.gz",
            sha256: "2483fd6429382eef00eb25f1cb420aea0e4717d5dff1de425eab45ae7dabd922",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyflows-module-0.9.3-ubuntu-24.04-x86_64.tar.gz",
            sha256: "0677b46abe3404e644e53a76c09ee32c22133fa3a09629d8e40fa9566638b096",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyflows-module-0.9.3-ubuntu-24.04-arm64.tar.gz",
            sha256: "a3e20503edcc7775c2c67624b4ffeca0a1804b44c3582a038a430369011103cc",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyflows-module-0.9.3-ubuntu-22.04-x86_64.tar.gz",
            sha256: "1498a25772b85396403f65766b1302ebfd58eec6b039ed07396c7923fc329cc8",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyflows-module-0.9.3-ubuntu-22.04-arm64.tar.gz",
            sha256: "207be0cbbbb992e8923628630a515e04c618a2e9f0239146b4679460280fa181",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyflows-module-0.9.3-windows-2025-x86_64.zip",
            sha256: "873ab7ced3711f733a02714e81d0c56fd41dedc7801c401a078e76dcc29d16e9",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyflows-module-0.9.3-windows-2022-x86_64.zip",
            sha256: "f940c600d672f7bf09da52875541c518d0e7c66caa478928cee4b0571dafb707",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyflows-module-0.9.3-windows-11-arm64.zip",
            sha256: "bb8dd79c29a2dcaccc32df6b9c02f1be4d5ded7eb9729ca6bc414afa61e01bf1",
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
