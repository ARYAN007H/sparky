//! Registry record for the `tinyjuice` module.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

pub(crate) const TINYJUICE: ModuleRecord = ModuleRecord {
    id: "tinyjuice",
    description: "Content-aware tool-output compression and recoverable caching",
    bus_name: tinyjuice_bus::names::BUS_NAME,
    object_path: tinyjuice_bus::names::OBJECT_PATH,
    version: "0.9.1",
    release_url: "https://github.com/tinyhumansai/tinyjuice/releases/tag/v0.9.1",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyjuice-module-0.9.1-ubuntu-24.04-x86_64.tar.gz",
            sha256: "d3e815756b62c438392456d41d90ad6f78de98a52d43d2871fa30b7b11583e34",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyjuice-module-0.9.1-ubuntu-24.04-arm64.tar.gz",
            sha256: "72727b59b32cd0ec7f63a7fc69ecdbd1695f799ca85e8c10a0081d3f7285ba92",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyjuice-module-0.9.1-ubuntu-22.04-x86_64.tar.gz",
            sha256: "d486ed35ea24636aa6a4d107df3cb453c89cb8a7e52dc3379bc008dca2a80c5d",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyjuice-module-0.9.1-ubuntu-22.04-arm64.tar.gz",
            sha256: "49fc9295b3614ac000a663917affb20322415ff6cbb0bb403cfad08fc096a116",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyjuice-module-0.9.1-macos-26-arm64.tar.gz",
            sha256: "790355c49f889584865e0c58e9ba902474960229b5adbf2c5346684738081fd2",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyjuice-module-0.9.1-macos-26-x86_64.tar.gz",
            sha256: "240d0a96159c0d9d4fd9c81d1d5368290560637fbeb6e882b5d57ee8f3a7c44a",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyjuice-module-0.9.1-macos-15-arm64.tar.gz",
            sha256: "8276e2c4d2003b875bca8de8ca77fb9cd47413c3766ed1c15fcee3b780cf81f9",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyjuice-module-0.9.1-macos-15-x86_64.tar.gz",
            sha256: "f87a3bc567e5a2a3f542632a44e25ce73f098cc2f0c08ebacdca8720f2d96073",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyjuice-module-0.9.1-windows-2025-x86_64.zip",
            sha256: "4ae0c3d5ea982b01cefd79da1cc4d7ea4b94d823cdaa6d75b04d55a031cec885",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyjuice-module-0.9.1-windows-2022-x86_64.zip",
            sha256: "e5efcc6f8e2de411202cdf2d4735c42faf05c6a22e050b1ce617520d73baa637",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyjuice-module-0.9.1-windows-11-arm64.zip",
            sha256: "55bb951ae319d30520a122795c29485046113e3ff00d62f9fc846d42ee899293",
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
