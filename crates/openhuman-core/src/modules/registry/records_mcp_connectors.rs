//! Registry records for the `tinymcp` and `tinyconnectors` modules.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset};

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
    bus_name: "ai.tinyhumans.tinymcp.Mcp",
    object_path: "/ai/tinyhumans/tinymcp/Mcp",
    version: "0.14.2",
    release_url: "https://github.com/tinyhumansai/tinymcp/releases/tag/v0.14.2",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-rolling-arm64",
            archive: "tinymcp-0.14.2-archlinux-arm64.tar.gz",
            sha256: "e473bdfab9ab21d2e7bc50bebc1a335eb8ef9efff03bb352ab57ac193d1c0dc7",
        },
        PlatformAsset {
            host_key: "archlinux-rolling-x86_64",
            archive: "tinymcp-0.14.2-archlinux-x86_64.tar.gz",
            sha256: "a9d9862f09955785a892fc4f7f084a5d619e0259449bf571ff6a79dbbc629e85",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinymcp-0.14.2-fedora-43-arm64.tar.gz",
            sha256: "ff58f2dd6e504a837b77c14e960a2e95fc09f8526ec3f6cce098b4cd8034b407",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinymcp-0.14.2-fedora-43-x86_64.tar.gz",
            sha256: "6aa3f0a7735222362dea8447929c3a50bf5679dfbdbba4a0a3984332bddbd65b",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinymcp-0.14.2-fedora-44-arm64.tar.gz",
            sha256: "d384d6a5da0aac0f7caf8f845e76b629bd23fb568e5067b8db34524c7371426c",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinymcp-0.14.2-fedora-44-x86_64.tar.gz",
            sha256: "d37f84a217ef08229e33b7b052b0a4875a4de8999249154e5862269286aef313",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinymcp-0.14.2-macos-15-arm64.tar.gz",
            sha256: "95b0776e17980188e969a92c3a64eb2477a5f384a07983f2fa00174dbcce24db",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinymcp-0.14.2-macos-15-x86_64.tar.gz",
            sha256: "c3b1395c74f514129e6b3fea145c7b447199d47675f125f6fd60ce16541bb7ef",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinymcp-0.14.2-macos-26-arm64.tar.gz",
            sha256: "6519710f74386e53065148e5b69519fbb4cc8239473cfad710f87875f221bdf3",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinymcp-0.14.2-macos-26-x86_64.tar.gz",
            sha256: "e878c2d88e90437d25b7eb471aca3a41e5d06ff90be2cf5bd37410dd260f884c",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinymcp-0.14.2-ubuntu-22.04-arm64.tar.gz",
            sha256: "6fa51ca50339a7595d6cc28a5616bc0df571983136fb492a1ac9ca8adf7cbe65",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinymcp-0.14.2-ubuntu-22.04-x86_64.tar.gz",
            sha256: "289e906ff7312b67815ee41823b98c0cadcafc697f91e921a55cb81679785263",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinymcp-0.14.2-ubuntu-24.04-arm64.tar.gz",
            sha256: "eaf2a70be275e3f50e5a26dadc6af59f6fd03a219a39a49c9b710e39e8369087",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinymcp-0.14.2-ubuntu-24.04-x86_64.tar.gz",
            sha256: "260554caeb5dd6d8b4183c62c4589ba550a82a726838134394497a55f89f6eae",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinymcp-0.14.2-windows-11-arm64.zip",
            sha256: "466e86f444b7e995f1007366bc5d72d8238d0fa07c9d7dd68c74069258b5d75d",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinymcp-0.14.2-windows-2022-x86_64.zip",
            sha256: "f6b25f6987c36414ff146ae086b6bcdc24076886b663ae2eade7bf2ed2dea7f5",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinymcp-0.14.2-windows-2025-x86_64.zip",
            sha256: "60f31b81fe721ed0af14b8891e089c6473d7259a1f2a048cde3afb1c760b518e",
        },
    ],
    load: LoadPolicy::Lazy,
};

pub(crate) const TINYCONNECTORS: ModuleRecord = ModuleRecord {
    id: "tinyconnectors",
    description: "OAuth connector integrations: accounts, actions, and triggers",
    bus_name: "ai.tinyhumans.connectors.Composio",
    object_path: "/ai/tinyhumans/connectors/Composio",
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
};
