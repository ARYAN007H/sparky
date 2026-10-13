//! Registry records for additional first-party TinyBus modules.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

/// The `tinybox` module, loaded on demand.
#[cfg(feature = "modules")]
pub(crate) const TINYBOX: ModuleRecord = ModuleRecord {
    id: "tinybox",
    description: "Sandbox execution and lifecycle operations through TinyBox",
    bus_name: tinybox_bus::INTERFACE,
    object_path: tinybox_bus::OBJECT_PATH,
    version: "0.1.28",
    release_url: "https://github.com/tinyhumansai/tinybox/releases/tag/v0.1.28",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinybox-0.1.28-archlinux-arm64.tar.gz",
            sha256: "7ebf8b928f9c9cca68175a6a1de8abd980838b61958aa944076d458fe83956d5",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinybox-0.1.28-archlinux-x86_64.tar.gz",
            sha256: "8a3ec4cd1a5b3b1e349193f357af0f192e8d73edeaba97099eec3f30fba73d74",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinybox-0.1.28-fedora-43-arm64.tar.gz",
            sha256: "3a0631524f050f65620c982ef271c4431e14d489270164e98fb1b6fadc059b1a",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinybox-0.1.28-fedora-43-x86_64.tar.gz",
            sha256: "e2cca3925abea1ddf1bacce5bdd36e2fc7b8d1381febc5693ed56ce8a990c619",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinybox-0.1.28-fedora-44-arm64.tar.gz",
            sha256: "fb49daf9ac01ae11f0b540a68533deacb20ea8f3d28c5a48e48096a789672d0a",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinybox-0.1.28-fedora-44-x86_64.tar.gz",
            sha256: "bdf769ad3890d9091448e2b0465e3efd7c06b8007837cc170965e559fd51c1fa",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinybox-0.1.28-macos-15-arm64.tar.gz",
            sha256: "875c299b57dc843fc2c4586eee7ca81b5d3dc278d02c55747bfe3f7f0ac1b0b5",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinybox-0.1.28-macos-15-x86_64.tar.gz",
            sha256: "bb72d856aa4a7b192debd6be123922de9e9246e8359268fb2f38f77cbefddcde",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinybox-0.1.28-macos-26-arm64.tar.gz",
            sha256: "baf77b4fcb475eeff9895f8916d69865e93bd6884ec73cd52b37e79a8a3ddb37",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinybox-0.1.28-macos-26-x86_64.tar.gz",
            sha256: "99ad8e64ed8c4a4d06183e896e8186a29eb7f20353dd22732bad62561d36485d",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinybox-0.1.28-ubuntu-22.04-arm64.tar.gz",
            sha256: "b88dab7ae9f88840ace3ce6e93f4fad5a6409473e70c8f9c953fe318cfee79a9",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinybox-0.1.28-ubuntu-22.04-x86_64.tar.gz",
            sha256: "1e1f23048d552d6cefbc95d36f8f6a190ce5e344c1d9af6bebd67a777eaaf927",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinybox-0.1.28-ubuntu-24.04-arm64.tar.gz",
            sha256: "91e1ee4dea11529b3e698d216901cc0efdb73596b8d5cdf36ad0dd9501cb6905",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinybox-0.1.28-ubuntu-24.04-x86_64.tar.gz",
            sha256: "92872c39c8685c2300d32ea43312420fb2991ad55b582a4c67b5684870c51659",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinybox-0.1.28-windows-11-arm64.zip",
            sha256: "e46d479f8c2048748b9fbd800c85fea6e126705eff093e40dccad8a37a898d20",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinybox-0.1.28-windows-2022-x86_64.zip",
            sha256: "310baff90e4d654a41ba3137c3c7656a972cdb36495e83df566ad1e291c3da1a",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinybox-0.1.28-windows-2025-x86_64.zip",
            sha256: "87b7456db936604b5d449780bd3a37ec603c26f993a7d918778f966fd831c4e5",
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
