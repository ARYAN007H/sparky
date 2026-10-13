//! Registry records for additional first-party TinyBus modules.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

/// The `tinybox` module, loaded on demand.
#[cfg(feature = "modules")]
pub(crate) const TINYBOX: ModuleRecord = ModuleRecord {
    id: "tinybox",
    description: "Sandbox execution and lifecycle operations through TinyBox",
    bus_name: tinybox_bus::INTERFACE,
    object_path: tinybox_bus::OBJECT_PATH,
    version: "0.1.24",
    release_url: "https://github.com/tinyhumansai/tinybox/releases/tag/v0.1.24",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinybox-0.1.24-archlinux-arm64.tar.gz",
            sha256: "11b4c959868e9e13f60af116946145530ca76ad68ab2ceef4fea11a0c2f76c5f",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinybox-0.1.24-archlinux-x86_64.tar.gz",
            sha256: "84764f24331dfb5ff1798419e69ada288233506f6d28f4f582f5e6a87f65d9e9",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinybox-0.1.24-fedora-43-arm64.tar.gz",
            sha256: "26763cb6a77e638adfb8641b58aaa0f57493403c7ab4dbca639f35e434fbd20c",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinybox-0.1.24-fedora-43-x86_64.tar.gz",
            sha256: "442d67e92c011125350e672a505925008b0bb26c368d3408c24029e60bc12be9",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinybox-0.1.24-fedora-44-arm64.tar.gz",
            sha256: "67c0bf40748a1bf9e749bc918cdaddc5ba53f29f9cc3327a7c56fc82ef8dfdee",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinybox-0.1.24-fedora-44-x86_64.tar.gz",
            sha256: "5e320dcce68a3b36c61a33647575c9e3f1449388f489e6aa094b7684863ab377",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinybox-0.1.24-macos-15-arm64.tar.gz",
            sha256: "59a76b8ea5ce093d26616b115cd11493459dcefebec73c2b63fa115e4c70d6f7",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinybox-0.1.24-macos-15-x86_64.tar.gz",
            sha256: "dd73c660e3c54004dad55257eeefc812fa8c2b3bf89d82e346b5bb4e95439b7f",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinybox-0.1.24-macos-26-arm64.tar.gz",
            sha256: "3c2091b15246cb6fb566c73cbfc2f1a334b9f1d1a4ea58839f3576536321b285",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinybox-0.1.24-macos-26-x86_64.tar.gz",
            sha256: "a077fae96405163fe4cc2ae0dcdfb2dee9e2b8da8ea7f7122d180d0c2f3789eb",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinybox-0.1.24-ubuntu-22.04-arm64.tar.gz",
            sha256: "beb26115d378dce167ffe6970f57dfd268c9851f10e254306812be3f478f4567",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinybox-0.1.24-ubuntu-22.04-x86_64.tar.gz",
            sha256: "be69717def1d836a8e64e3ed544fdb4fb772ef641df68b473e9677f030f94809",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinybox-0.1.24-ubuntu-24.04-arm64.tar.gz",
            sha256: "4935e8099f1a373b7d3fffc6f5a4019d3e40a4c5c3d360e30c1d311ab6eba777",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinybox-0.1.24-ubuntu-24.04-x86_64.tar.gz",
            sha256: "33391c0e42a5dc48328bb3cb2f785c44f4fc056ec5fc125a655c1416669b233e",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinybox-0.1.24-windows-11-arm64.zip",
            sha256: "9b00a82bacbba00f528aaf6655bc005fa96751f774cb41cf960cb5c1ac0face9",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinybox-0.1.24-windows-2022-x86_64.zip",
            sha256: "a5b676f18bc1252ee81d614c35fd2b1ffb2b2ecc3a0335face170d2185432255",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinybox-0.1.24-windows-2025-x86_64.zip",
            sha256: "c09e9d417442be2748cea8f6a1fe17b323675400f3e8b82dc647ec5a84eb3b99",
        },
    ],
    load: LoadPolicy::Lazy,
    source: SourceSpec {
        submodule: "tinybox",
        manifest: "crates/tinybox-module/Cargo.toml",
        package: "tinybox-module",
        lib_name: "tinybox",
        features: &[],
    },
};

