//! Registry records for the `tinymcp` and `tinyconnectors` modules.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

/// The `tinymcp` module: the Model Context Protocol client.
///
/// Owns both transports (Streamable HTTP and a subprocess over stdio), the
/// statically declared server set a host puts in its own configuration, the
/// dynamic registry of user-installed servers with its SQLite store, the
/// reconnect supervisor, the browser sign-in flow, and the write-audit log.
///
/// Lazy, because dialing an MCP server is something most sessions never do: a
/// host with no installed servers and no configured ones would otherwise pay a
/// download and a `dlopen` for a capability it never reaches. That differs from
/// the module's own `lazy = false` export hint, which speaks for a host whose
/// servers should be connected the moment it comes up — this host decides when
/// that moment is, and does so on the first ask.
///
/// **What stays out of the module is host policy**, and the split is the same
/// one the contract's own documentation draws: the prompt-injection scan over
/// remote tool definitions, the `mcp_clients` RPC surface, the
/// agent-facing tools, and the proxy *scoping* decision all belong to this
/// application's threat model, not to a protocol client. `tinymcp-bus` carries
/// the vocabulary; this table says which bytes may speak it.
pub(crate) const TINYMCP: ModuleRecord = ModuleRecord {
    id: "tinymcp",
    description: "Model Context Protocol client: transports, registry, and the write-audit log",
    bus_name: tinymcp_bus::names::INTERFACE,
    object_path: tinymcp_bus::names::OBJECT_PATH,
    version: "0.16.1",
    release_url: "https://github.com/tinyhumansai/tinymcp/releases/tag/v0.16.1",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-rolling-arm64",
            archive: "tinymcp-0.16.1-archlinux-arm64.tar.gz",
            sha256: "6a1c3f73c4b3002c7ca35a81170a6e727fc0df942607b5b28a65d1f8afa31565",
        },
        PlatformAsset {
            host_key: "archlinux-rolling-x86_64",
            archive: "tinymcp-0.16.1-archlinux-x86_64.tar.gz",
            sha256: "f6eba52cb67f95c0f21217c02efedc0adc1611204b8e0b49319d9d4b60e53681",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinymcp-0.16.1-fedora-43-arm64.tar.gz",
            sha256: "34d21f069d145188f0a5a14169f58c71227bca985bacde9062dda8ba8ba8e526",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinymcp-0.16.1-fedora-43-x86_64.tar.gz",
            sha256: "7b81c3a28db19f54fec6a98b3226e7a96328633b5bf7de398ef082a551832fbf",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinymcp-0.16.1-fedora-44-arm64.tar.gz",
            sha256: "e95f24918ea02b355fb269922acc54bb47c49ee31c74f9686903482880851040",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinymcp-0.16.1-fedora-44-x86_64.tar.gz",
            sha256: "ee4a4a2f071d951baeb839d3f05573575585af253645e6735edb1aab0f07cd5d",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinymcp-0.16.1-macos-15-arm64.tar.gz",
            sha256: "2aea627b5cf3d5f1d7328dc5ee9b5eb1a67ebd918f6d1dd71987f315b2b9cf47",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinymcp-0.16.1-macos-15-x86_64.tar.gz",
            sha256: "5aa351af862dc496b6fecb1e355f8fe209959c8148a0c4625341e22ffaaee7d9",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinymcp-0.16.1-macos-26-arm64.tar.gz",
            sha256: "2a7bb7dc867a2f4d033f8c41e9169dad0eee4887b7668ef87df08691021dca5e",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinymcp-0.16.1-macos-26-x86_64.tar.gz",
            sha256: "03384b0f0b57410d4de9ab121ac3b3267f7af2ead334c7705b4c7b751285fdd9",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinymcp-0.16.1-ubuntu-22.04-arm64.tar.gz",
            sha256: "5226c7b9abf1e11ae2070c6d5c57b804264df2b11b1a317a51c40a18da8189be",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinymcp-0.16.1-ubuntu-22.04-x86_64.tar.gz",
            sha256: "6c8defda965dfdfb764c6ba7b239cc4656ce98707bda30433d1f77d0b2750087",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinymcp-0.16.1-ubuntu-24.04-arm64.tar.gz",
            sha256: "3df2d754b894f37c54a451b988d7d85242f18a92ea02da1388e38b835c739f3d",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinymcp-0.16.1-ubuntu-24.04-x86_64.tar.gz",
            sha256: "20e98ee6a52bbcbd53714642400a65285f6039cdad89ce64d04a5b7f3deb168c",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinymcp-0.16.1-windows-11-arm64.zip",
            sha256: "dd81b22ada15f34e3d710e5b7cc3c7ad7e5784d1cb7d784760c72ba77f6950f2",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinymcp-0.16.1-windows-2022-x86_64.zip",
            sha256: "21e1422ca50d4206102d406594e78f54974ef39b401d429b6b41d8bcfc8f901b",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinymcp-0.16.1-windows-2025-x86_64.zip",
            sha256: "e2ed40815c76717d4c01c2b3b5ae05808d7a0c22402b980f0f6bddf1a50be5d3",
        },
    ],
    load: LoadPolicy::Lazy,
    source: SourceSpec {
        submodule: "tinymcp",
        manifest: "crates/tinymcp/Cargo.toml",
        package: "tinymcp",
        lib_name: "tinymcp",
        features: &[],
    },
};

