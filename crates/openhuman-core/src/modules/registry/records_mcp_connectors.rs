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
    version: "0.15.0",
    release_url: "https://github.com/tinyhumansai/tinymcp/releases/tag/v0.15.0",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-rolling-arm64",
            archive: "tinymcp-0.15.0-archlinux-arm64.tar.gz",
            sha256: "c77d02b8c03278ad24294dc63e0cd1ae490d4d17ee7b8543e71c2600176a7b60",
        },
        PlatformAsset {
            host_key: "archlinux-rolling-x86_64",
            archive: "tinymcp-0.15.0-archlinux-x86_64.tar.gz",
            sha256: "836259b5cb2a06da6df598593fe2b60670d36faf3151e875946f16ccb5456fff",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinymcp-0.15.0-fedora-43-arm64.tar.gz",
            sha256: "0ba117c96137e13f958de0a557094a06c58797c7c03eee1b8fedfc25c7f17b6a",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinymcp-0.15.0-fedora-43-x86_64.tar.gz",
            sha256: "da5154722f65019f7de2452662f1f46c8425bfba95d0aa6613dab1efff6fec7e",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinymcp-0.15.0-fedora-44-arm64.tar.gz",
            sha256: "ada62a47ad9d88a4bd9f8e0defe67718a75db2f30e8e298e838b7d5d3f036f4f",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinymcp-0.15.0-fedora-44-x86_64.tar.gz",
            sha256: "2036d7a13f06bfb243fd7f54626cdcf7ed4653a534ac780503ee9980be1488ca",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinymcp-0.15.0-macos-15-arm64.tar.gz",
            sha256: "11aaa66c1dc42ec2b6f6a70c01d1c98e95bef22c7f626d32c6da9455b2cdf38f",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinymcp-0.15.0-macos-15-x86_64.tar.gz",
            sha256: "11ee1ecede7599f575d75fcc3e030ea0f18df9ffe91a8264dd20da4453825569",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinymcp-0.15.0-macos-26-arm64.tar.gz",
            sha256: "4062a4a7de2bda4d707960c715412ce61d66e8b5983f05b75ef5bca5931eb866",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinymcp-0.15.0-macos-26-x86_64.tar.gz",
            sha256: "8a144d5f19bb743186373ec18c118da9f1b772d592a14caed892f808a690c41f",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinymcp-0.15.0-ubuntu-22.04-arm64.tar.gz",
            sha256: "1d3de88aeaf2255d44f5895ae06d7c7b4e2869314fff0efe8661a1eea50d0863",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinymcp-0.15.0-ubuntu-22.04-x86_64.tar.gz",
            sha256: "aacd2ea47433e8c61941be71b5ff95036afe4435da4c38f2ab1dfa0d31ddade1",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinymcp-0.15.0-ubuntu-24.04-arm64.tar.gz",
            sha256: "c5dcc8c5f74d00f08d8a4dace062ebaaf347edcee7342d11ff0ba9da840e5807",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinymcp-0.15.0-ubuntu-24.04-x86_64.tar.gz",
            sha256: "fefa976a7e8ea19c3c63ce9dedb6652411258c478b0c16abceaa460c3afb95e7",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinymcp-0.15.0-windows-11-arm64.zip",
            sha256: "055fc3a47d050ab5fa76c993ff85d8690dbcaaab31b4673ba40e88575951d9a8",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinymcp-0.15.0-windows-2022-x86_64.zip",
            sha256: "e9f06cc33a3e29f1de05935c26238c2f23c46936544ba55a74ac89e3c8c8f690",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinymcp-0.15.0-windows-2025-x86_64.zip",
            sha256: "943dea2ae0695c10a9f4376b5025c560f05668f5bd121e06e355bebfc819f970",
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
    version: "0.14.0",
    release_url: "https://github.com/tinyhumansai/tinyconnectors/releases/tag/v0.14.0",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyconnectors-0.14.0-ubuntu-24.04-x86_64.tar.gz",
            sha256: "836bffa2fe4f935c8504f608e08bdf023326d1285fc977fd37149f2799899478",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyconnectors-0.14.0-ubuntu-24.04-arm64.tar.gz",
            sha256: "b7dbafe019aca715e55444a37b5ff4fc469ef2bcc675dfa987e1de626e70fd22",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyconnectors-0.14.0-ubuntu-22.04-x86_64.tar.gz",
            sha256: "a29cb970b385cf8cd40998baa3b6f040447196d019a8d3413d67ca64a6509687",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyconnectors-0.14.0-ubuntu-22.04-arm64.tar.gz",
            sha256: "e961477b9cac1892beef8b498ba4f44daa4b39b8dddf277ec49496e3869427b5",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyconnectors-0.14.0-macos-26-arm64.tar.gz",
            sha256: "5a24f1eabb0f246a5954e4d6f93cc0feeaed4fe6bfbf1fceb7399078e0e5734a",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyconnectors-0.14.0-macos-26-x86_64.tar.gz",
            sha256: "5df7dbc74b2f39174e57695c5556ab4c232b775897e7bd534be015ead01ef51b",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyconnectors-0.14.0-macos-15-arm64.tar.gz",
            sha256: "ea8010163f279f14e25a4ad884b4665e24385c560dd2caf46e8c80220556ba11",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyconnectors-0.14.0-macos-15-x86_64.tar.gz",
            sha256: "508e44cc9cd273dcbd2d269295ab0b7009efb1a27d3244ba9adde6b297306db1",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyconnectors-0.14.0-windows-2025-x86_64.zip",
            sha256: "5a5e62e8d11bbdc792c9d8fd2a9b59bcea9add80978457ea0faa7c058cd48546",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyconnectors-0.14.0-windows-2022-x86_64.zip",
            sha256: "1560ed909f338a5bf1118d644d44e1fbeee5c26461241df9f24630e3f9ba3cbe",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyconnectors-0.14.0-windows-11-arm64.zip",
            sha256: "0bdfb0424ad62fc93ba66a7d8dd7063a39ae8656b78d2e65f4f1a6425aec3279",
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
