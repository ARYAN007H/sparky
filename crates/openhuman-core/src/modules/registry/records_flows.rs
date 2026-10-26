//! TinyFlows module: scheduled job runtime and workflow authoring through its bus contract.
use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

pub(crate) const TINYFLOWS: ModuleRecord = ModuleRecord {
    id: "tinyflows",
    description: "Scheduled jobs, workflow Catalog/Runs execution, and authoring through TinyFlows",
    bus_name: tinyflows_bus::BUS_NAME,
    object_path: tinyflows_bus::OBJECT_PATH,
    version: "0.9.2",
    release_url: "https://github.com/tinyhumansai/tinyflows/releases/tag/v0.9.2",
    // Verbatim from the published v0.9.0 checksum.toml.
    assets: &[
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyflows-module-0.9.2-macos-26-arm64.tar.gz",
            sha256: "283d960f158eb237797d6a8159309e3f2c3635e9b5c93f43f265b4f5add15611",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyflows-module-0.9.2-macos-26-x86_64.tar.gz",
            sha256: "eee906fa2c97ddd7fd16b8ccef6cd0fe95f14a3352ead66590d742b7c3b7a30f",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyflows-module-0.9.2-macos-15-arm64.tar.gz",
            sha256: "c20a68c370ae6c6184917ad136b0bde55c7a91207717960ebe3fc71f33609c0b",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyflows-module-0.9.2-macos-15-x86_64.tar.gz",
            sha256: "5d33229a1d4ea3b934b9a5199831d5d94c6725c51435da9e18f1493324cde728",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyflows-module-0.9.2-ubuntu-24.04-x86_64.tar.gz",
            sha256: "4830d646cdd866399204eb53d43ef567dcb5bfd0ea4b0a99ea33b36fdb31cc20",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyflows-module-0.9.2-ubuntu-24.04-arm64.tar.gz",
            sha256: "6606505283da68b5f403e8a68abdbb1b2cf4a5f8af546cc6d32fedaadb47470b",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyflows-module-0.9.2-ubuntu-22.04-x86_64.tar.gz",
            sha256: "38dd32039169ac2647a16e76484d3285a36a6bdc21e66eacc3e04aee15972bc8",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyflows-module-0.9.2-ubuntu-22.04-arm64.tar.gz",
            sha256: "84dabcfd8170081f79232c45a3f6662919d6b55e41018e6e8819fa0584546ac9",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyflows-module-0.9.2-windows-2025-x86_64.zip",
            sha256: "6747f1016231087770522c9e92b623c093de7f3081fd96a0fa36eee08bdfab2f",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyflows-module-0.9.2-windows-2022-x86_64.zip",
            sha256: "b9974418d6802f6f9c32646be17a3ee58b6a73e27ae119d89cce039e6e3e2d01",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyflows-module-0.9.2-windows-11-arm64.zip",
            sha256: "94e5bcc612b6046a1c3c8cfebb5270f28ca5048eeee452d5a0d56a132d7e32f0",
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
