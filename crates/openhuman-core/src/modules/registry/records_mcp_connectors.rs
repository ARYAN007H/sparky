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
    version: "0.11.1",
    release_url: "https://github.com/tinyhumansai/tinymcp/releases/tag/v0.11.1",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-rolling-x86_64",
            archive: "tinymcp-0.11.1-archlinux-rolling-x86_64.tar.gz",
            sha256: "4b7704c703c724d37478330ca6e446150bfd516cc58b3b52c78c18cf9e216ed9",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinymcp-0.11.1-fedora-43-arm64.tar.gz",
            sha256: "d523cc1c19031fc024414a0352a26f2e2a833e607032d76a4d28b3ab13088b5f",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinymcp-0.11.1-fedora-43-x86_64.tar.gz",
            sha256: "9432d80588666fff92cc079c08018c134729adc59805b1087a91980524971698",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinymcp-0.11.1-fedora-44-arm64.tar.gz",
            sha256: "46d647924345ba76cb0e33a7c85041d554fabb0b4c7a25e04fe287a7d18b75a9",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinymcp-0.11.1-fedora-44-x86_64.tar.gz",
            sha256: "dc338ab36d6e5984e552339e04545873e21777426b783ff15416aa33363279e8",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinymcp-0.11.1-macos-15-arm64.tar.gz",
            sha256: "1ad5fa04ee47e64bc371b9e09e39128e82f0e582da1b7cecfb2b54bb09c7f8d3",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinymcp-0.11.1-macos-15-x86_64.tar.gz",
            sha256: "04fee0406a77e3d00ccdabd69d69edb8620afa8c4835cd16c7c8feddb7b697e2",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinymcp-0.11.1-macos-26-arm64.tar.gz",
            sha256: "244d3b7e9291677c6aa93ec494204cdcc93d677f76d53c883b20a0413ffc6d13",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinymcp-0.11.1-macos-26-x86_64.tar.gz",
            sha256: "85b4eeefae41e5df40abff54b410b7dd7aa7bb089b63989ca839acde541f0141",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinymcp-0.11.1-ubuntu-22.04-arm64.tar.gz",
            sha256: "2be71040efdf868bbf0c828f550c97735cac37887884ab3ecf2958ead534a676",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinymcp-0.11.1-ubuntu-22.04-x86_64.tar.gz",
            sha256: "ae7f687ac5747e5f5b67618b42bea130d7c948ec42c54a207e8201215bcdbe76",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinymcp-0.11.1-ubuntu-24.04-arm64.tar.gz",
            sha256: "962e3625be2c9a0caa37d5b1d007e5b08706b661708b9ae2c341ccb11c0b590b",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinymcp-0.11.1-ubuntu-24.04-x86_64.tar.gz",
            sha256: "baf880685d9b48b820dd334e417c52dec64595fbd5010768fbad9fcb6723b917",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinymcp-0.11.1-windows-11-arm64.zip",
            sha256: "af90a3ed2f788a3cbe65acde9d8180191bac2ed0df022f530fceeb3d21e7533a",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinymcp-0.11.1-windows-2022-x86_64.zip",
            sha256: "6190a49896e0071dab16cb95a67022c28a532a871219fe08d961121ef0c6ee2a",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinymcp-0.11.1-windows-2025-x86_64.zip",
            sha256: "aa20fadd0966cbcb5f994201060f20c1d6c077c3a252430c7c6d475a8985d70a",
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
