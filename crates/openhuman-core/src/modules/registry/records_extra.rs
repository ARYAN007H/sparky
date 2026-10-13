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
    version: "0.1.30",
    release_url: "https://github.com/tinyhumansai/tinychannels/releases/tag/v0.1.30",
    assets: &[
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinychannels-module-0.1.30-macos-15-arm64.tar.gz",
            sha256: "c23e3c099f33121e14a520dfec994e7b5da6205da02ea001d71b28889e5f269b",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinychannels-module-0.1.30-macos-15-x86_64.tar.gz",
            sha256: "455e33ffb1d93b0a1023e8b4d528b49ccbc0ed95851b1fb2733649994efe9bfc",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinychannels-module-0.1.30-macos-26-arm64.tar.gz",
            sha256: "b2bcc666b980eab9a3090312ed83482a34591f1f4ecc73346f8e1e2422f482eb",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinychannels-module-0.1.30-macos-26-x86_64.tar.gz",
            sha256: "d92cb8757f49ebaf3b5d7ecbd79af193a4d4bb002e9b78ce473091d649b08b31",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinychannels-module-0.1.30-ubuntu-22.04-arm64.tar.gz",
            sha256: "5852f6888541332cacaed2401ed67815588ba076d60926fdb31daedb13185b12",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinychannels-module-0.1.30-ubuntu-22.04-x86_64.tar.gz",
            sha256: "f63bb9034aa48a2335bea0a86ad0f1f920121281b88611c11a9e09fdac8cfe02",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinychannels-module-0.1.30-ubuntu-24.04-arm64.tar.gz",
            sha256: "810f68f723e9817d7343944137a00689ef997f57e4e241f496120350a1ba58f6",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinychannels-module-0.1.30-ubuntu-24.04-x86_64.tar.gz",
            sha256: "61e7a6f956b17ae53e83a2b0068f1009b0fde0b1266ca4f8f1a7feb8fb077104",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinychannels-module-0.1.30-windows-11-arm64.zip",
            sha256: "122b253a859f769a6ae5e89ea9adc5edaefc5471bf26e6bf8d09e7e0bef3d236",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinychannels-module-0.1.30-windows-2022-x86_64.zip",
            sha256: "2a8905d6a72bdbc7c4839fa7afb38362deef54eb2296f918b6af016dd8d54090",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinychannels-module-0.1.30-windows-2025-x86_64.zip",
            sha256: "feeddcb7beaa48e4bfda78e40cf6ce999cb7e4abd0414a6595da44665546af88",
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
