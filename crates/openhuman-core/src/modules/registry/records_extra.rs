//! Registry records for additional first-party TinyBus modules.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

/// The `tinybox` module, loaded on demand.
#[cfg(feature = "modules")]
pub(crate) const TINYBOX: ModuleRecord = ModuleRecord {
    id: "tinybox",
    description: "Sandbox execution and lifecycle operations through TinyBox",
    bus_name: tinybox_bus::INTERFACE,
    object_path: tinybox_bus::OBJECT_PATH,
    version: "0.1.30",
    release_url: "https://github.com/tinyhumansai/tinybox/releases/tag/v0.1.30",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinybox-0.1.30-archlinux-arm64.tar.gz",
            sha256: "1ed931f3bc8299cf11a67f01bebec2d498ba2d65b011779c39aad2559234523c",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinybox-0.1.30-archlinux-x86_64.tar.gz",
            sha256: "942f202c6888f93fb55697f34e94088df736c2aa1d7627bd010d1fc82a8de6eb",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinybox-0.1.30-fedora-43-arm64.tar.gz",
            sha256: "9d0f4114ffa740d88f020107e66048a24ddab87b595d392f56083e2b58695447",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinybox-0.1.30-fedora-43-x86_64.tar.gz",
            sha256: "29bdb2e3d629d6983cd81c75c48ffce09383071a02f8f6069c7f70bbfe0f15ee",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinybox-0.1.30-fedora-44-arm64.tar.gz",
            sha256: "54d706bf29de5356c8709159461bb4e7b8a69f15a7437992b747938b73b26fbe",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinybox-0.1.30-fedora-44-x86_64.tar.gz",
            sha256: "2c2bd426ce201639527c69d73916f3c83eb1b086195340dc45c36c0426d76ba5",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinybox-0.1.30-macos-15-arm64.tar.gz",
            sha256: "b671dc6321939d189058724ad1d668e835026af44c6634837dc29b0285ef16fc",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinybox-0.1.30-macos-15-x86_64.tar.gz",
            sha256: "1c09e0e25456c4f9b62199807a2d5ce77926ff73787636e42d3aced255f8b81c",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinybox-0.1.30-macos-26-arm64.tar.gz",
            sha256: "de601f633e842751d22c717135436fb4616cb6956f7f01556857e2d731c6a519",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinybox-0.1.30-macos-26-x86_64.tar.gz",
            sha256: "8fdc30a945847689832b569d6ffe47f7a46bd913a297d85d4442eb9f946e1cb5",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinybox-0.1.30-ubuntu-22.04-arm64.tar.gz",
            sha256: "25236b2e0de828acdff256d23b80fc722a724c95b772ec37cb03b7ada7dce8c8",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinybox-0.1.30-ubuntu-22.04-x86_64.tar.gz",
            sha256: "757301472a233da55dd39d70a8688a0b4bf7c04c6c0604bd3cbc2df56f7506e2",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinybox-0.1.30-ubuntu-24.04-arm64.tar.gz",
            sha256: "5a277e2fd46453d961c4c9a09b9c33e5810d816f7dc8f057e620dd73532a2c76",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinybox-0.1.30-ubuntu-24.04-x86_64.tar.gz",
            sha256: "74787ae77f683cf6d01b4a337cb7515a60c5f2fe2ca79d33e21755477c4cdf80",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinybox-0.1.30-windows-11-arm64.zip",
            sha256: "2aede8c034cc37d0df38b2490aea4db2face458255322250841893af031ea190",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinybox-0.1.30-windows-2022-x86_64.zip",
            sha256: "514e9215c711f7e35c6d6fd5257b73109d1670db5297bb647e98d3882821aa2f",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinybox-0.1.30-windows-2025-x86_64.zip",
            sha256: "d5353e969d5a0557f25c9d4846b861e2c0a4e1dcc48f12f1b23de2dcb23b1a4b",
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
