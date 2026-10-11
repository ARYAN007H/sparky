//! Registry records for additional first-party TinyBus modules.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset};

/// The `tinybox` module, loaded on demand.
pub(crate) const TINYBOX: ModuleRecord = ModuleRecord {
    id: "tinybox",
    description: "Sandbox capability discovery through TinyBox",
    bus_name: "ai.tinyhumans.tinybox.Box",
    object_path: "/ai/tinyhumans/tinybox/Box",
    version: "0.1.19",
    release_url: "https://github.com/tinyhumansai/tinybox/releases/tag/v0.1.19",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinybox-0.1.19-ubuntu-24.04-x86_64.tar.gz",
            sha256: "a1dae0e4af4a1ba84fc37b048ad1d1ef0e226ec2c2b312d7607a8ec08d09d134",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinybox-0.1.19-ubuntu-24.04-arm64.tar.gz",
            sha256: "245eaf3ab5465e9e01da207ef63bf930c46069b3858fe9ce9da8e71559c0faf8",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinybox-0.1.19-ubuntu-22.04-x86_64.tar.gz",
            sha256: "c23102c295b8e5ceef428d1ed178ac910031a23640eaebceb7ebfe3c19a2e05c",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinybox-0.1.19-ubuntu-22.04-arm64.tar.gz",
            sha256: "b81e147875a5f21b9b681dffa633c480cfa0bf0121e3ea309feb859d7206d866",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinybox-0.1.19-macos-26-arm64.tar.gz",
            sha256: "437ae2b09337b30ad1af85f3fe95be5cc0d8134986f401a231448a37c340f54b",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinybox-0.1.19-macos-26-x86_64.tar.gz",
            sha256: "24c3397596e20e3661e6fc6d0ba28f2f36a8c3d7f4e5106030f674d5d3703f6d",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinybox-0.1.19-macos-15-arm64.tar.gz",
            sha256: "61145d8dbb56d866d85610702938973b7c8f523093154ec3a27790dc060565bd",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinybox-0.1.19-macos-15-x86_64.tar.gz",
            sha256: "8e68322fe3ff762ad6b31a55413cfd7772174b2bdadc0b5391d1816d36a6b373",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinybox-0.1.19-windows-2025-x86_64.zip",
            sha256: "8ac5b73f23deee35d2f61209b801a5fcb6465277e35afc45dac4e91cfe528ce0",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinybox-0.1.19-windows-2022-x86_64.zip",
            sha256: "f2682b28908c9736382fab57c0f579db4a17ae3e037d9422f185a0f949e69143",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinybox-0.1.19-windows-11-arm64.zip",
            sha256: "50873a041c05d009f8f65812e6ac95fef001b1d71a897383da1f34fac1f2a7d0",
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
    version: "0.1.24",
    release_url: "https://github.com/tinyhumansai/tinychannels/releases/tag/v0.1.24",
    assets: &[
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinychannels-module-0.1.24-macos-15-arm64.tar.gz",
            sha256: "2b77fa9632dd64be8c8dfa1ebb092a56825fabe227e8c62f014ae5de2d54a2d6",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinychannels-module-0.1.24-macos-15-x86_64.tar.gz",
            sha256: "996e3d96e0391c485947038741293e3502f26843ae33708d615eb626ded70123",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinychannels-module-0.1.24-macos-26-arm64.tar.gz",
            sha256: "c5d2a85417fcdb891c7027940f11dc427c7bcfa8f487e2743d255776c288c612",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinychannels-module-0.1.24-macos-26-x86_64.tar.gz",
            sha256: "557e021f151c3efd45899b26d4bb0d2927316b732fbe636ab223d917a303da61",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinychannels-module-0.1.24-ubuntu-22.04-arm64.tar.gz",
            sha256: "7e5852d3887562f90e05d3138ceb725f9ffed8c2e8d0ff6b3df4610716115a0b",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinychannels-module-0.1.24-ubuntu-22.04-x86_64.tar.gz",
            sha256: "792f434520e950c15f69d6fe88881cd4c572cbee35e0604455945876911bbdb3",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinychannels-module-0.1.24-ubuntu-24.04-arm64.tar.gz",
            sha256: "2b856dcca09ea3218e50ef1ac4f32dbf66aef7308e140d44500549863ef118a3",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinychannels-module-0.1.24-ubuntu-24.04-x86_64.tar.gz",
            sha256: "3393a40617666778da2177914b0fbfb151b985d2500e2ce5b44a62761f2ad1f0",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinychannels-module-0.1.24-windows-11-arm64.zip",
            sha256: "8f7ca3adc8fa3dba511c6adb2e99309b9676c91fd0dca5b8234c29ece8ec2cbb",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinychannels-module-0.1.24-windows-2022-x86_64.zip",
            sha256: "be7f0a3f10f55b5cdb699d826c4a12157e1e0ca4d99a915d66c3ef8bd2c2e46a",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinychannels-module-0.1.24-windows-2025-x86_64.zip",
            sha256: "47c312bf9b96a636b1ea22806e675d0118c5986d9058a22addecb634bf29a1f7",
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
