//! Registry records for additional first-party TinyBus modules.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

/// The `tinybox` module, loaded on demand.
#[cfg(feature = "modules")]
pub(crate) const TINYBOX: ModuleRecord = ModuleRecord {
    id: "tinybox",
    description: "Sandbox execution and lifecycle operations through TinyBox",
    bus_name: tinybox_bus::INTERFACE,
    object_path: tinybox_bus::OBJECT_PATH,
    version: "0.1.38",
    release_url: "https://github.com/tinyhumansai/tinybox/releases/tag/v0.1.38",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinybox-0.1.38-archlinux-arm64.tar.gz",
            sha256: "0e2b68b98239e8822cbf5e79114e68060639541840a36b51d577c830982100df",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinybox-0.1.38-archlinux-x86_64.tar.gz",
            sha256: "71c434c09574c21833f5313c3baf4646663dfc277a3b82826830d293287d06c3",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinybox-0.1.38-fedora-43-arm64.tar.gz",
            sha256: "6c0bfecb1aeb97d42b9319754865d2c8595e6310d75ec61358e9d0131efe4ac6",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinybox-0.1.38-fedora-43-x86_64.tar.gz",
            sha256: "d6280f22c8f2c7a7793974d8fb8872285ab8cc2615cdb0b5b21093c85de5075d",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinybox-0.1.38-fedora-44-arm64.tar.gz",
            sha256: "83617a9497c27b6a2597fc8dc9131de054f7c957f114a09ef9a9c9da837d1524",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinybox-0.1.38-fedora-44-x86_64.tar.gz",
            sha256: "daf26b47941f9fc5d0d65327beff3f38efe374241401e8717e216235b4970bef",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinybox-0.1.38-macos-15-arm64.tar.gz",
            sha256: "c226235682e64edd4f8f07eafc71de1e7dfc738a495cce47497cd9f9582ada70",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinybox-0.1.38-macos-15-x86_64.tar.gz",
            sha256: "ed066d3478015e6f063126e10d51556b0069fd6f7896513aa52dffbd75c0ad78",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinybox-0.1.38-macos-26-arm64.tar.gz",
            sha256: "f7b2448881a4af64ea3994a7c8efa83242c1da8b8e8f8c582bb9f0d4e67c636f",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinybox-0.1.38-macos-26-x86_64.tar.gz",
            sha256: "82c3058ad9685e3a88d903e4545a3a60c15769992d7f510ba5e3f3a7cedaa68d",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinybox-0.1.38-ubuntu-22.04-arm64.tar.gz",
            sha256: "b2c10d32409960f2ef9e07bcfdecbfadda3320c077dbcad521919344a790f238",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinybox-0.1.38-ubuntu-22.04-x86_64.tar.gz",
            sha256: "016caee6b9548708508c722dc57760d99099d91d2a41e533166b0f1af0147e28",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinybox-0.1.38-ubuntu-24.04-arm64.tar.gz",
            sha256: "873f91ae0b85e78a5613b44fe42d8b562b046e8ea1728c9aad1ff6842b5a699a",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinybox-0.1.38-ubuntu-24.04-x86_64.tar.gz",
            sha256: "59ff8602dede10a6e3f749aab6623a7bd653b974ed096bad762dcc76fcf6df9a",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinybox-0.1.38-windows-11-arm64.zip",
            sha256: "7bdab1bf410930267f442fe6511eb0298f05928bac47f57d27e5e749d008c898",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinybox-0.1.38-windows-2022-x86_64.zip",
            sha256: "ce2aad5f025b9e886d47696a534339a18a2dfcdc5f5a3facd6c5ff1dfa01fa49",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinybox-0.1.38-windows-2025-x86_64.zip",
            sha256: "8188eea326549f594cadad130abb54452cf0191626dfa043d3a43dfd806abeca",
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
    version: "0.1.38",
    release_url: "https://github.com/tinyhumansai/tinychannels/releases/tag/v0.1.38",
    assets: &[
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinychannels-module-0.1.38-macos-15-arm64.tar.gz",
            sha256: "1c092b81c7bde10a5d83f6f797c2dd80b9992b5c292ecee06c0c8fc866c395bd",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinychannels-module-0.1.38-macos-15-x86_64.tar.gz",
            sha256: "b0302b04b0feb2cdc4ddaf6ede9063eb727db95f4a1f07dc3140c03b8deff546",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinychannels-module-0.1.38-macos-26-arm64.tar.gz",
            sha256: "ba84151bf95ef809703c87fef6c37f598da7c492f659a10ffb9893e0e8209ffd",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinychannels-module-0.1.38-macos-26-x86_64.tar.gz",
            sha256: "421de74c0fb993ccfac3a927d4ac192d615716358f5ed72cd3ce0ab17e904ee4",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinychannels-module-0.1.38-ubuntu-22.04-arm64.tar.gz",
            sha256: "ec92e0622f2b10f6c102c5b2e808695d500eacf75c7d99c558407b085e6cee4d",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinychannels-module-0.1.38-ubuntu-22.04-x86_64.tar.gz",
            sha256: "f300153ac2cb857896550bf5e53919426f43eee684f259e5d7f761b22054f1c2",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinychannels-module-0.1.38-ubuntu-24.04-arm64.tar.gz",
            sha256: "a8e4cc8cfd5528e07ffe5e6c9963d21bfd3b124bbedab7ac6c2ecbfc3e8a2821",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinychannels-module-0.1.38-ubuntu-24.04-x86_64.tar.gz",
            sha256: "7a30f6fb0f71ef6627b246e91040a7938f94aa0519bfda93a043709aa0141e49",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinychannels-module-0.1.38-windows-11-arm64.zip",
            sha256: "26f919a77fefd8d65c7eb6a2231a35059cb2049b3b7877673073558e19d600dd",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinychannels-module-0.1.38-windows-2022-x86_64.zip",
            sha256: "ffeec695e7aad3c22dc885266badc08cf330c885425dacdbe5f35cd9ad72bfc3",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinychannels-module-0.1.38-windows-2025-x86_64.zip",
            sha256: "5ed7a3875d6e103fdd5a577e078e7288d0dfa7eb9c4b380ab4fe0e4246611aa9",
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
