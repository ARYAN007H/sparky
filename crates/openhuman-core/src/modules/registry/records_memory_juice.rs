//! Registry record for the `tinyjuice` module.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

pub(crate) const TINYJUICE: ModuleRecord = ModuleRecord {
    id: "tinyjuice",
    description: "Content-aware tool-output compression and recoverable caching",
    bus_name: tinyjuice_bus::names::BUS_NAME,
    object_path: tinyjuice_bus::names::OBJECT_PATH,
    version: "0.10.7",
    release_url: "https://github.com/tinyhumansai/tinyjuice/releases/tag/v0.10.7",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinyjuice-module-0.10.7-archlinux-arm64.tar.gz",
            sha256: "9b2b78128b5129aae9aab3a61af76e93ac6d7169d366a039ed7c1c6c1bc22b98",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinyjuice-module-0.10.7-archlinux-x86_64.tar.gz",
            sha256: "fe9576c4f0f03f2adfbc40a315f4a63a71bab36e6024d8571b62778734d7a5c9",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinyjuice-module-0.10.7-fedora-43-arm64.tar.gz",
            sha256: "18466159cbdbe40d94d8ccca9d8173acd99fa4f7fb2ec9fa29075943cf29f3e9",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinyjuice-module-0.10.7-fedora-43-x86_64.tar.gz",
            sha256: "23616f862dc558032c8dcf9194b5ac6a1c644211ce3a1fcd56b40902efe534b9",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinyjuice-module-0.10.7-fedora-44-arm64.tar.gz",
            sha256: "664e1a8c871b529a658f91f80827444efa81f070a3c995ede78e4f9cfd33e7a4",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinyjuice-module-0.10.7-fedora-44-x86_64.tar.gz",
            sha256: "fb09736ae170f8e2e3972f8b29e9862f8c173d30e4a3e31636d38a9d17caf350",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyjuice-module-0.10.7-macos-15-arm64.tar.gz",
            sha256: "ae23a0b2495971c1c50a7b65e4d5479f62a978d18258613d6fa47604ccb92914",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyjuice-module-0.10.7-macos-15-x86_64.tar.gz",
            sha256: "2022eea676aafd0fb4d623f8422d935fff0b3ad22c3f5527099fd8d7755ec51f",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyjuice-module-0.10.7-macos-26-arm64.tar.gz",
            sha256: "3826509443ca138ac73e6d69e404a8c280d192b2d7c8d2e56c429fa33badd1f6",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyjuice-module-0.10.7-macos-26-x86_64.tar.gz",
            sha256: "49569705476e1defb6f3bfb7c9660ee4622a91691c282c6e1ffddc02c901ad62",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyjuice-module-0.10.7-ubuntu-22.04-arm64.tar.gz",
            sha256: "068e8ba7efabba21cb4dde08de4da3ff1569479ccf77b5d5f5bbf9c1ed3a0b0a",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyjuice-module-0.10.7-ubuntu-22.04-x86_64.tar.gz",
            sha256: "51b9d32d4a9ae49e5a7029babe90b5cb3eee8b9aaa7ae7df11ea8d026954e1f3",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyjuice-module-0.10.7-ubuntu-24.04-arm64.tar.gz",
            sha256: "15481b6aec6fb33a093574e56ebcf5af07db1e19ac0ccde82188c7f3f0cc54d4",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyjuice-module-0.10.7-ubuntu-24.04-x86_64.tar.gz",
            sha256: "ecce207435ff15f5e80f4406fe4e01708ada62526332e3d1fd20dafc09bf81f6",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyjuice-module-0.10.7-windows-11-arm64.zip",
            sha256: "6d38a30e3b534321026312cdc34ca3b13062bb7cd374871cd311daf41846773d",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyjuice-module-0.10.7-windows-2022-x86_64.zip",
            sha256: "2a88534b751d4fc83a3b34a3bb92ec9a4688305c3b4ecf07e42b4fdf4baa7cba",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyjuice-module-0.10.7-windows-2025-x86_64.zip",
            sha256: "e210c3ba09572e7afdb039627d0e4f469bff275f2744eccba01939fb6a07a781",
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
