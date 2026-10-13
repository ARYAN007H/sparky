//! Registry record for the `tinyjuice` module.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

pub(crate) const TINYJUICE: ModuleRecord = ModuleRecord {
    id: "tinyjuice",
    description: "Content-aware tool-output compression and recoverable caching",
    bus_name: tinyjuice_bus::names::BUS_NAME,
    object_path: tinyjuice_bus::names::OBJECT_PATH,
    version: "0.10.1",
    release_url: "https://github.com/tinyhumansai/tinyjuice/releases/tag/v0.10.1",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinyjuice-module-0.10.1-archlinux-arm64.tar.gz",
            sha256: "e45ada17b48a863ce24a30e633c9cc6448f2403cbb70bf1bc18a3a175e54dd43",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinyjuice-module-0.10.1-archlinux-x86_64.tar.gz",
            sha256: "d18a404ed2edd387c0b4b5c117abf5ec366ce65a9e6bb06f75a2745ea254821f",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinyjuice-module-0.10.1-fedora-43-arm64.tar.gz",
            sha256: "b01f41653a1bd53934896e13d79669f82c19609ad4e51b77a208939f4b71eca5",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinyjuice-module-0.10.1-fedora-43-x86_64.tar.gz",
            sha256: "f86918384bdf01c67bed259c6330e3067b78d36efdd16cb124adeb6a75ea40d7",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinyjuice-module-0.10.1-fedora-44-arm64.tar.gz",
            sha256: "c7f3ecba603c137c01b28a390b43ca3ebcc3744567c275458dbe537c14378aa0",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinyjuice-module-0.10.1-fedora-44-x86_64.tar.gz",
            sha256: "dc69a26c73b0ea326275dda05af5157563e30b5eb42fe0f0c9946082568130cc",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyjuice-module-0.10.1-macos-15-arm64.tar.gz",
            sha256: "e0be4697e9efae8fd8899cf793cfe222cb5d8437ae55e3369ebc26b7792fbb0a",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyjuice-module-0.10.1-macos-15-x86_64.tar.gz",
            sha256: "c4dd084574cd44b0d7fea8bff14693d9471766f86a53ac13ea32b54a8a75037d",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyjuice-module-0.10.1-macos-26-arm64.tar.gz",
            sha256: "19d4fa582294ee516679d240bc3782a173a240216a4f8031126a3f00ddaad2c9",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyjuice-module-0.10.1-macos-26-x86_64.tar.gz",
            sha256: "31d7f78bd907eb686371820b04ac4f83f749ca43a562ec9e44a9533e4b7037b1",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyjuice-module-0.10.1-ubuntu-22.04-arm64.tar.gz",
            sha256: "e79c3207917a90435ee4065703670b6f16c4de2197fa56af2bab2c584aca5975",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyjuice-module-0.10.1-ubuntu-22.04-x86_64.tar.gz",
            sha256: "0bcfc7d50a3f91e46f53451ec20f5b62e4875d93815029260d041f4ac1344e05",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyjuice-module-0.10.1-ubuntu-24.04-arm64.tar.gz",
            sha256: "43a8cbfe69f0c1bd489057a4b96f7d04307b8c34ba15e2a221b1949c40e100b5",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyjuice-module-0.10.1-ubuntu-24.04-x86_64.tar.gz",
            sha256: "f12a78e0b26f91cc462ec4b30a8d9f5b200d3a0c5601e49b7079b66296df9c9b",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyjuice-module-0.10.1-windows-11-arm64.zip",
            sha256: "3c9d55c24b3e8b85872eba3c4c74312ca496ea2257310185959722b032439e98",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyjuice-module-0.10.1-windows-2022-x86_64.zip",
            sha256: "65c59298e3a8f753233ba802d2867395ca27ec911662457d5acab2e7c03cada6",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyjuice-module-0.10.1-windows-2025-x86_64.zip",
            sha256: "f4dc7c05b90b33a30f27673bd47c077606ceb9138a7e052863becdb3cab9fe71",
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
