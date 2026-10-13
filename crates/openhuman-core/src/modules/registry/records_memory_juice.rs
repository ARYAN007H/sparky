//! Registry record for the `tinyjuice` module.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

pub(crate) const TINYJUICE: ModuleRecord = ModuleRecord {
    id: "tinyjuice",
    description: "Content-aware tool-output compression and recoverable caching",
    bus_name: tinyjuice_bus::names::BUS_NAME,
    object_path: tinyjuice_bus::names::OBJECT_PATH,
    version: "0.9.0",
    release_url: "https://github.com/tinyhumansai/tinyjuice/releases/tag/v0.9.0",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyjuice-module-0.9.0-ubuntu-24.04-x86_64.tar.gz",
            sha256: "8be24e6da833e4ecbcac0f85bd178bd7850a7c01d584e687cf143df4591a1d68",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyjuice-module-0.9.0-ubuntu-24.04-arm64.tar.gz",
            sha256: "35069cadb2588b07d288c28141dc66dadea6973243e0473ef6c3048a6772f069",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyjuice-module-0.9.0-ubuntu-22.04-x86_64.tar.gz",
            sha256: "6fa85f60392c0fa369878e298ad14f84e8664f70aac01cd2775c7c2be7e7e6a1",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyjuice-module-0.9.0-ubuntu-22.04-arm64.tar.gz",
            sha256: "8f7a0bbdd86d8d7e30641acdb3afc5b7eb579c94cbefc1fdeee0eee0e987a58f",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyjuice-module-0.9.0-macos-26-arm64.tar.gz",
            sha256: "ebe1b531d09792e40c4deb9a962c524256b5ded2838101bc94b6b72ff6893373",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyjuice-module-0.9.0-macos-26-x86_64.tar.gz",
            sha256: "a67036a80b1ac4444ee81e9836e24964ed5703f6acfdcb6d37deb2c2e15f9900",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyjuice-module-0.9.0-macos-15-arm64.tar.gz",
            sha256: "11ca561822f01dda1cf61faede20d030f493693ff5e49021d11104cad4a8c34a",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyjuice-module-0.9.0-macos-15-x86_64.tar.gz",
            sha256: "94c5fa026ed56d5159b7c1f2c396e237e44787953f79afcc0f27eaf836e415e4",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyjuice-module-0.9.0-windows-2025-x86_64.zip",
            sha256: "e6cf1984853e65da02f2615f3109f9ba94aef916f4013e3e34fae850373c37d5",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyjuice-module-0.9.0-windows-2022-x86_64.zip",
            sha256: "ea2c13ecb346c18be6a76e4f85cf9c8524917c6ffdc862c9d1d0be1a37bac890",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyjuice-module-0.9.0-windows-11-arm64.zip",
            sha256: "f0561ab1af46737a59e976aad44702163455f8a6efbbb80b005a42653bb22571",
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
