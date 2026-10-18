//! TinyFlows module: scheduled job runtime and workflow authoring through its bus contract.
use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

pub(crate) const TINYFLOWS: ModuleRecord = ModuleRecord {
    id: "tinyflows",
    description: "Scheduled jobs and workflow authoring through TinyFlows",
    bus_name: tinyflows_bus::BUS_NAME,
    object_path: tinyflows_bus::OBJECT_PATH,
    version: "0.8.26",
    release_url: "https://github.com/tinyhumansai/tinyflows/releases/tag/v0.8.26",
    // Verbatim from the published v0.8.26 checksum.toml.
    assets: &[
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyflows-module-0.8.26-macos-26-arm64.tar.gz",
            sha256: "7580d7ca414b1a1cb5d950cb02ca274bad3028bfcd6193a0bf1101f36e8e3f0c",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyflows-module-0.8.26-macos-26-x86_64.tar.gz",
            sha256: "92ed4344f3838f35a030caf88407886127c0293529d06640b124c3e708f8998a",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyflows-module-0.8.26-macos-15-arm64.tar.gz",
            sha256: "2aae71e0d241d0cf39e219b96b10ddacc24b3c5c9e88d32acaf8dccc221c3da3",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyflows-module-0.8.26-macos-15-x86_64.tar.gz",
            sha256: "3c5fcc9302ee1a11334eb9aead916a607fc7a01d9aa067545c0c41bf118e58db",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyflows-module-0.8.26-ubuntu-24.04-x86_64.tar.gz",
            sha256: "4f5030fba094e81e909ad342b6a31abf55ae0615c0b894c4c71f1d712fa2981c",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyflows-module-0.8.26-ubuntu-24.04-arm64.tar.gz",
            sha256: "442963394c17e5c43e02b3e57558b2549c752048eef36a616b03119b84cc556e",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyflows-module-0.8.26-ubuntu-22.04-x86_64.tar.gz",
            sha256: "e86eda1b9d37d9733f265821ff0d7e89e276991ce7cae2838d6c7a273d15631f",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyflows-module-0.8.26-ubuntu-22.04-arm64.tar.gz",
            sha256: "4b0deb89170ccbf22ae89d994520559c4243c24822d376d6bf92b72c01efc9d9",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyflows-module-0.8.26-windows-2025-x86_64.zip",
            sha256: "f13088909316c00873ff87b9ee2970d35c231c946f27f3506ecdf6f7bc0b20c0",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyflows-module-0.8.26-windows-2022-x86_64.zip",
            sha256: "f71ba5038f4658ab4b29967eb37020376e5e5a1fc32323036a8bdd079e140b9a",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyflows-module-0.8.26-windows-11-arm64.zip",
            sha256: "96c860ecce2d82537aedb2efd97c8c4fc9b023c0b370e06be8a0efe50cbef078",
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
