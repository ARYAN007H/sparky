//! Registry record for the `tinyjuice` module.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

pub(crate) const TINYJUICE: ModuleRecord = ModuleRecord {
    id: "tinyjuice",
    description: "Content-aware tool-output compression and recoverable caching",
    bus_name: tinyjuice_bus::names::BUS_NAME,
    object_path: tinyjuice_bus::names::OBJECT_PATH,
    version: "0.10.2",
    release_url: "https://github.com/tinyhumansai/tinyjuice/releases/tag/v0.10.2",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinyjuice-module-0.10.2-archlinux-arm64.tar.gz",
            sha256: "4883ab560dfa94507bbea3d4fbaf76e51bce9dc79466a35e3ce11358988fa7bd",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinyjuice-module-0.10.2-archlinux-x86_64.tar.gz",
            sha256: "c23a5a3138677260fbf07149cd84990aa37bc90fb83a714d1afefc7762b3f94f",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinyjuice-module-0.10.2-fedora-43-arm64.tar.gz",
            sha256: "0a3b8a011848591f0c29c57fefbc8b33c6296ea427b46800e52593a1abd0194d",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinyjuice-module-0.10.2-fedora-43-x86_64.tar.gz",
            sha256: "167999c38c7bb8562a40d45138a7a3f30e1964c48d835f9b9c996dd9838e9dd7",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinyjuice-module-0.10.2-fedora-44-arm64.tar.gz",
            sha256: "f2dea63ef974ee676b46e224c94ba9ea0a50c4462c72eb2329e97d4d6403964f",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinyjuice-module-0.10.2-fedora-44-x86_64.tar.gz",
            sha256: "af60373206d89af0ee018152e9332f27275ca7e879742dfb5a4d6020e7b8ce00",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyjuice-module-0.10.2-macos-15-arm64.tar.gz",
            sha256: "8010fc6afa11e7937d58165d168d4d27bcd6b322d8de18a7f10fe47ac5bea206",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyjuice-module-0.10.2-macos-15-x86_64.tar.gz",
            sha256: "674c4276f67a7e60453786b55295d4241a838b35e78319c3e719230ee1a2ba2f",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyjuice-module-0.10.2-macos-26-arm64.tar.gz",
            sha256: "942857f2ff2099e5376dd27181b4415564ce7f51ddd8071df62d3eef5fb92013",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyjuice-module-0.10.2-macos-26-x86_64.tar.gz",
            sha256: "831b35a7bc96e0851b9e32d610fc69beb05b02fe6b2115ff0fdc09efb2332256",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyjuice-module-0.10.2-ubuntu-22.04-arm64.tar.gz",
            sha256: "5b3f428fa443a66cfb55fdd2e72f2670e3dded6d92f678cc1e08d20e4401e83a",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyjuice-module-0.10.2-ubuntu-22.04-x86_64.tar.gz",
            sha256: "60b78a362552d7c01051d896c8e3ecfc3197e0595afef78986d9301dd6dc5e3f",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyjuice-module-0.10.2-ubuntu-24.04-arm64.tar.gz",
            sha256: "012340e225a4ab8203d6f850833340af7790baa473ffa7ff8f88d4108ac5aa39",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyjuice-module-0.10.2-ubuntu-24.04-x86_64.tar.gz",
            sha256: "86ab6bb9d14245a57225d419d18612907e3d48eacd7eac978a2898bbf185a2e7",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyjuice-module-0.10.2-windows-11-arm64.zip",
            sha256: "71e03004ed08e95df4e73e0be94f2666adc231bf188df88f8ebd4405ac47d9fb",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyjuice-module-0.10.2-windows-2022-x86_64.zip",
            sha256: "a74b8fcbbf91bdd0d196d849018d9cefbeeef0f013b5499d634e4a4d4f5a92d0",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyjuice-module-0.10.2-windows-2025-x86_64.zip",
            sha256: "3e74a3b2ff4cf8ec62ae1c9912d1cf523fec09abbf1a69c526385423af11012b",
        },
    ],
    load: LoadPolicy::Lazy,
    source: SourceSpec {
        submodule: "tinyjuice",
        manifest: "crates/tinyjuice-module/Cargo.toml",
        package: "tinyjuice-module",
        lib_name: "tinyjuice_module",
        features: &[],
    },
};
