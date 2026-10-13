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
    version: "0.1.31",
    release_url: "https://github.com/tinyhumansai/tinychannels/releases/tag/v0.1.31",
    assets: &[
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinychannels-module-0.1.31-macos-15-arm64.tar.gz",
            sha256: "cffe8ed587aba386971c10fb22a27f4088c88f94fe11551a947f3908e365b150",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinychannels-module-0.1.31-macos-15-x86_64.tar.gz",
            sha256: "1c72499c0cb3ef19f39cc08df86ecedbfac5ff5c646407a3d097c2726f656e42",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinychannels-module-0.1.31-macos-26-arm64.tar.gz",
            sha256: "f921babea4035542d8459253090b077baa23d004abe21739af804537efb1e672",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinychannels-module-0.1.31-macos-26-x86_64.tar.gz",
            sha256: "3b827f24d58a566d17869adc29f8478c1b0c6c1e26b4cc54faf2cdf0f5fceb63",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinychannels-module-0.1.31-ubuntu-22.04-arm64.tar.gz",
            sha256: "619887ff89c917155ccf2f3f76f67c2d20729dad4eba8f49ebc1cf48bdbe4f0f",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinychannels-module-0.1.31-ubuntu-22.04-x86_64.tar.gz",
            sha256: "747f5da64c9fd0225ffebb284df65480ea03aff5f4a3ba507190f8295fedde18",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinychannels-module-0.1.31-ubuntu-24.04-arm64.tar.gz",
            sha256: "d5c0fc807b26eb2d22996818510a1226f6a31cdcefa12353d0b35fb510effb90",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinychannels-module-0.1.31-ubuntu-24.04-x86_64.tar.gz",
            sha256: "22d3cf8f364bd3a14f6374488e493bd619bed338de4a02f635cee369472dcbc8",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinychannels-module-0.1.31-windows-11-arm64.zip",
            sha256: "e06f1a3495e1879a709b8eedc4389653dffc7a2ab7e1d19c6b4f6d534f37b5fb",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinychannels-module-0.1.31-windows-2022-x86_64.zip",
            sha256: "37501850dc658d7e6b1241d8e01a804492efd02e4c9b28a558734138206c3020",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinychannels-module-0.1.31-windows-2025-x86_64.zip",
            sha256: "8e3500ed6dc073ece37269b13351ceb4f4955cf0d5745c7fb1e2cc08528ca2b1",
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
