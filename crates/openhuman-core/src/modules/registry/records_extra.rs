//! Registry records for additional first-party TinyBus modules.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

/// The `tinybox` module, loaded on demand.
#[cfg(feature = "modules")]
pub(crate) const TINYBOX: ModuleRecord = ModuleRecord {
    id: "tinybox",
    description: "Sandbox execution and lifecycle operations through TinyBox",
    bus_name: tinybox_bus::INTERFACE,
    object_path: tinybox_bus::OBJECT_PATH,
    version: "0.2.1",
    release_url: "https://github.com/tinyhumansai/tinybox/releases/tag/v0.2.1",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinybox-0.2.1-archlinux-arm64.tar.gz",
            sha256: "ff2404f24840632a00d8107d02ea4ac258cc56b77e650653b6c440e769da282a",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinybox-0.2.1-archlinux-x86_64.tar.gz",
            sha256: "3b8f14a175fb0e73578047192f5aeefe3f76014d5a1baf3fb85d63f300ee48fc",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinybox-0.2.1-fedora-43-arm64.tar.gz",
            sha256: "8b53c7f60cfddfadda3f0269aee15676069d727c1fc8c10f7aa8211baa55292a",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinybox-0.2.1-fedora-43-x86_64.tar.gz",
            sha256: "aaafcd8b33a844e44362f7566d4c384c6a541abfd5888b0e415158c8d5bc0ea4",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinybox-0.2.1-fedora-44-arm64.tar.gz",
            sha256: "5441fda12064b029e745229e0c9fd2fd64dcc8e30f3416f5cdbe877583524295",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinybox-0.2.1-fedora-44-x86_64.tar.gz",
            sha256: "1d48b6b734bba68dc3160b85f0cb9e8f7e96e36acab18afd30f71a20e002df2b",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinybox-0.2.1-macos-15-arm64.tar.gz",
            sha256: "8d59ecb80220913770a25ab32116c6ff8e55cdb6fac1db634eab3b8767e22690",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinybox-0.2.1-macos-15-x86_64.tar.gz",
            sha256: "4c1f22592913ca5c57a840cca65a638493cb76b95cee4d88b7367d762a5afa01",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinybox-0.2.1-macos-26-arm64.tar.gz",
            sha256: "daaec8cbb2a8060783c53621794ae6c03b62c9f5295753c5040fe0bbdf75b98e",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinybox-0.2.1-macos-26-x86_64.tar.gz",
            sha256: "de5c07812e54b5ae421bd28a0071566c36d124c07f87dde2eef6e0df83745350",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinybox-0.2.1-ubuntu-22.04-arm64.tar.gz",
            sha256: "488082b3619698a82dfe14d13547ffed90aabc9278f64384f39e7b2f3b8ec020",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinybox-0.2.1-ubuntu-22.04-x86_64.tar.gz",
            sha256: "d6dc0320b2944ebc3d1972d80d4a47e459c5ebb24d0c41ef0f64e5f442f90685",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinybox-0.2.1-ubuntu-24.04-arm64.tar.gz",
            sha256: "ff1f9e0b9658c463a4b8f1e6aeb36cc7f9bc8b97117e8d17fd8986245a8e5b97",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinybox-0.2.1-ubuntu-24.04-x86_64.tar.gz",
            sha256: "87f9017e0384babe52a43cc9002a23d13558cdbab2d1807caf6d48152684871a",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinybox-0.2.1-windows-11-arm64.zip",
            sha256: "86ab6d28269b9e7eb59b54f6534fdc3e9bff67ebfcf66311a0ce6fdd16eaf1d1",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinybox-0.2.1-windows-2022-x86_64.zip",
            sha256: "68b7089dd4141434d3dfc91b32fae688cae549a297d0a933797a3b2b43e92795",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinybox-0.2.1-windows-2025-x86_64.zip",
            sha256: "059e7fbc993627d866c6b57d59286fe80eefc4ebe6542d4535d0e0f7d08e22a9",
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
    version: "0.4.3",
    release_url: "https://github.com/tinyhumansai/tinyhosts/releases/tag/v0.4.3",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinyhosts-0.4.3-archlinux-arm64.tar.gz",
            sha256: "97388fd17b72b92f0384397378fece2e7d0806004676edcdfc2b1798669b67a8",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinyhosts-0.4.3-archlinux-x86_64.tar.gz",
            sha256: "93e0500e30b77aca6d05de69636c4afc014a3856809e3f781868635d626ef9c0",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinyhosts-0.4.3-fedora-43-arm64.tar.gz",
            sha256: "fef318d960bbaea005a2f1f8bd1c64ff0bce09a3e4cef4922e7a75019dadd29f",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinyhosts-0.4.3-fedora-43-x86_64.tar.gz",
            sha256: "cef9120fc178bf5b40f06e23727d47901746952aab4129eecbc3babd2f516421",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinyhosts-0.4.3-fedora-44-arm64.tar.gz",
            sha256: "be1a4c1c2da9e8745fd18a3d5c050cc99fc6655f6eba2e914e191086e7b0ec6d",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinyhosts-0.4.3-fedora-44-x86_64.tar.gz",
            sha256: "ed5a96104a1db21f3852e0d3824cb493c5091318d36f44de97122abc8f7969ef",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyhosts-0.4.3-macos-15-arm64.tar.gz",
            sha256: "79a3fa3c643f50d7e46fec58bfbd3117590853ab76792175b62b4325846377cf",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyhosts-0.4.3-macos-15-x86_64.tar.gz",
            sha256: "4ac3e43e3d48210f40361b199760df738ed3ab01c248f2d45495a8e4c1d50509",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyhosts-0.4.3-macos-26-arm64.tar.gz",
            sha256: "efff57af745179807ef8eb41a1c83df6cce1cde23ae664ac8e2e67065dae06b7",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyhosts-0.4.3-macos-26-x86_64.tar.gz",
            sha256: "6c7f70714c7d2246436487ce986c58e1962a393d1d9f3db6d1634c7ce3f75dad",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyhosts-0.4.3-ubuntu-22.04-arm64.tar.gz",
            sha256: "d554e1a524040b0ae023ff6aec0c0129d9e01526077ca5d6ba1249ce5cd440f0",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyhosts-0.4.3-ubuntu-22.04-x86_64.tar.gz",
            sha256: "c6d7cf38c8b3e062e1a8af7785cf8c902c6e7d2d1be51a502407537369210b50",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyhosts-0.4.3-ubuntu-24.04-arm64.tar.gz",
            sha256: "9eec6afb328b22dad77e96ca802c39a83656f0d57faea9c2d2d2017547e03c0b",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyhosts-0.4.3-ubuntu-24.04-x86_64.tar.gz",
            sha256: "70da542ec03e90485c84b8ef1f75867b15f09292ac941d3a457be8bec410f6bc",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyhosts-0.4.3-windows-11-arm64.zip",
            sha256: "28e10831884064238ebf897d49eb7050f98e5550f7552061015d018fe4c778d3",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyhosts-0.4.3-windows-2022-x86_64.zip",
            sha256: "19e525b143662e3580d90dcba003b661a97b5bcd8ed4e00872081766cdd2b59d",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyhosts-0.4.3-windows-2025-x86_64.zip",
            sha256: "83483a4e5eaf8fd0f02c7bc6575d506f443bf0938408288e4cc4930f092fac89",
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
