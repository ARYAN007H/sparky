//! Registry records for additional first-party TinyBus modules.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

/// The `tinybox` module, loaded on demand.
#[cfg(feature = "modules")]
pub(crate) const TINYBOX: ModuleRecord = ModuleRecord {
    id: "tinybox",
    description: "Sandbox execution and lifecycle operations through TinyBox",
    bus_name: tinybox_bus::INTERFACE,
    object_path: tinybox_bus::OBJECT_PATH,
    version: "0.1.33",
    release_url: "https://github.com/tinyhumansai/tinybox/releases/tag/v0.1.33",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinybox-0.1.33-archlinux-arm64.tar.gz",
            sha256: "b3857f408972b21ad2e894820d2e4bdbcd43c380b2f4da89d633740417a72943",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinybox-0.1.33-archlinux-x86_64.tar.gz",
            sha256: "203c8fbc59b104e0f0e499909fbbe2c98355fedc01f00369e01531da3cb40eec",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinybox-0.1.33-fedora-43-arm64.tar.gz",
            sha256: "97fd2fceaea5d5d13ab244458e7226c9a7ace87cc41d2b72361b10a06aff9610",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinybox-0.1.33-fedora-43-x86_64.tar.gz",
            sha256: "61228f1c18e0729d0e5e6d1b1b8020c2e0c6378396d85f908f38a3b728a1ca76",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinybox-0.1.33-fedora-44-arm64.tar.gz",
            sha256: "11e11b442a64185c0bc21e7b67db61eff7a1654ce549a09b25111a4115b2a54f",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinybox-0.1.33-fedora-44-x86_64.tar.gz",
            sha256: "2103c93dec85979f53c81ed5f6d844eae15b638de5242aa755041fa549abe9f4",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinybox-0.1.33-macos-15-arm64.tar.gz",
            sha256: "cba18dba3dd523e95ee9a7407dc5500bd10511fefa92c49a590681118defb86a",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinybox-0.1.33-macos-15-x86_64.tar.gz",
            sha256: "8fbb3dd65bfb830b44dce35daf70edc5bc2c2148760116fef0444362ae0f1a43",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinybox-0.1.33-macos-26-arm64.tar.gz",
            sha256: "c10c583039492d0bdabb2d05debe20c9c948011cbd7532450c31e1e5aac6eb6e",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinybox-0.1.33-macos-26-x86_64.tar.gz",
            sha256: "663172672604b089474175e80ed3866bc3e69a77bac40b4a75742e37bfc4e4b0",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinybox-0.1.33-ubuntu-22.04-arm64.tar.gz",
            sha256: "d9ed99f3f4b329d101dc34c3b30e39114b0ef60318a0f70b3d0091c4b7149fb6",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinybox-0.1.33-ubuntu-22.04-x86_64.tar.gz",
            sha256: "c34b6c5f70b6d3b207d4b3937bca0c0411c5e865a55edbe029ec295d1c5c0ebe",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinybox-0.1.33-ubuntu-24.04-arm64.tar.gz",
            sha256: "aa2e7c7870ed4c76606a7859dc3e56f97c8fd3765a8536b878d9533aa9f87c4e",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinybox-0.1.33-ubuntu-24.04-x86_64.tar.gz",
            sha256: "257047ba8a3e49274828266322f3994b8635d6a2dba3e54ebd79d308077e2300",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinybox-0.1.33-windows-11-arm64.zip",
            sha256: "309d0241af813128c95677ae9e550b68e9aab7ec6b88c4ee68ea091c92d63da0",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinybox-0.1.33-windows-2022-x86_64.zip",
            sha256: "b7548d3243c1b57b5abdb2aa24fbc91ac8883b89a4622b29f0e3eb0785062748",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinybox-0.1.33-windows-2025-x86_64.zip",
            sha256: "5ac9710973aac69a5b95e607f0b82689a9c89c5a03c19be6748ffbfd8ef10744",
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
    version: "0.1.34",
    release_url: "https://github.com/tinyhumansai/tinychannels/releases/tag/v0.1.34",
    assets: &[
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinychannels-module-0.1.34-macos-15-arm64.tar.gz",
            sha256: "b58f68ac45af467fa9eea0443afa59ed17a2f4e7ab9c3f4223ded6d83af88b22",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinychannels-module-0.1.34-macos-15-x86_64.tar.gz",
            sha256: "ec04b49b184388b514ffc4aad9e130141987bd28d9cc5f4a76daf87c25e28a98",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinychannels-module-0.1.34-macos-26-arm64.tar.gz",
            sha256: "7b4f3be24e83a056e68d072cc1a5e00420d18f30eef0cd767cb6be40d0b2ad95",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinychannels-module-0.1.34-macos-26-x86_64.tar.gz",
            sha256: "08e3d4ad69b30c46b06f4846133bb8066396e95f8f161f98c7d20d375b50453c",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinychannels-module-0.1.34-ubuntu-22.04-arm64.tar.gz",
            sha256: "035d782536952a7e60d3ba9011ee96cca71e63d515ef78e7397d4800ecf3aece",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinychannels-module-0.1.34-ubuntu-22.04-x86_64.tar.gz",
            sha256: "98292be5f921c027a79e41b463ffc9981fca139e9c269bb8daf882392c97817c",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinychannels-module-0.1.34-ubuntu-24.04-arm64.tar.gz",
            sha256: "57af460080ecdd14970e47dc02d8f27334cf165ab70ba3fb82e34dc88e445329",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinychannels-module-0.1.34-ubuntu-24.04-x86_64.tar.gz",
            sha256: "e153a65569f3e7aba90f78c3fe6ada4684280ea02776d5263a9df19071335fc7",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinychannels-module-0.1.34-windows-11-arm64.zip",
            sha256: "ef163bcd82fcdeafbb364f1f287eda0e9778255e04fb4062534ccf365f87bdeb",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinychannels-module-0.1.34-windows-2022-x86_64.zip",
            sha256: "a570821ef804e71ad98c747e51a22b8631a7d3db760e99b240cf666fafe67ced",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinychannels-module-0.1.34-windows-2025-x86_64.zip",
            sha256: "95bbf5aca663af06a6e6b912fa519e249fd1f454c65c0bb2993d1e1a79973316",
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