pub(crate) const TINYCONNECTORS: ModuleRecord = ModuleRecord {
    id: "tinyconnectors",
    description: "OAuth connector integrations: accounts, actions, and triggers",
    bus_name: tinyconnectors_bus::names::INTERFACE,
    object_path: tinyconnectors_bus::names::OBJECT_PATH,
    version: "0.17.0",
    release_url: "https://github.com/tinyhumansai/tinyconnectors/releases/tag/v0.17.0",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyconnectors-0.17.0-ubuntu-24.04-x86_64.tar.gz",
            sha256: "a5519ae7ffefda10d9123acd562d60a9ba9b6f4d64702293cc5ed295bbf6dd54",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyconnectors-0.17.0-ubuntu-24.04-arm64.tar.gz",
            sha256: "9dbdcecbd924a66c714941c4b3178c4f4c24172b1639d9c4f093803ad4243fda",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyconnectors-0.17.0-ubuntu-22.04-x86_64.tar.gz",
            sha256: "283c739b07be7652be35c36fd3772f860d68f17323e50f2e3d6669da0fa87c36",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyconnectors-0.17.0-ubuntu-22.04-arm64.tar.gz",
            sha256: "48615f4fd2581a2b88049a8a4f527212210eda41a9b2eb44c542bc72137f68e9",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyconnectors-0.17.0-macos-26-arm64.tar.gz",
            sha256: "063cc8ca4e5cae044f2530a538ecf38627b54814a41d619ae8af9d9c5a163be0",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyconnectors-0.17.0-macos-26-x86_64.tar.gz",
            sha256: "5384f6fb3dd5159de629921799d4a17656f5ccc83b9cab52e3976be915d1a3ad",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyconnectors-0.17.0-macos-15-arm64.tar.gz",
            sha256: "35ea4b27c7e2d320b33d036f86d8b55a93cef5365350a781e21c159fe330cf8a",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyconnectors-0.17.0-macos-15-x86_64.tar.gz",
            sha256: "a211341fc876274125ce851630078b6d2ed552e6f29782d3164a11870710f066",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyconnectors-0.17.0-windows-2025-x86_64.zip",
            sha256: "ed8672a2d42180322e36ab080711e88cb90f80c0ae239b6cd960a872e2375c13",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyconnectors-0.17.0-windows-2022-x86_64.zip",
            sha256: "32e18a099d2b6c8a2ac6b5012d9f6b221fdf0b7b9ec6e7914d819a24eb58aceb",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyconnectors-0.17.0-windows-11-arm64.zip",
            sha256: "6956eba7c7e24b354849f33b2d59b63ec455ca7f3ee30f8af6df9cd5d3477edb",
        },
    ],
    // Lazy: a user with no connected accounts should not pay to load it, and
    // most sessions never touch a connector. Safe even signed out — the module
    // loads without configuration and still answers the capability members.
    load: LoadPolicy::Lazy,
    source: SourceSpec {
        submodule: "tinyconnectors",
        manifest: "crates/tinyconnectors/Cargo.toml",
        package: "tinyconnectors",
        lib_name: "tinyconnectors",
        features: &[],
    },
};
