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
    version: "0.15.0",
    release_url: "https://github.com/tinyhumansai/tinyconnectors/releases/tag/v0.15.0",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyconnectors-0.15.0-ubuntu-24.04-x86_64.tar.gz",
            sha256: "61b4d4505ab68c2a2fa3621bc5416e33105b2dd9e78dd08c6d3ab804f3721c7a",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyconnectors-0.15.0-ubuntu-24.04-arm64.tar.gz",
            sha256: "696a458cee6469bc8cc267e7c539eba4fa06cba31e7dc43dfbe664a1ec4473a3",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyconnectors-0.15.0-ubuntu-22.04-x86_64.tar.gz",
            sha256: "be939f953b425b884b52480b8e5646f5f6fc6a82ac69bea3cbbc1da838ad74d6",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyconnectors-0.15.0-ubuntu-22.04-arm64.tar.gz",
            sha256: "3249ac2212f35b5d1748b2930a4f5519fab516f094203d2f8aeccf612b8988a1",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyconnectors-0.15.0-macos-26-arm64.tar.gz",
            sha256: "791cf7ac7f3c29840a54634d60a4ac18f9a40b8f9a53bfd486d087786c3cf82b",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyconnectors-0.15.0-macos-26-x86_64.tar.gz",
            sha256: "12a76f7009e9a87fed566edeea0d8a5103603370edac9aa3bec2cada461c647a",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyconnectors-0.15.0-macos-15-arm64.tar.gz",
            sha256: "5539a396c4c824ad9d28b77eed94e9ea4595b1996dc9f4d93957df4394b80261",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyconnectors-0.15.0-macos-15-x86_64.tar.gz",
            sha256: "329d51019bbcb9da1b18fcc9f4317c73734a777861d3a81496c4946eaf96df80",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyconnectors-0.15.0-windows-2025-x86_64.zip",
            sha256: "9b2b64f90aec0130a6905a9e012e6eef9911827058608868d8e539af9620dc0f",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyconnectors-0.15.0-windows-2022-x86_64.zip",
            sha256: "1208ccfb5126cc8e00e2ded9f7800b14cc9c3089266cc024b81361018e6d21b7",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyconnectors-0.15.0-windows-11-arm64.zip",
            sha256: "38d8af4a2a50c5ad03ae2f95df2f685af9e69ad4a968a07b043feed1d17e5a63",
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
