//! Registry records for additional first-party TinyBus modules.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

/// The `tinybox` module, loaded on demand.
#[cfg(feature = "modules")]
pub(crate) const TINYBOX: ModuleRecord = ModuleRecord {
    id: "tinybox",
    description: "Sandbox execution and lifecycle operations through TinyBox",
    bus_name: tinybox_bus::INTERFACE,
    object_path: tinybox_bus::OBJECT_PATH,
    version: "0.2.2",
    release_url: "https://github.com/tinyhumansai/tinybox/releases/tag/v0.2.2",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinybox-0.2.2-archlinux-arm64.tar.gz",
            sha256: "1ffb8939dd2d4863743af1c141fd57c5b7a40fef23ef76986cc9e78bf10f59db",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinybox-0.2.2-archlinux-x86_64.tar.gz",
            sha256: "f905754bc510090cf8cf62f407e4a11d39b3dd4c854033d51ceda988b40fb0d0",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinybox-0.2.2-fedora-43-arm64.tar.gz",
            sha256: "9d4af8477c4c4fbdd9a5f07ce7c0af7dcc680da742807f2251c1ec9e1e52422e",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinybox-0.2.2-fedora-43-x86_64.tar.gz",
            sha256: "4d159a01d9fe4d48bdd06443a4c179cc213410a2f659a55432661c1552e381b8",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinybox-0.2.2-fedora-44-arm64.tar.gz",
            sha256: "a152c2168b279dc5e34b2538134715e14e820f205ac2ad17d9bc6d1fce1cd6ba",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinybox-0.2.2-fedora-44-x86_64.tar.gz",
            sha256: "89e074b41946396cbf08587290ae3936c9ba931a7f598ff64f531428ae0f65a9",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinybox-0.2.2-macos-15-arm64.tar.gz",
            sha256: "0db00ded02f00bc90b1b247560aee7bc3899d33f34ee3d01447e7d48e7c6d170",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinybox-0.2.2-macos-15-x86_64.tar.gz",
            sha256: "af8d4c3ec05f2d87bf1cfb19e69f98e15cb47c50b62391d502dfb236a9074291",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinybox-0.2.2-macos-26-arm64.tar.gz",
            sha256: "281aed7470018142a8f5605a7d495599426178594fcb2fcc06907ff478ef4606",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinybox-0.2.2-macos-26-x86_64.tar.gz",
            sha256: "8048fa03a2e0b1858ab783dc10271773180f14d4ba915e68f29463abed3a5a75",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinybox-0.2.2-ubuntu-22.04-arm64.tar.gz",
            sha256: "7da76007f7c61572035401e6ca96aa57884bcf69c78a5dc0fc1d44cffbea7a79",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinybox-0.2.2-ubuntu-22.04-x86_64.tar.gz",
            sha256: "4ea0af27c33b0fa21cafd4b304fb9f5137ac023ed96c9204eb008849e42f6afa",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinybox-0.2.2-ubuntu-24.04-arm64.tar.gz",
            sha256: "6ff35191084f21a6d4aca29ad41a21c4f9325ea4a50244def13afbb5fc46ae1d",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinybox-0.2.2-ubuntu-24.04-x86_64.tar.gz",
            sha256: "e52934c1bdd541bd0b23b565b930dba17fbd1cc112513f7dfa6ded83921031b6",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinybox-0.2.2-windows-11-arm64.zip",
            sha256: "c3118be2e00d3823b9e79337ea8d3628db51564781c1839547490d0efb36f899",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinybox-0.2.2-windows-2022-x86_64.zip",
            sha256: "cf3d75b5386b9e1924dca7bc46aaec33a59ec1baefe60d7eeacc94d4d8b61f3e",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinybox-0.2.2-windows-2025-x86_64.zip",
            sha256: "7dc94f4f4f33582fc4b7efce4fadfb1870ea8902b3f09b5525c20233145dddf2",
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
    version: "0.1.43",
    release_url: "https://github.com/tinyhumansai/tinychannels/releases/tag/v0.1.43",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinychannels-module-0.1.43-archlinux-arm64.tar.gz",
            sha256: "4b9d759be6ded94e07884ea4cd5a92ec12211b6fd274ff958b24b816af43d3a5",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinychannels-module-0.1.43-archlinux-x86_64.tar.gz",
            sha256: "b68bf10234ed7b3bb5b2ae9f57ceb3377e65facca47450c2c3542094f0a7f883",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinychannels-module-0.1.43-fedora-43-arm64.tar.gz",
            sha256: "a9f331f5768dad8370e63002084efc1bef2f1e6c3bcbe8b063e8df8a1519e09a",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinychannels-module-0.1.43-fedora-43-x86_64.tar.gz",
            sha256: "ef807b7c271bc5c35ac9a91029275d4511f31d13c053bb5999413aff542a4a77",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinychannels-module-0.1.43-fedora-44-arm64.tar.gz",
            sha256: "fa37cde5a84986babde4916a61b5ed38380a7d68ad08fb0a519d24195a5d8886",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinychannels-module-0.1.43-fedora-44-x86_64.tar.gz",
            sha256: "c6d2acd0334ccffb465bc420bc558ed30edbbb8f4d6d03d8b5409ca1ecce5388",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinychannels-module-0.1.43-macos-15-arm64.tar.gz",
            sha256: "57748164f05e662d4cc6b01d52a1dfd1524fb114e85c382dd9c067cdbef098b5",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinychannels-module-0.1.43-macos-15-x86_64.tar.gz",
            sha256: "850c369dfe6001dfc8d76b4f1f39866c62e4b74d5e2a03eeb4ce90a035a33e15",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinychannels-module-0.1.43-macos-26-arm64.tar.gz",
            sha256: "542a209ed3f1fff22811a2f8e1150e44471e28350da40613573ca548f3a8ddbf",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinychannels-module-0.1.43-macos-26-x86_64.tar.gz",
            sha256: "4be169ce377ffd8b30761b473f9ea9cd6574769c9720934f6ae1b268963a7a47",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinychannels-module-0.1.43-ubuntu-22.04-arm64.tar.gz",
            sha256: "3025c71abf61c2a339898563a36d1a7280b15d2a44dcdca27551594751197fda",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinychannels-module-0.1.43-ubuntu-22.04-x86_64.tar.gz",
            sha256: "98ae46d1b6e7869bf7e569ddd26c4b1acc488679593a475435d43db00d8e391f",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinychannels-module-0.1.43-ubuntu-24.04-arm64.tar.gz",
            sha256: "e1f1c77712467bee8397c7cb9445f7e3c8a138998405955c64407cb03e68eb6b",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinychannels-module-0.1.43-ubuntu-24.04-x86_64.tar.gz",
            sha256: "da2d6283331e35e485779ca1a7f6d62b4f84fef9a8cc151f5b76dd508fa565e3",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinychannels-module-0.1.43-windows-11-arm64.zip",
            sha256: "e6edde84f041182775a1549de4d370aad5422055a0ef95363f741d2bea0d009e",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinychannels-module-0.1.43-windows-2022-x86_64.zip",
            sha256: "a4bbef8b92a4cd3318e2fd70f250334fba3fe4ff203ee1add104973c60181062",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinychannels-module-0.1.43-windows-2025-x86_64.zip",
            sha256: "77155d6f38a2b60d17f316dc03d43393b4c94349ce81d1bd06c033d258a75bba",
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
    version: "0.4.4",
    release_url: "https://github.com/tinyhumansai/tinyhosts/releases/tag/v0.4.4",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinyhosts-0.4.4-archlinux-arm64.tar.gz",
            sha256: "a4426923a360bbf2726d3ec41b9a6c3d356089175a139d21062d2eb0f951cf2f",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinyhosts-0.4.4-archlinux-x86_64.tar.gz",
            sha256: "ee21c6d3b8fc3a5e1c6c0c7936e9f762bac7d31f9f7bad5c9c1d142a93c7bfe6",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinyhosts-0.4.4-fedora-43-arm64.tar.gz",
            sha256: "17d62a5edafb7d5a0744c176e92643bfc1942532b7983408e0f5c9db970c8d62",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinyhosts-0.4.4-fedora-43-x86_64.tar.gz",
            sha256: "def0543fd308d2bcfe26a01840ffc87b1e83a0217a20c126b0569afb9f978807",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinyhosts-0.4.4-fedora-44-arm64.tar.gz",
            sha256: "6fa28057eca0d549bf39b204e527b6ac921f52567637a44bb81aa6e1832dbb86",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinyhosts-0.4.4-fedora-44-x86_64.tar.gz",
            sha256: "69f94ffa4cc1206ded2dded8e6fbac0eea583f54cc9ea2006b922762ecb7a512",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyhosts-0.4.4-macos-15-arm64.tar.gz",
            sha256: "7b79cdaee9d8956aeb3b9fd660198d246f672926f80b1be60418966b084d2363",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyhosts-0.4.4-macos-15-x86_64.tar.gz",
            sha256: "53acf40bcf1d1fbbf5be937ca98a60d2b8e4755accf8272e304e0c5f7bb17c40",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyhosts-0.4.4-macos-26-arm64.tar.gz",
            sha256: "162f69d470e7fc70ea756b8f67dde0f42a2e0936cc961b3ff28235b368ccbf9e",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyhosts-0.4.4-macos-26-x86_64.tar.gz",
            sha256: "8235b95e1398285b2147fc11c22993ce5aefbe848695473e62696ae93e42e394",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyhosts-0.4.4-ubuntu-22.04-arm64.tar.gz",
            sha256: "3b7dc11a4a66bbcc2e2912e36e946f8ee1d921ac44579a7dad6cf01751fa494f",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyhosts-0.4.4-ubuntu-22.04-x86_64.tar.gz",
            sha256: "1d486fbc6a52510180e58b5aabae620ead7f1da92060112f87aae2fc845c8806",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyhosts-0.4.4-ubuntu-24.04-arm64.tar.gz",
            sha256: "e232bd43b25614486de18d6777fb19f3af8b29cda2cdd1354975c67589d5aacf",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyhosts-0.4.4-ubuntu-24.04-x86_64.tar.gz",
            sha256: "72c5ead7301a863e78e9eaf5a47943923566907a759fe62d2f81e7118c50eaa0",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyhosts-0.4.4-windows-11-arm64.zip",
            sha256: "dbf65473a44193d7cb7059be27982fc1dcc63f5e990d91c9d3d0bd901e52edd6",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyhosts-0.4.4-windows-2022-x86_64.zip",
            sha256: "63d118c5a69b6e0c074ec3dac866bde5784fecb5794c78b0ef4c1a2f4ec75967",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyhosts-0.4.4-windows-2025-x86_64.zip",
            sha256: "b4deb9b67bdbf3fdbb0d09cb4867ebb8caa3222d37663a4d7f1c1af7e96b9f69",
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
