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
    version: "0.19.1",
    release_url: "https://github.com/tinyhumansai/tinyconnectors/releases/tag/v0.19.1",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinyconnectors-0.19.1-archlinux-arm64.tar.gz",
            sha256: "cc4eba0bb699b31871ae218730483752e5fbe255b17e3970d248731d5d83bd2b",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinyconnectors-0.19.1-archlinux-x86_64.tar.gz",
            sha256: "2b2326ce62a849bd0226615e4afcc873a770cccd6c0c186ba75acfd7912da296",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinyconnectors-0.19.1-fedora-43-arm64.tar.gz",
            sha256: "71108a10b9ce4579597ec480fb4bd8f22316d9fc53321df52bf146c05e1ae8b0",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinyconnectors-0.19.1-fedora-43-x86_64.tar.gz",
            sha256: "dba369e8ee8bec31ef39a125b3cd25134c586056bca60986d42b96e8eb1154a9",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinyconnectors-0.19.1-fedora-44-arm64.tar.gz",
            sha256: "671f03485ebd89526ebe7866d55fe4216958e1f1293cd4b5a24009b08f1b8a9e",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinyconnectors-0.19.1-fedora-44-x86_64.tar.gz",
            sha256: "0ac1cbb97ff6066da73b9c87dad94c01a1fb7ae89b452894a95ccb883965f531",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyconnectors-0.19.1-macos-15-arm64.tar.gz",
            sha256: "70a0b34c6aa711d8f9707a5c471e1e582e43d7ffe1a3d559f08b3087d1852af5",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyconnectors-0.19.1-macos-15-x86_64.tar.gz",
            sha256: "ef03484f6cbe314117f6c71e314fa50eab21e5d2574ca5958fefce1dd160af6e",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyconnectors-0.19.1-macos-26-arm64.tar.gz",
            sha256: "97c84c8d42864e92aebdfd7563352eba919dd7aca4fc604bb99abfb3cba6f05f",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyconnectors-0.19.1-macos-26-x86_64.tar.gz",
            sha256: "82e0a01cf0830c62e79c86302f9ab4e4811645188060c3fde87fc098e18dd10a",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyconnectors-0.19.1-ubuntu-22.04-arm64.tar.gz",
            sha256: "6ef9c0faa19dd22cc0ba3e89d80f7bdd9559231e03429fae09d3fffceaac338e",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyconnectors-0.19.1-ubuntu-22.04-x86_64.tar.gz",
            sha256: "d1479bd2079d1fd743ad62cd4ed3e8120a14bc11c6d7ad6f6d3bed0047547125",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyconnectors-0.19.1-ubuntu-24.04-arm64.tar.gz",
            sha256: "024c0ecf301dd3f1bae33efc4699c4990cc2416cc90e6f310201867b205b994d",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyconnectors-0.19.1-ubuntu-24.04-x86_64.tar.gz",
            sha256: "8a92e1fd49241231be9a36004c6a68a14c245fcfaceb046924884ba0a87ee51b",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyconnectors-0.19.1-windows-11-arm64.zip",
            sha256: "f8922118614f942d13bcf21c22dd871f0873d1884543812c19bc9086f9c32c03",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyconnectors-0.19.1-windows-2022-x86_64.zip",
            sha256: "a470bbb9d321653d32e5caa9255d3fd4c28b36ada234181450a3b8fa148fcb01",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyconnectors-0.19.1-windows-2025-x86_64.zip",
            sha256: "618247ce6d27003372c744132ef0dcc367ea83c604978776fab7b4f579c424a8",
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
