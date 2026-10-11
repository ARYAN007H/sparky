//! Native computer-use module (desktop and browser control). Published digests are added only from its release manifest.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset};
use tinycomputer_bus::names;

pub(crate) const TINYCOMPUTER: ModuleRecord = ModuleRecord {
    id: "tinycomputer",
    description: "Permission-aware desktop and browser observation and control",
    bus_name: names::INTERFACE,
    object_path: names::OBJECT_PATH,
    version: "0.11.0",
    release_url: "https://github.com/tinyhumansai/tinycomputer/releases/tag/v0.11.0",
    // Verbatim from the published tinycomputer v0.11.0 checksum.toml. Linux remains outside
    // the initial product surface; this registry admits macOS and Windows.
    assets: &[
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinycomputer-0.11.0-macos-26-arm64.tar.gz",
            sha256: "3a8e20a809262be2154c1b7339fed2a627c13c32b2a7b9de52e4d54ced94a994",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinycomputer-0.11.0-macos-26-x86_64.tar.gz",
            sha256: "a61a79cb63798b844009cc8c69575690d6b2d5a25b637e0b05d559b14ad66c10",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinycomputer-0.11.0-macos-15-arm64.tar.gz",
            sha256: "4654e5143f81ac0865df37cc25ab3a003c7fe91edb0942c0d44d6382519d2b05",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinycomputer-0.11.0-macos-15-x86_64.tar.gz",
            sha256: "38eddc6ed86dbed7c91a36bda3ab8fe7cd46db1dd467a84003bfadf5a54ec2b7",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinycomputer-0.11.0-windows-2025-x86_64.zip",
            sha256: "b65b7ea2f8f678e5a55920b8d72ae62642e3e03ba44e0db78bc8807e8c62520d",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinycomputer-0.11.0-windows-2022-x86_64.zip",
            sha256: "494efca1d2aee2b8371bebc70cb6b791426ada4bc35da0524c7317fd084e9157",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinycomputer-0.11.0-windows-11-arm64.zip",
            sha256: "df97f97f18ed670e57b8b79775fc9bc4cbe44b59f9320c957f93f89f7eba41c0",
        },
    ],
    load: LoadPolicy::Lazy,
};
