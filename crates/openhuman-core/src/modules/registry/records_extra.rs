//! Registry records for additional first-party TinyBus modules.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

/// The `tinybox` module, loaded on demand.
#[cfg(feature = "modules")]
pub(crate) const TINYBOX: ModuleRecord = ModuleRecord {
    id: "tinybox",
    description: "Sandbox execution and lifecycle operations through TinyBox",
    bus_name: tinybox_bus::INTERFACE,
    object_path: tinybox_bus::OBJECT_PATH,
    version: "0.2.0",
    release_url: "https://github.com/tinyhumansai/tinybox/releases/tag/v0.2.0",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinybox-0.2.0-archlinux-arm64.tar.gz",
            sha256: "ce07fe53911d05e80fd691fae5f20572b7e33df38330127c4228903b6e22c295",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinybox-0.2.0-archlinux-x86_64.tar.gz",
            sha256: "fe576bc032666c2e7029887c0f1cfd07bd6658190a9f66bc9aa825a351370018",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinybox-0.2.0-fedora-43-arm64.tar.gz",
            sha256: "04f739e5d606f3b30486d43d803c97d56d60a748977c2dc8301372e3873d7e9c",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinybox-0.2.0-fedora-43-x86_64.tar.gz",
            sha256: "42e37e4297c1e8210ae4448b43bc73c42e1f4be2e9a599052033280b72f2c425",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinybox-0.2.0-fedora-44-arm64.tar.gz",
            sha256: "537c9697c48f6662450471ea81f3c54724038eb8c8bb7b223e73297832a3711e",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinybox-0.2.0-fedora-44-x86_64.tar.gz",
            sha256: "fbeed2edc4fb015ecd84d2a6e027e4ee86d190576b4314e4d8519865f60232b5",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinybox-0.2.0-macos-15-arm64.tar.gz",
            sha256: "6a17d9a6d84110a7e8d555181def8196351f15ce4dd61ff912bf9b7edd854b3c",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinybox-0.2.0-macos-15-x86_64.tar.gz",
            sha256: "9be3fb561145581dd95f7ad974f53165bbcdb7d773149bb4ab715db152f82225",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinybox-0.2.0-macos-26-arm64.tar.gz",
            sha256: "6458fb8491e538cd145682289f883523c2bc3660d90c97995454424ff1c6b533",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinybox-0.2.0-macos-26-x86_64.tar.gz",
            sha256: "b5375543c6892d46af74af0b177b36da764f353726b7c1a253df165de3f772a1",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinybox-0.2.0-ubuntu-22.04-arm64.tar.gz",
            sha256: "bb8b8855ff6e470ed8dacd67412aeefd43a00cd52d4416baa76b1868e228a024",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinybox-0.2.0-ubuntu-22.04-x86_64.tar.gz",
            sha256: "dcac85d16068258fd66eb655e05929325f917deba4e318a1f069bd31ce2248d3",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinybox-0.2.0-ubuntu-24.04-arm64.tar.gz",
            sha256: "913777de0ebca4afec6865c9a9a6c5c6ed16ca9a989e0d2b0d7cdae20491263b",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinybox-0.2.0-ubuntu-24.04-x86_64.tar.gz",
            sha256: "9ab157e04471e966711dea110e0d079050afb0d2a462a6a761828241893ac8b4",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinybox-0.2.0-windows-11-arm64.zip",
            sha256: "f4e07ee9e405542c52de457b3df05de18ccf0ce3afed16bd4ef011e41713e97c",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinybox-0.2.0-windows-2022-x86_64.zip",
            sha256: "fee04c70ea82459d3f24849755ca6cbfb431af090b0d650eb546a465cee5841b",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinybox-0.2.0-windows-2025-x86_64.zip",
            sha256: "0ed055f46352e8a6de3145a3fdbaa7e33da5e4eaee5a9614ce49c4f08ebea2b0",
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
    version: "0.1.39",
    release_url: "https://github.com/tinyhumansai/tinychannels/releases/tag/v0.1.39",
    assets: &[
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinychannels-module-0.1.39-macos-15-arm64.tar.gz",
            sha256: "2a360880d72239bb370fd25c50f384f7c07a6a349d2da7854645b88fe2bc567f",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinychannels-module-0.1.39-macos-15-x86_64.tar.gz",
            sha256: "672c0aa2a34954c0445fdcf532f4507468288f045925950a4c5cd726a34a16b8",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinychannels-module-0.1.39-macos-26-arm64.tar.gz",
            sha256: "2c89dee2d3b99f6879ede752b29ffce2532ac7b4c37f0d1cd9f51693d97ca807",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinychannels-module-0.1.39-macos-26-x86_64.tar.gz",
            sha256: "725b3b02bf312523940551b8456a043a6b0fec56c4738ee751776206438d2b76",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinychannels-module-0.1.39-ubuntu-22.04-arm64.tar.gz",
            sha256: "a0356ad855de475cdb56bb60a501a75adceb07a860ac06c15ce4aa57dbc35ab2",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinychannels-module-0.1.39-ubuntu-22.04-x86_64.tar.gz",
            sha256: "d14e379f625ed1dad0f2f0d5ca6e76d57caa2c678a929f11de096a55df8ebbbc",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinychannels-module-0.1.39-ubuntu-24.04-arm64.tar.gz",
            sha256: "89dac4db8429b0eb3a99a5edc8fd56e1edeef76ffcfc996aa073810a05c6b5a4",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinychannels-module-0.1.39-ubuntu-24.04-x86_64.tar.gz",
            sha256: "04c53b7649b1f0323fc4af0bed6f7518c1feadbf570262c5a96a0c2879f51287",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinychannels-module-0.1.39-windows-11-arm64.zip",
            sha256: "e9da5f08300cf40e1f23f7bea1abb5322b69425178e0b59df6eaa3e75e16aed5",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinychannels-module-0.1.39-windows-2022-x86_64.zip",
            sha256: "b4fb4dfb2019593ddd1dc336d660bd3926ba39dc2e0e8849dc8c06053acea53b",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinychannels-module-0.1.39-windows-2025-x86_64.zip",
            sha256: "4adefa0e8589a52f2cd92d5839c78b52476f03734a6f7f534283f2b6b2ad436c",
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
