//! Registry records for additional first-party TinyBus modules.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset};

/// The `tinybox` module, loaded on demand.
pub(crate) const TINYBOX: ModuleRecord = ModuleRecord {
    id: "tinybox",
    description: "Sandbox execution and lifecycle operations through TinyBox",
    bus_name: "ai.tinyhumans.tinybox.Box",
    object_path: "/ai/tinyhumans/tinybox/Box",
    version: "0.1.21",
    release_url: "https://github.com/tinyhumansai/tinybox/releases/tag/v0.1.21",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-rolling-x86_64",
            archive: "tinybox-0.1.21-archlinux-rolling-x86_64.tar.gz",
            sha256: "4965edcc3a8f7d1632049a0b8db8f3401058e0ef0b92abf94e9656f61c2fc711",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinybox-0.1.21-fedora-43-arm64.tar.gz",
            sha256: "ae9245da0eb2fc875755d3ec626c6ab87bc595586b150951433f4f76046f5b44",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinybox-0.1.21-fedora-43-x86_64.tar.gz",
            sha256: "4342711e82128cf78219fc3dde8d87b1687f181ca0f6b0ed320bf3d2cb9b97d3",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinybox-0.1.21-fedora-44-arm64.tar.gz",
            sha256: "e942990dddde0445e07aa30459aa8d0bc2da1c0908d7e10dd338b5504c295daa",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinybox-0.1.21-fedora-44-x86_64.tar.gz",
            sha256: "6a134d9bccb5c2320f290fcdf32f27fca1dc71256b4d1ea4419d393ca41d178d",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinybox-0.1.21-macos-15-arm64.tar.gz",
            sha256: "9c4afa9c05743c0b087d8666b7ce4bf6b202796456263070596d52f6d6ba172d",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinybox-0.1.21-macos-15-x86_64.tar.gz",
            sha256: "839d9a9f0757224c087c11f553d7be3f756f3bb70018b1ed9152a78906ffebfd",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinybox-0.1.21-macos-26-arm64.tar.gz",
            sha256: "222fee7d8e8e257e2275139295254ae7db4c527cae27217f2382404125cb90b8",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinybox-0.1.21-macos-26-x86_64.tar.gz",
            sha256: "64df0211efeca51c0e5325e3d82206b97839c0a4451c1b98ca4cf1cfafee1a3d",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinybox-0.1.21-ubuntu-22.04-arm64.tar.gz",
            sha256: "957dfb09285808d6017d26c2676ec9a21a91f4b7b1cef23cd0156e6b0fda1a94",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinybox-0.1.21-ubuntu-22.04-x86_64.tar.gz",
            sha256: "493eacdd629cb757a32504be05daa0966e6f4ed01c319885c69aa353ab32367a",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinybox-0.1.21-ubuntu-24.04-arm64.tar.gz",
            sha256: "dd78799d2c0b2cc06d4d4cff7c0b4ccafc11f7dcf8b89da4aa694fa92744a737",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinybox-0.1.21-ubuntu-24.04-x86_64.tar.gz",
            sha256: "bb790cc7543bd9887e57ecde60265720823e2b5312b4991e4d5ad9693e998b6a",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinybox-0.1.21-windows-11-arm64.zip",
            sha256: "2aab102e289da4e9e48aba0da76d32aa548107280a58ba5aa67cb3fe626f9cdc",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinybox-0.1.21-windows-2022-x86_64.zip",
            sha256: "526d1150312011a513cfd54d3c66d657b48b9cdd04eabee9d6321fbba04b39d6",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinybox-0.1.21-windows-2025-x86_64.zip",
            sha256: "1583415db7f80f78baad876a7ea897b2a1a99bb33db830e3d3c89573d17b664e",
        },
    ],
    load: LoadPolicy::Lazy,
};

