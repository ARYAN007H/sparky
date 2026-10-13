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
    version: "0.18.2",
    release_url: "https://github.com/tinyhumansai/tinymcp/releases/tag/v0.18.2",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-rolling-arm64",
            archive: "tinymcp-0.18.2-archlinux-arm64.tar.gz",
            sha256: "81d3e59954938768f8384dcf30b0b8aa79f008f0943cce60cd4ba77f0e1ec2dc",
        },
        PlatformAsset {
            host_key: "archlinux-rolling-x86_64",
            archive: "tinymcp-0.18.2-archlinux-x86_64.tar.gz",
            sha256: "54c44adc674ece88f747c6d977df661ad7e068a8f0e5769533995c1f2423d87c",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinymcp-0.18.2-fedora-43-arm64.tar.gz",
            sha256: "4b4e66d9cd14df0d264c35cf85c445f4df1f6ca9a33651815c1d90711b21d032",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinymcp-0.18.2-fedora-43-x86_64.tar.gz",
            sha256: "dd1f8fb674e8a07d795bbcabf7c69a479e9bd1fdcdba6ac2af930d02af3570e6",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinymcp-0.18.2-fedora-44-arm64.tar.gz",
            sha256: "1b3733741afed877f9c7757ed6c48f75235ac7017e99f717c3334954221d05ae",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinymcp-0.18.2-fedora-44-x86_64.tar.gz",
            sha256: "69328ff8e8b553cf095a482c41a94418caf0bcb44f36921acf583b30cf8eb713",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinymcp-0.18.2-macos-15-arm64.tar.gz",
            sha256: "da8d98429d356914e2e953c73c99613bb0f333af99321c7f2574e2950423acf7",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinymcp-0.18.2-macos-15-x86_64.tar.gz",
            sha256: "73adee09c2fd747a180a43f9c6700ea03e3f97544994e2564fc9775e2dfe3ce9",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinymcp-0.18.2-macos-26-arm64.tar.gz",
            sha256: "f334675b6b3565faeb6e17f0d2e8caafbf87a72e72aef1fad0d8c8ba40c49f0b",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinymcp-0.18.2-macos-26-x86_64.tar.gz",
            sha256: "00d094d768553e9dfdb73ae298bab1c5103f3ae806ad5fdd36eda4bfdafc4751",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinymcp-0.18.2-ubuntu-22.04-arm64.tar.gz",
            sha256: "bdbcd45b415fe1c30d1c7fe583d7a71a2296ef51290b9e545dc02812a3f0229b",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinymcp-0.18.2-ubuntu-22.04-x86_64.tar.gz",
            sha256: "4313c456e1e947ac090fec304b13229baba680175ab7312e38b230e5c42393d3",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinymcp-0.18.2-ubuntu-24.04-arm64.tar.gz",
            sha256: "41cc557273334b6da977b0b0a29f37364a2455a5444cb0fff8c63ab976a846f4",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinymcp-0.18.2-ubuntu-24.04-x86_64.tar.gz",
            sha256: "e74f33f32a5dfb3209a414236ea368bb746e648d7b24fa0dbd15a3a3df908147",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinymcp-0.18.2-windows-11-arm64.zip",
            sha256: "e1c53505f1137f1c0a536f9d16e280d834ec96706ef34d64095931419b6260f6",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinymcp-0.18.2-windows-2022-x86_64.zip",
            sha256: "c9e43713feab3fa66a29328170b3b25cbabe2ae2f1ef0431291c43805ba950c8",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinymcp-0.18.2-windows-2025-x86_64.zip",
            sha256: "790be0a331f000d34b93515efe67954ec33fe5d869f529c970649aea5db1d8b7",
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
    version: "0.19.0",
    release_url: "https://github.com/tinyhumansai/tinyconnectors/releases/tag/v0.19.0",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinyconnectors-0.19.0-archlinux-arm64.tar.gz",
            sha256: "5c6c781ebf782d126ebad4bfa1afe0b2e3972815b6f792b636b8be85874c6343",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinyconnectors-0.19.0-archlinux-x86_64.tar.gz",
            sha256: "6c57f02f37e2ef849364354e92beb75460bf6245bc7e9cdca1897492a4fa8db9",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinyconnectors-0.19.0-fedora-43-arm64.tar.gz",
            sha256: "8be5a7320483179c79c4442561c386960ee11ca9e1fd1632b6e9e49ddaebbca6",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinyconnectors-0.19.0-fedora-43-x86_64.tar.gz",
            sha256: "a109e622392b8002abdc26c7acd9d5cc60f11c357c2e3d6481c0caf3ce33ccc6",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinyconnectors-0.19.0-fedora-44-arm64.tar.gz",
            sha256: "cee6da0dfc4400fc0054bbe61060b58c4add03e0bdfaee77517a2d12c99818d7",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinyconnectors-0.19.0-fedora-44-x86_64.tar.gz",
            sha256: "a5a22f0f7e9d95c9b9f57ed9c3e51cce09404cdcca2967f7a7fb04638587e52b",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyconnectors-0.19.0-macos-15-arm64.tar.gz",
            sha256: "9e719905168a41cd9b1fed09e33b657d0423847911b38fbcf720b314ca5c5f2d",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyconnectors-0.19.0-macos-15-x86_64.tar.gz",
            sha256: "13b699742f11d889af10289afb87bb27d715a241f96c153afdb0c4f0b3d1e342",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyconnectors-0.19.0-macos-26-arm64.tar.gz",
            sha256: "569269cc0feca41c9875a9220a1f4c1cebd051515974b251e356fc04769360f2",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyconnectors-0.19.0-macos-26-x86_64.tar.gz",
            sha256: "78e3b69d79962885bf30137903694101a09855fa9b0fdc0b767c65df20ce14d6",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyconnectors-0.19.0-ubuntu-22.04-arm64.tar.gz",
            sha256: "904a3e5da5a6ca514abaebd7978fd93115b41de89b01c78087f69635200ffdaf",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyconnectors-0.19.0-ubuntu-22.04-x86_64.tar.gz",
            sha256: "7a4264211de403363a426b883e09a0ce4de26bda92b15a4035b8993c9910d3d2",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyconnectors-0.19.0-ubuntu-24.04-arm64.tar.gz",
            sha256: "f89622dee9ca3f0c473dff822b8b1b6f0ae08cfcb065a543299a962b8ab324d7",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyconnectors-0.19.0-ubuntu-24.04-x86_64.tar.gz",
            sha256: "d4c081d41d09d610c6245358854a81e42972639b0e0514a46caf093bd7bf98c5",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyconnectors-0.19.0-windows-11-arm64.zip",
            sha256: "7d27c582d614c86332ce851c3fb0a14aa71b7cfbc45bad339b58008da1856cf4",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyconnectors-0.19.0-windows-2022-x86_64.zip",
            sha256: "3402bed1d0bdf98d7c02232db16aa14d74f443f0b0741683d0a8de2fee093267",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyconnectors-0.19.0-windows-2025-x86_64.zip",
            sha256: "aab4c65c744a0c258043b8044a7c0a0fdfcb19ed8a056403f351fa9c216103d4",
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