/// The `tinychannels` module, loaded on demand.
pub(crate) const TINYCHANNELS: ModuleRecord = ModuleRecord {
    id: "tinychannels",
    description: "Channel provider lifecycle and message transport",
    bus_name: tinychannels_bus::names::BUS_NAME,
    object_path: tinychannels_bus::names::OBJECT_PATH,
    version: "0.1.29",
    release_url: "https://github.com/tinyhumansai/tinychannels/releases/tag/v0.1.29",
    assets: &[
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinychannels-module-0.1.29-macos-15-arm64.tar.gz",
            sha256: "a1dbca6cc5bb04baec1f5b82d43593d87eb135619f95be3d64441a96cc95127e",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinychannels-module-0.1.29-macos-15-x86_64.tar.gz",
            sha256: "464ceaca6acb3f6653c5e77c6e7d6d357a0cfbcc5e9691f55dc7eb4e1368a7a6",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinychannels-module-0.1.29-macos-26-arm64.tar.gz",
            sha256: "2943b3e8d6b2006a93ad1f880cfc7cd4fbcfb48540c0913bb6318470249edf9c",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinychannels-module-0.1.29-macos-26-x86_64.tar.gz",
            sha256: "50e92f04b2ef30db6f5053742819a2fbfdfb6d6361f63124fac9f32079775970",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinychannels-module-0.1.29-ubuntu-22.04-arm64.tar.gz",
            sha256: "ae621d0ff6cc908135073a2d990ac7d3bc9fd5cc9da99ddcd52e2c610359f7b0",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinychannels-module-0.1.29-ubuntu-22.04-x86_64.tar.gz",
            sha256: "e9b5d5d77f551fc17c715bcc23ce9d18d337570ccc5500438bdeff3d5d507dd6",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinychannels-module-0.1.29-ubuntu-24.04-arm64.tar.gz",
            sha256: "699dfb6ae0ab31deb1334fc287b58fc75e31606ce8f28c07a9dcec1c089c6d9c",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinychannels-module-0.1.29-ubuntu-24.04-x86_64.tar.gz",
            sha256: "3b90fb51324a9068ad7ab5deb8162584d1bebac75ef4abfa05984b6d73fe2db0",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinychannels-module-0.1.29-windows-11-arm64.zip",
            sha256: "efd1b177c73832510434e97f3c3e7e4ee40f36e0d24f6938a5b3e0dfc9297f0c",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinychannels-module-0.1.29-windows-2022-x86_64.zip",
            sha256: "c8d8dac1e6529d552920ae2d77520971a9cd1241e164b0b4c588384ebaf297da",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinychannels-module-0.1.29-windows-2025-x86_64.zip",
            sha256: "c347c824605aae4411b3b5e1385c689043da288445ce75fc58839f7d354aafb6",
        },
    ],
    load: LoadPolicy::Lazy,
    source: SourceSpec {
        submodule: "tinychannels",
        manifest: "crates/tinychannels-module/Cargo.toml",
        package: "tinychannels-module",
        lib_name: "tinychannels_module",
        features: &[],
    },
};

#[cfg(feature = "hosting")]
/// The `tinyhosts` module, loaded on demand.
pub(crate) const TINYHOSTS: ModuleRecord = ModuleRecord {
    id: "tinyhosts",
    description: "Hosting provider operations",
    bus_name: tinyhosts_bus::BUS_NAME,
    object_path: tinyhosts_bus::OBJECT_PATH,
    version: "0.4.0",
    release_url: "https://github.com/tinyhumansai/tinyhosts/releases/tag/v0.4.0",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyhosts-0.4.0-ubuntu-24.04-x86_64.tar.gz",
            sha256: "ffaf1b749808185b01ed36a90d013c8492d03a113cc63eb02657191983c99b1d",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyhosts-0.4.0-ubuntu-24.04-arm64.tar.gz",
            sha256: "10d74c3d50cfe7b099d7ed3cc6017c017f4d807da7622b96482ef0e26371554f",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyhosts-0.4.0-ubuntu-22.04-x86_64.tar.gz",
            sha256: "cdd9a2b222ec368a20db45c55c3deeb40a4dc8ffc47bea73d16f6edfce864cd7",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyhosts-0.4.0-ubuntu-22.04-arm64.tar.gz",
            sha256: "4b50f878cd7c65812a05384fd824e59c0be9c02e566b472725b04ef7eacdec19",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyhosts-0.4.0-macos-26-arm64.tar.gz",
            sha256: "8452dbbf4de45f313e76e75191923ccdb9d294d2a31dced56b14ee284bbf2176",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyhosts-0.4.0-macos-26-x86_64.tar.gz",
            sha256: "86882b94ee6a3bdb5796523e000136a3780ca59db66a7c645f8a3ea5c2c930b4",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyhosts-0.4.0-macos-15-arm64.tar.gz",
            sha256: "cbb66815cf8513985afde49b3cb9f8406d2bf538609956467da3a7bdacd21044",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyhosts-0.4.0-macos-15-x86_64.tar.gz",
            sha256: "7178b5d5bdb851f4485f8981d99277d160262131c0444996e7a0828209034bda",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyhosts-0.4.0-windows-2025-x86_64.zip",
            sha256: "6a29fc50de7ed2224d6faa26440c87bb0293d7ed0f703bd5cbb169bcb5f20234",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyhosts-0.4.0-windows-2022-x86_64.zip",
            sha256: "730aa387d3f18cdc55f9a3fce83f835a3b8da9a26c0f4701c31c76de790081de",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyhosts-0.4.0-windows-11-arm64.zip",
            sha256: "ab281a73d91117fb12adfc62dfe9e7fbdafa36b5fdfa716c5e440ce33d26a2c5",
        },
    ],
    load: LoadPolicy::Lazy,
    source: SourceSpec {
        submodule: "tinyhosts",
        manifest: "Cargo.toml",
        package: "tinyhosts",
        lib_name: "tinyhosts",
        features: &[],
    },
};
