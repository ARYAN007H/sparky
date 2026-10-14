//! Registry records for additional first-party TinyBus modules.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

/// The `tinybox` module, loaded on demand.
#[cfg(feature = "modules")]
pub(crate) const TINYBOX: ModuleRecord = ModuleRecord {
    id: "tinybox",
    description: "Sandbox execution and lifecycle operations through TinyBox",
    bus_name: tinybox_bus::INTERFACE,
    object_path: tinybox_bus::OBJECT_PATH,
    version: "0.1.31",
    release_url: "https://github.com/tinyhumansai/tinybox/releases/tag/v0.1.31",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinybox-0.1.31-archlinux-arm64.tar.gz",
            sha256: "f261dc7992df85dd2fd42e671c40ca90e87da8a7c9c7a246cb65ebec92070ae0",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinybox-0.1.31-archlinux-x86_64.tar.gz",
            sha256: "aa0e7a91b7772c80c2a0be3cd99269d93b80a922d46be184eee011754b8b2af7",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinybox-0.1.31-fedora-43-arm64.tar.gz",
            sha256: "62af32f2ab2b90133df13ec7037cd0f5b5c160ee94a9828ccd1f20c20455bbfc",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinybox-0.1.31-fedora-43-x86_64.tar.gz",
            sha256: "f5144f0926ca50a27510029ca24afaeb22c95ce3732a45113cef5c6f86d5e14b",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinybox-0.1.31-fedora-44-arm64.tar.gz",
            sha256: "893368d05de8847c96d087a7b186a247485483a60a377b8dca39a7eaec48c8f4",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinybox-0.1.31-fedora-44-x86_64.tar.gz",
            sha256: "ffbbd8deb020ff210478bb267b64669429da21667c83baecc83025094d71165a",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinybox-0.1.31-macos-15-arm64.tar.gz",
            sha256: "9c7c04d8be65a88fee4cfe1ed763c2076dea3aa60794539d55167fa72968cce7",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinybox-0.1.31-macos-15-x86_64.tar.gz",
            sha256: "f976d44ea0e08ce34e845f593897c558b69e787fae1a36fa4fbf20e250bf5d35",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinybox-0.1.31-macos-26-arm64.tar.gz",
            sha256: "abd59c035f8f452ee5466e46a731e6d39adaf478c08d562e8fc3ffdf5d88b8a6",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinybox-0.1.31-macos-26-x86_64.tar.gz",
            sha256: "73dd91aee335ff0437adb831f2e22d1d891587518a1e9fd5f6ff094b562669cc",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinybox-0.1.31-ubuntu-22.04-arm64.tar.gz",
            sha256: "a892040113bde90a1aa8d2c3cf0721be27b58ff3ae3c90341024af2b91137839",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinybox-0.1.31-ubuntu-22.04-x86_64.tar.gz",
            sha256: "f73d8d5a31b3db2aff802594b6219af69ac9b04638cd23154e32fd32247c578c",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinybox-0.1.31-ubuntu-24.04-arm64.tar.gz",
            sha256: "dd140c0da699ee91f65593424a98a071f5af9f7f4bcc6e29bd6a825493e5071a",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinybox-0.1.31-ubuntu-24.04-x86_64.tar.gz",
            sha256: "addaf5c10364d05d4afa5e7b7780de329638c4e08f07027d8cd7bce313ea89f2",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinybox-0.1.31-windows-11-arm64.zip",
            sha256: "994c21e2f77c14323bdf186a4100bbd7077e1fa6efb5c20501413de54fae9ac3",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinybox-0.1.31-windows-2022-x86_64.zip",
            sha256: "e5c0599d7ac90b254c946a8d544a8c18d1ea3b7e2b8eadc5cce4fbc2f8e3b41a",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinybox-0.1.31-windows-2025-x86_64.zip",
            sha256: "136291f2ae3df0f509cea6573e926f660cbb55bf3a5e0305cbda22677b9fa579",
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
    version: "0.1.32",
    release_url: "https://github.com/tinyhumansai/tinychannels/releases/tag/v0.1.32",
    assets: &[
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinychannels-module-0.1.32-macos-15-arm64.tar.gz",
            sha256: "0322f515b80548cddcf0b731f722fa26ff494d2afd30bfbcb65f6780f06f20c3",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinychannels-module-0.1.32-macos-15-x86_64.tar.gz",
            sha256: "90525be14322388c88b72d1376cab6fa128e572bddcbdfb4e0dcef21aee19127",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinychannels-module-0.1.32-macos-26-arm64.tar.gz",
            sha256: "b4649d9d8bb28310c91b35d51149848b3990afc674bf8f45625a33befcb1ecd0",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinychannels-module-0.1.32-macos-26-x86_64.tar.gz",
            sha256: "e5461a8b34bc110e023d5f72c102923dc31ef9726ba90e2dc542fb86c35ea505",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinychannels-module-0.1.32-ubuntu-22.04-arm64.tar.gz",
            sha256: "4f6b4d343775f850ffa2d53c7ea318ee71cc399058355eddd57f711158a2c3fc",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinychannels-module-0.1.32-ubuntu-22.04-x86_64.tar.gz",
            sha256: "f39ac40d83bd2e6df22d613bd45ceb3e4d53a41e11a79fd405fdf3deae60695a",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinychannels-module-0.1.32-ubuntu-24.04-arm64.tar.gz",
            sha256: "3504e2f88904f1834d5bce9a09924b32fc1a06cb863c25a780dfdab87cc86c95",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinychannels-module-0.1.32-ubuntu-24.04-x86_64.tar.gz",
            sha256: "a51bfe7a27260df3683f3e99a0a04ca1088dede0324486da95c1a24f4deb07a6",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinychannels-module-0.1.32-windows-11-arm64.zip",
            sha256: "b2e28cbec56b18f510062370a17784779ee285dda057084d9f9ca9b9456e1b38",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinychannels-module-0.1.32-windows-2022-x86_64.zip",
            sha256: "313de6b8e1c234e1fa44d08d761503a2ef7c8d6790fbda8a96c7cf2fe013c06d",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinychannels-module-0.1.32-windows-2025-x86_64.zip",
            sha256: "fa425b3c0de352c5048aa19c367dacecd05351e09fd5c76ddd5e42138510c103",
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
    version: "0.4.1",
    release_url: "https://github.com/tinyhumansai/tinyhosts/releases/tag/v0.4.1",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyhosts-0.4.1-ubuntu-24.04-x86_64.tar.gz",
            sha256: "7e8a72b7951db71cc98c94fed0908660a58bd3f2cbf23103d953e28e1315d4d5",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyhosts-0.4.1-ubuntu-24.04-arm64.tar.gz",
            sha256: "27cbc5a233292850797adc7b51d9d654a10d4aa77ec285a7e564e378889e0a03",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyhosts-0.4.1-ubuntu-22.04-x86_64.tar.gz",
            sha256: "a03588ef8b012aea29f2b2b7105fde6c3c7b4bb0dc6f93d99868e91468c2b6d6",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyhosts-0.4.1-ubuntu-22.04-arm64.tar.gz",
            sha256: "80b26591eb7beff4e27de9e6239b919ff6c17b00ead11d22bae8e7fcb0d33c15",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyhosts-0.4.1-macos-26-arm64.tar.gz",
            sha256: "ba40e889ebd450edd791865945b4146b2f243bc885f4228463e65b0a4ea8074e",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyhosts-0.4.1-macos-26-x86_64.tar.gz",
            sha256: "eeb2a66603374934724036e9fb86fcba0c22a2fd3aa168127cca5960eb5a19aa",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyhosts-0.4.1-macos-15-arm64.tar.gz",
            sha256: "b67a0aecb0527af68d5b5428d55d897ece11500e2ee9798b3d133ef0df7344b7",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyhosts-0.4.1-macos-15-x86_64.tar.gz",
            sha256: "7582dc327e247b28e657a01c8f8a4cb0924c8211144dfb24e4917868cd237354",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyhosts-0.4.1-windows-2025-x86_64.zip",
            sha256: "42c2a5b98eb5c51a8a05731e762af801c585383024d27cf4cd8bb01f9014308f",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyhosts-0.4.1-windows-2022-x86_64.zip",
            sha256: "0d579e0d08856307881b8965bf78762f0064b129861e13e796f91fcb9895ccaf",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyhosts-0.4.1-windows-11-arm64.zip",
            sha256: "67b64f0917fa0cf561aa08dfe0c74818e9ac9066a9f03c03e6b216f7af87aea3",
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