/// The `tinychannels` module, loaded on demand.
pub(crate) const TINYCHANNELS: ModuleRecord = ModuleRecord {
    id: "tinychannels",
    description: "Channel provider lifecycle and message transport",
    bus_name: "ai.tinyhumans.tinychannels.Channels",
    object_path: "/ai/tinyhumans/tinychannels/Channels",
    version: "0.1.26",
    release_url: "https://github.com/tinyhumansai/tinychannels/releases/tag/v0.1.26",
    assets: &[
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinychannels-module-0.1.26-macos-15-arm64.tar.gz",
            sha256: "5924dc78a1b2931c526b5ca2590c675455eaca6db914c648a321ea9225d65142",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinychannels-module-0.1.26-macos-15-x86_64.tar.gz",
            sha256: "9be7ad6b11e9dcf14cbe8dba246e8aab55da29b03b60c36cd1a81dc250e312ac",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinychannels-module-0.1.26-macos-26-arm64.tar.gz",
            sha256: "466d6ceaf91735d278e65280f315efa5cfec7e609c9fe12cb8ff4e545846a4e3",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinychannels-module-0.1.26-macos-26-x86_64.tar.gz",
            sha256: "2f3636ac9bf0b5fadd4b2539730064437770dee25442991591842cf4f5e34d8c",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinychannels-module-0.1.26-ubuntu-22.04-arm64.tar.gz",
            sha256: "2456cb336de10471199ba38cc680b955242837b49e9b7ef72cf80a63516e834d",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinychannels-module-0.1.26-ubuntu-22.04-x86_64.tar.gz",
            sha256: "07fba7fb3d730cfebcc6e5fe8ecbc9c799e1d3b207cb3ad251d6579d965cd6b0",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinychannels-module-0.1.26-ubuntu-24.04-arm64.tar.gz",
            sha256: "141c7d769728070df7bce8bd1f8cf4e9e43501f55fa4d055750842d2f9da6734",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinychannels-module-0.1.26-ubuntu-24.04-x86_64.tar.gz",
            sha256: "ed507c285fd63b0b6ea377b5ae8840dda549a1e03442d95157ca9a5878fe7fa6",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinychannels-module-0.1.26-windows-11-arm64.zip",
            sha256: "834bb5d76b42d32840059ee40d0b1ee3c4aa59b03c0882994228a8013c6c6c56",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinychannels-module-0.1.26-windows-2022-x86_64.zip",
            sha256: "fc33d7cd4ebb2bc8b7dcacc33ab88d174d07d5b73e4c76eaa22bd74401033b08",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinychannels-module-0.1.26-windows-2025-x86_64.zip",
            sha256: "f0778807f16f022f7cebaba7f426d5453ed66313090be031418cf41743208938",
        },
    ],
    load: LoadPolicy::Lazy,
};

/// The `tinyhosts` module, loaded on demand.
pub(crate) const TINYHOSTS: ModuleRecord = ModuleRecord {
    id: "tinyhosts",
    description: "Hosting provider operations",
    bus_name: "ai.tinyhumans.tinyhosts.Hosting",
    object_path: "/ai/tinyhumans/tinyhosts/Hosting",
    version: "0.3.0",
    release_url: "https://github.com/tinyhumansai/tinyhosts/releases/tag/v0.3.0",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyhosts-0.3.0-ubuntu-24.04-x86_64.tar.gz",
            sha256: "368f00ee397649481ff8f61d33eeb8b908cc416d6b2f9d4ef3c81263d6b0460e",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyhosts-0.3.0-ubuntu-24.04-arm64.tar.gz",
            sha256: "e298942fddec8930fd78651632c02abe78968a17ab3589608875b8dc4f731669",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyhosts-0.3.0-ubuntu-22.04-x86_64.tar.gz",
            sha256: "f7c9484176b62357fb449eb79830e3df0eebfd4e0814c8d8e1c58669d829f482",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyhosts-0.3.0-ubuntu-22.04-arm64.tar.gz",
            sha256: "576ff6204ccaa1e2dc2ef1b27762848efa4555d4d32ef46b4c329daf1e04eac3",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyhosts-0.3.0-macos-26-arm64.tar.gz",
            sha256: "b0083e70b567659d095a1ff0f9d2c8cb5bafe110d90fb08527c509362ce77a55",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyhosts-0.3.0-macos-26-x86_64.tar.gz",
            sha256: "44fa7c9efde0c388f154f154809ce2829460bae4931f27d20f8f0d3ee3bf2a8c",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyhosts-0.3.0-macos-15-arm64.tar.gz",
            sha256: "56ca93b2cab2010ff599c38179eb99f6ad67631e39f671be973256c3d9142381",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyhosts-0.3.0-macos-15-x86_64.tar.gz",
            sha256: "0ffb9fc8d1c7fc7d95a7808af7722d8c6537a508c0c3a294ecf8fb8a28430207",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyhosts-0.3.0-windows-2025-x86_64.zip",
            sha256: "3d6d5f7fd0806ac6fd6c2ed0c60a9155d6c09a861ebd1ac0c43864e969f88fb9",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyhosts-0.3.0-windows-2022-x86_64.zip",
            sha256: "a5003f1254052828d6fa362b993b0bed6b7256c252f969536090e148378c331f",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyhosts-0.3.0-windows-11-arm64.zip",
            sha256: "21f8909a2a81a541e3d0347510b2bf25baec59438a5536edcd57e65fae7f4a4d",
        },
    ],
    load: LoadPolicy::Lazy,
};
