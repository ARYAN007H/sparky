//! Registry records for additional first-party TinyBus modules.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

/// The `tinybox` module, loaded on demand.
#[cfg(feature = "modules")]
pub(crate) const TINYBOX: ModuleRecord = ModuleRecord {
    id: "tinybox",
    description: "Sandbox execution and lifecycle operations through TinyBox",
    bus_name: tinybox_bus::INTERFACE,
    object_path: tinybox_bus::OBJECT_PATH,
    version: "0.1.40",
    release_url: "https://github.com/tinyhumansai/tinybox/releases/tag/v0.1.40",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinybox-0.1.40-archlinux-arm64.tar.gz",
            sha256: "303883247b457c47d787efe77fdf9378d32a7752b5891bb8f8590c8919c9310b",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinybox-0.1.40-archlinux-x86_64.tar.gz",
            sha256: "4b2aeb9fc91c4ed6fd8d27fb1ee6666a90cfde31108dc8012073fe4b0d1bf8b1",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinybox-0.1.40-fedora-43-arm64.tar.gz",
            sha256: "c13130d96a159b29316f0c41f63c25b3b0de23b0476f7e988219f121c28c96ee",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinybox-0.1.40-fedora-43-x86_64.tar.gz",
            sha256: "1685d8efd5267a56563f6f3e1e3fe3f7d43ec85d29b218dc65ae923df45e826d",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinybox-0.1.40-fedora-44-arm64.tar.gz",
            sha256: "0c03e21248ba303baf6ee83410b77fd366bea0f66028d2fb54983e1ef2a4fbfa",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinybox-0.1.40-fedora-44-x86_64.tar.gz",
            sha256: "4def3f78e3963b33a937cd47c89d8940af8ada9b037b9dc040ad4cf96d81c0dd",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinybox-0.1.40-macos-15-arm64.tar.gz",
            sha256: "aae2e9d0e3df8f8253c5a3ef3a0b9fb1d224e0a4ee689b5bdfecd8ac1f192bec",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinybox-0.1.40-macos-15-x86_64.tar.gz",
            sha256: "0800d426346160ec84b61a47b8c339321cbfcded8805cdac96ee53f763df0646",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinybox-0.1.40-macos-26-arm64.tar.gz",
            sha256: "078348a5263cc13e58cbdef41c82b12a7b94781f7f15aae84886a428445d301c",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinybox-0.1.40-macos-26-x86_64.tar.gz",
            sha256: "7240c5c369bd20dc6466ec42d9104bf14d59225fac55feb4eed684885ce3efb9",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinybox-0.1.40-ubuntu-22.04-arm64.tar.gz",
            sha256: "456498d248a748200a2372c0dc75a6880789b4beec437916b921e10ab982f751",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinybox-0.1.40-ubuntu-22.04-x86_64.tar.gz",
            sha256: "ade2273ca24573831b38a7513aec89e98f7e1bf56ed16759d87e11aa6813a0e9",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinybox-0.1.40-ubuntu-24.04-arm64.tar.gz",
            sha256: "4fbc9399be13f95f3d5ce9dea9d966fcab306324e422c081281b0c16f0f7090c",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinybox-0.1.40-ubuntu-24.04-x86_64.tar.gz",
            sha256: "16c2aa1d293bc09fe882714f380d605d80537936956e3a61e9987333abe5cbe1",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinybox-0.1.40-windows-11-arm64.zip",
            sha256: "73307f0b6a7a6d6978f6e9ecd9092a957f64e2daa4b03ea3a9170207f5212f37",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinybox-0.1.40-windows-2022-x86_64.zip",
            sha256: "688f4184225cf392cdfa00f4cc7ef8ca5e1d2b38bbf4ebc3a1f63ea133b48007",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinybox-0.1.40-windows-2025-x86_64.zip",
            sha256: "7ac5140fdde15208a61d52c1bc8d9ba3f610337f6964e64fd53303c4f68bf5cb",
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
