//! Registry records for additional first-party TinyBus modules.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

/// The `tinybox` module, loaded on demand.
#[cfg(feature = "modules")]
pub(crate) const TINYBOX: ModuleRecord = ModuleRecord {
    id: "tinybox",
    description: "Sandbox execution and lifecycle operations through TinyBox",
    bus_name: tinybox_bus::INTERFACE,
    object_path: tinybox_bus::OBJECT_PATH,
    version: "0.1.26",
    release_url: "https://github.com/tinyhumansai/tinybox/releases/tag/v0.1.26",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinybox-0.1.26-archlinux-arm64.tar.gz",
            sha256: "7f6984d64be252ae4367d188fe08884154301c7a57b93b830189b345aa3f7ac3",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinybox-0.1.26-archlinux-x86_64.tar.gz",
            sha256: "35cbc62fb2e24cab231f705bc689954caf11c73352a3eafc51ba810f507687e3",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinybox-0.1.26-fedora-43-arm64.tar.gz",
            sha256: "14112e7ed9d6dd15543ec68e1bfaa212ca0ea2e62a1dd423d409b4ba3087059b",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinybox-0.1.26-fedora-43-x86_64.tar.gz",
            sha256: "07661b6c59c4f6e40c36a25efe1d99fd1a465291ba4b13d58160b3baafc0762f",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinybox-0.1.26-fedora-44-arm64.tar.gz",
            sha256: "1d1dc263ae85337203b584b4d66b6736ea5bbbe50dfcf0ebd9f83d770e4eceac",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinybox-0.1.26-fedora-44-x86_64.tar.gz",
            sha256: "080d50a2527169fbfc4b1cae61dc55468320b1d5d3819db20f05dca29edf3704",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinybox-0.1.26-macos-15-arm64.tar.gz",
            sha256: "806ef4507a1317c295f08356827d53212a627e329c47f0b41db2fbadd1b10f3e",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinybox-0.1.26-macos-15-x86_64.tar.gz",
            sha256: "3a80345165eda9a2ddd7a09e5447c8042eacee2109a2a7f2e7311efd24a45415",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinybox-0.1.26-macos-26-arm64.tar.gz",
            sha256: "4c63c88eab009946a05e505c146a879ee8446e94103956721af8fa8ecff30c11",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinybox-0.1.26-macos-26-x86_64.tar.gz",
            sha256: "38ec8ee7bf72783e8c3dbe452664312e1b9f55123a5585132fc1e1e1f19a7f90",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinybox-0.1.26-ubuntu-22.04-arm64.tar.gz",
            sha256: "13f9bcdade1873c8cbc82c95e150f06b82182071649906e798bb8d8cd001b9b1",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinybox-0.1.26-ubuntu-22.04-x86_64.tar.gz",
            sha256: "ec051ccaf104cd547c1bd4b6248e8ae09d73ea9f908e8d9a7b696d3238aedddb",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinybox-0.1.26-ubuntu-24.04-arm64.tar.gz",
            sha256: "4057741f50158040f0d2c7c030c15ae81dbb625d03c0a083c7571e4d0a698d52",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinybox-0.1.26-ubuntu-24.04-x86_64.tar.gz",
            sha256: "ec379c022516f6ec1af6288302aed9344a73b64f95b7be45b7e3e64d974552ec",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinybox-0.1.26-windows-11-arm64.zip",
            sha256: "342063b0050d3cfd2f6f957e8f0c407d74d6cda0faebcd145832e7ae3221c820",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinybox-0.1.26-windows-2022-x86_64.zip",
            sha256: "74b480d762323e8aa63227d7e4f50f7d05783e440838adbd288d4663a420567e",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinybox-0.1.26-windows-2025-x86_64.zip",
            sha256: "f25e8c639c2997fc752bd40840418076a0e86687bd53e746170c05cad5f60abd",
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
