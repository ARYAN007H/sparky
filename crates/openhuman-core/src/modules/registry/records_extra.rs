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
    version: "0.1.13",
    release_url: "https://github.com/tinyhumansai/tinychannels/releases/tag/v0.1.13",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinychannels-module-0.1.13-ubuntu-24.04-x86_64.tar.gz",
            sha256: "3fe360f6863a01e2fe48737d9895fd997467121ee1fa9bb30e7f875368c9b5c9",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinychannels-module-0.1.13-ubuntu-24.04-arm64.tar.gz",
            sha256: "e636f26b9484b351409f1c1ac0f22ee61435a24961cb0f57e2b7537774b0934a",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinychannels-module-0.1.13-ubuntu-22.04-x86_64.tar.gz",
            sha256: "f0ca5091e42c1493311db34015d4485a557700100feac4245468c3d0f64eef08",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinychannels-module-0.1.13-ubuntu-22.04-arm64.tar.gz",
            sha256: "2824102014b345c21c9d7e16fae8917cdf6fea03529a67de1ee58065f1a694e6",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinychannels-module-0.1.13-macos-26-arm64.tar.gz",
            sha256: "81e16aa479839a673b03a18bd715ebc1b5efef7e6f223953233688ad08d1bbf2",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinychannels-module-0.1.13-macos-26-x86_64.tar.gz",
            sha256: "dc82f10309946132e7837fb97c86203d79741416536b0d944972cb8acaa3f220",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinychannels-module-0.1.13-macos-15-arm64.tar.gz",
            sha256: "7770a906ac3bb46e89725363d723e32709b45300d234b360c4822666c29f7c51",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinychannels-module-0.1.13-macos-15-x86_64.tar.gz",
            sha256: "cbd540871efc81822181ddd51502c93f31e2a99a8be58b725d01bc70d7d9ba6d",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinychannels-module-0.1.13-windows-2025-x86_64.zip",
            sha256: "a7fee0a4bcbb52b4f7866eab6a1c4eca31fa7de8541c8321c61314c1286f981a",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinychannels-module-0.1.13-windows-2022-x86_64.zip",
            sha256: "1a74d967ac4f859d6c210865788fcdb124accc027e75823eb3d9a55a700a0dcd",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinychannels-module-0.1.13-windows-11-arm64.zip",
            sha256: "cbcaaeed4c6e2c3baae1474686e210281c16d8f75024d1a4805f68cbbc8f1f67",
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
