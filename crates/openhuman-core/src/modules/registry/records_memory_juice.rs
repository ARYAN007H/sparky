//! Registry record for the `tinyjuice` module.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

pub(crate) const TINYJUICE: ModuleRecord = ModuleRecord {
    id: "tinyjuice",
    description: "Content-aware tool-output compression and recoverable caching",
    bus_name: tinyjuice_bus::names::BUS_NAME,
    object_path: tinyjuice_bus::names::OBJECT_PATH,
    version: "0.11.1",
    release_url: "https://github.com/tinyhumansai/tinyjuice/releases/tag/v0.11.1",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinyjuice-module-0.11.1-archlinux-arm64.tar.gz",
            sha256: "fc5ee45411b04696ff349baa29ddc6750a5b35c57e383b887c3459943c5a8d7b",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinyjuice-module-0.11.1-archlinux-x86_64.tar.gz",
            sha256: "1dad8e6af274527dfafebde8d7ab7e309c6ce8a9fa14e8f9e4be28c7a3ed107b",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinyjuice-module-0.11.1-fedora-43-arm64.tar.gz",
            sha256: "a8107d1a9bbaeaeea6b2ba56ae75e9abaf79563241198d2e9eb3618f85adfeb4",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinyjuice-module-0.11.1-fedora-43-x86_64.tar.gz",
            sha256: "51c48caf2903e53fb17393f30f3e3aca7e4d7645e2d4e90a84bc9dde795daf9a",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinyjuice-module-0.11.1-fedora-44-arm64.tar.gz",
            sha256: "a410381d9bea50f44b6d811ce3f01a55483bc21a0af0c863b87c584d25b14429",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinyjuice-module-0.11.1-fedora-44-x86_64.tar.gz",
            sha256: "a65fa6b5407b0ed1a2ea9a173f8768db72c9ad14bf0192dcba9d390d00b84d84",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyjuice-module-0.11.1-macos-15-arm64.tar.gz",
            sha256: "03e1c7a8897d3e36148bdbf4b9aeed85ae355529297db3ad7e99ef620743fa88",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyjuice-module-0.11.1-macos-15-x86_64.tar.gz",
            sha256: "e28edfbeb37bc1eebf694c19f7b86d3bc797306eb3a0039292e29d065d10ffc3",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyjuice-module-0.11.1-macos-26-arm64.tar.gz",
            sha256: "7a83d592c82ac9081b5901c703538b7c5e1b606e60cb772a23adbc752cbb9c05",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyjuice-module-0.11.1-macos-26-x86_64.tar.gz",
            sha256: "72b2ada4955ce32dab84b8bbe3f4d6b8302966ef5c649173b8c833d06b0a5d65",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyjuice-module-0.11.1-ubuntu-22.04-arm64.tar.gz",
            sha256: "7e1e63849d0f600efb5d7a4d30dcc68037adbb66bacaba7cb208d90cf0a613ef",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyjuice-module-0.11.1-ubuntu-22.04-x86_64.tar.gz",
            sha256: "b84db9cb10bd4c4ff449255a6a6abfb9b3560473fc59fbd9535279b28ecd4f1d",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyjuice-module-0.11.1-ubuntu-24.04-arm64.tar.gz",
            sha256: "94d45f098cd2cab2d78c9aa32cfaa2cca5a2e0c5d9f8fcf3c489bac625f84e01",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyjuice-module-0.11.1-ubuntu-24.04-x86_64.tar.gz",
            sha256: "e3833a5e818ca3aef8a67b5b16df919cd09473dab62456a18db7a76b082783c4",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyjuice-module-0.11.1-windows-11-arm64.zip",
            sha256: "0cb5182f401031df66c0899495bae778e912de53efea0f599598a69a68801177",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyjuice-module-0.11.1-windows-2022-x86_64.zip",
            sha256: "7de58ca22f51e2c4022182a12ce18ab23911bd20d3bf3494bfa6b006746bac37",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyjuice-module-0.11.1-windows-2025-x86_64.zip",
            sha256: "26a72291db00895468c7e9c5736db66f18fe2bf6fec7ea0ed67ba8568c98434a",
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
