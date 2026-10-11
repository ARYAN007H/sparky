//! Registry records for additional first-party TinyBus modules.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset};

/// The `tinybox` module, loaded on demand.
pub(crate) const TINYBOX: ModuleRecord = ModuleRecord {
    id: "tinybox",
    description: "Sandbox capability discovery through TinyBox",
    bus_name: "ai.tinyhumans.tinybox.Box",
    object_path: "/ai/tinyhumans/tinybox/Box",
    version: "0.1.18",
    release_url: "https://github.com/tinyhumansai/tinybox/releases/tag/v0.1.18",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinybox-0.1.18-ubuntu-24.04-x86_64.tar.gz",
            sha256: "232934349cb0ece8f049c00e5d15e38e30248a100cd9b66b203e69aee23145da",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinybox-0.1.18-ubuntu-24.04-arm64.tar.gz",
            sha256: "994a37f95e128cd1b54999a9d4eebf012eaeb17b6a3006c9985a83b10e4ee6e1",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinybox-0.1.18-ubuntu-22.04-x86_64.tar.gz",
            sha256: "f8210816dfacd7ee58dcfb4482845f40ee4dbb435f706c2e154e9542a15f8329",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinybox-0.1.18-ubuntu-22.04-arm64.tar.gz",
            sha256: "b192c1ce4333b8668ec9f38a0197aeb443c5a9afd63d5f9b617a6de97f080f4b",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinybox-0.1.18-macos-26-arm64.tar.gz",
            sha256: "6d7c9120f6f773861ed0901360142ad0c131fe9597d937a21788891a6cae9663",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinybox-0.1.18-macos-26-x86_64.tar.gz",
            sha256: "7e61a78a8ea82aec31692d84979169ba72202ac7698ecac7cf515445f911473d",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinybox-0.1.18-macos-15-arm64.tar.gz",
            sha256: "b827b00712d8ea5eaf081be9bb8d9e5283dd29fbf34c1dc8d26d15a64224483b",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinybox-0.1.18-macos-15-x86_64.tar.gz",
            sha256: "c4526cc2f05137d1d82d8a41681403c3c74d0c585a7ff51c4dd49ff3f2eb9694",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinybox-0.1.18-windows-2025-x86_64.zip",
            sha256: "c320392f979d3298c4bcecb4b54f18db40d49bb546634a441219eef9f5fbf592",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinybox-0.1.18-windows-2022-x86_64.zip",
            sha256: "d06c300eb7651147e9c7672e1377f25cc88e824be6afadfdafd4ec7eb6f5f8d1",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinybox-0.1.18-windows-11-arm64.zip",
            sha256: "b3a26410b6a7adda83ae9ce23e0716367e06c3561e8a2f9ffbe30b6820ccae89",
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
    version: "0.1.14",
    release_url: "https://github.com/tinyhumansai/tinychannels/releases/tag/v0.1.14",
    assets: &[
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinychannels-module-0.1.14-macos-15-arm64.tar.gz",
            sha256: "be83aeca16520766708dd473a52e8b71f1d1d283fae82e5a8c89cb4e45d140bc",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinychannels-module-0.1.14-macos-15-x86_64.tar.gz",
            sha256: "435b6ec18b25be5bd0b63c5c3cf3d5d688e9934204189b645d13fee29495bbdd",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinychannels-module-0.1.14-macos-26-arm64.tar.gz",
            sha256: "8379e9d60dac93bb6b3f6efe466514767023fa45099922c8eee10acaa9aeff24",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinychannels-module-0.1.14-macos-26-x86_64.tar.gz",
            sha256: "753df946102a41ff119d05b4733ce3ee07483d0b63e2bd94ab8db6ac0fbf319a",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinychannels-module-0.1.14-ubuntu-22.04-arm64.tar.gz",
            sha256: "079e0bcb34bf75b711a6b5ca17f40b2d7d10296e2dba56bbc3cf18f59301e5e9",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinychannels-module-0.1.14-ubuntu-22.04-x86_64.tar.gz",
            sha256: "fe00f8199bdca8ff3d860371cf462526bcc284bc4c343ad6059df0a61e337b15",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinychannels-module-0.1.14-ubuntu-24.04-arm64.tar.gz",
            sha256: "33a7f5d131f9a223cddaeaecfb09a176d7f9b021edeaa2ea0d6d7ca1f4d7d1f4",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinychannels-module-0.1.14-ubuntu-24.04-x86_64.tar.gz",
            sha256: "88dd9becede87cd4ffaf4587d16379f61b607a7cbb302ba421f1e46d32f14edd",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinychannels-module-0.1.14-windows-11-arm64.zip",
            sha256: "c2b072246e3dd0e33ebed351cae7e57f30e07f8b89490ed501495b23dd5b01ab",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinychannels-module-0.1.14-windows-2022-x86_64.zip",
            sha256: "cd3794169dbce1b4f08386f30d47b662f3e6ba785fdbf131530fbd2986756473",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinychannels-module-0.1.14-windows-2025-x86_64.zip",
            sha256: "ed50dddcd9b6c07e9286977e28c3f836cb878db5e87e46a4d83aa04104360b90",
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
