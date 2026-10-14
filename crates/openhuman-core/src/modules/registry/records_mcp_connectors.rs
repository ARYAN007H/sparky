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
    version: "0.18.4",
    release_url: "https://github.com/tinyhumansai/tinymcp/releases/tag/v0.18.4",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-rolling-arm64",
            archive: "tinymcp-0.18.4-archlinux-arm64.tar.gz",
            sha256: "8a434d356550b9ea7b11b2cddf5625e850cf48af69605710b2b4276ed5e75b7d",
        },
        PlatformAsset {
            host_key: "archlinux-rolling-x86_64",
            archive: "tinymcp-0.18.4-archlinux-x86_64.tar.gz",
            sha256: "39c2c8d4948e5c875810211de5f959aaaf9494f817f9c93a2a9fdd4f99694684",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinymcp-0.18.4-fedora-43-arm64.tar.gz",
            sha256: "b7104d7ffe7cd496cb665880bc6bf2ca4050ac7f2721d3f87dcbdfbb3af57737",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinymcp-0.18.4-fedora-43-x86_64.tar.gz",
            sha256: "0dcaa7b7096896f46b9de847b01163f06119eb1b29592af9c96a7b58849dae66",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinymcp-0.18.4-fedora-44-arm64.tar.gz",
            sha256: "ce78d76cf17c32cb5dfaa7648987fca65b77a4818da84da626770b3e3b2a77f2",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinymcp-0.18.4-fedora-44-x86_64.tar.gz",
            sha256: "01d9f83f1ef1dec3c6db48887b3dd7c3be92ebed8dd306f6c2bb7989a0ad2afc",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinymcp-0.18.4-macos-15-arm64.tar.gz",
            sha256: "43c4e47db069a86767de487bc57a79face9dc9caa58261b25c5cc10c8716415d",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinymcp-0.18.4-macos-15-x86_64.tar.gz",
            sha256: "123a0d34db002d4b745d84aec3e04d26dca7221e4c2e3fe22b313a6ac5aa30cd",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinymcp-0.18.4-macos-26-arm64.tar.gz",
            sha256: "98b28fe4854e484ae630352975e36f06f2cb110e2645c36f2a9daeec80c8a7c5",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinymcp-0.18.4-macos-26-x86_64.tar.gz",
            sha256: "acad0bf15933bc04a0ca709cf95f00ae10c3440fd10f50de80bcdb911271361c",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinymcp-0.18.4-ubuntu-22.04-arm64.tar.gz",
            sha256: "98f6089d76e009ef647f835a4e985e443af13daee6b83fe0cc95d83fb239dad9",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinymcp-0.18.4-ubuntu-22.04-x86_64.tar.gz",
            sha256: "30ee02fdc4f2934de66cbec891ffc063f8f1ddb16d9cbf074f0463c592c1bdb5",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinymcp-0.18.4-ubuntu-24.04-arm64.tar.gz",
            sha256: "e3d96c039531af30842c050315551de3b8c954d29bdfa3b24e2ef9e36b53cf6d",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinymcp-0.18.4-ubuntu-24.04-x86_64.tar.gz",
            sha256: "a3ac5bccb3fc5ca08febafbe1c778e0b2843e73710fcab2ed0438f4073f61aed",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinymcp-0.18.4-windows-11-arm64.zip",
            sha256: "6954b5b490dc9acecfdbb82056fd674a9a8520c720b23b9cd51d09210cd3d812",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinymcp-0.18.4-windows-2022-x86_64.zip",
            sha256: "df3f69eea335c48c520989d56a44ef86a091ad0421e5e0270a61ca3a747c2eeb",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinymcp-0.18.4-windows-2025-x86_64.zip",
            sha256: "cdd4081565fe4c8c8365c7ead02c1ea2551735a5cf9031a367df524a49286295",
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
    version: "0.19.3",
    release_url: "https://github.com/tinyhumansai/tinyconnectors/releases/tag/v0.19.3",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinyconnectors-0.19.3-archlinux-arm64.tar.gz",
            sha256: "1b223604c366769a67d0345d12da9b1b679722b54062157c68fba08b608f3ec9",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinyconnectors-0.19.3-archlinux-x86_64.tar.gz",
            sha256: "b7c83dbf1a4f40a343fe687708412f5b33c137cf384ebea5d8b9b6223d890f58",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinyconnectors-0.19.3-fedora-43-arm64.tar.gz",
            sha256: "386552936cc9964ec4a006664656fa1ac58aaccd104dbf8fd71626824805dac4",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinyconnectors-0.19.3-fedora-43-x86_64.tar.gz",
            sha256: "7d810750a28d64ff332448095c91e57064da1dfdd70028ac378c86c88df8ca2c",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinyconnectors-0.19.3-fedora-44-arm64.tar.gz",
            sha256: "a7094a70e28f99dbe4312be4c0d5b49337b5b35e605bd07c7dde323e247fcdf6",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinyconnectors-0.19.3-fedora-44-x86_64.tar.gz",
            sha256: "39c4938e9daba8643910e2b4f8573c0559d1cf8a15b44bd99e615a9dab926081",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyconnectors-0.19.3-macos-15-arm64.tar.gz",
            sha256: "644a1c58c2ab7911ea2808c23b75b96e3ae312af1d12f97cca55f71971f551b9",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyconnectors-0.19.3-macos-15-x86_64.tar.gz",
            sha256: "53297c0fffad47ac19dba05a9e360af1601cf4de779e7db0d9e06d2059e91099",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyconnectors-0.19.3-macos-26-arm64.tar.gz",
            sha256: "96290a9f9b9b07fffbe3f333478002d7f46e5c2a563542d73ddac47420e597a4",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyconnectors-0.19.3-macos-26-x86_64.tar.gz",
            sha256: "990a0414fb4a2fc834efeedcd5e4e51e7057aed928f9c51aa4fae2c4a43a7f0d",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyconnectors-0.19.3-ubuntu-22.04-arm64.tar.gz",
            sha256: "f5ab8ced9095150df3ab0f3b84b4fe505a61d1fec64de535355d59a7ee13954f",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyconnectors-0.19.3-ubuntu-22.04-x86_64.tar.gz",
            sha256: "26daaa208a624c1afc5520fdd3d715eca032b956324ad5918431171097381620",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyconnectors-0.19.3-ubuntu-24.04-arm64.tar.gz",
            sha256: "ded63f30313e3196c0f75e6c6d211920d799df5e39b4d0657c4901de419db96e",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyconnectors-0.19.3-ubuntu-24.04-x86_64.tar.gz",
            sha256: "15cfa5718aeabe2882503c618e22c5067f050fbc3aaa4be31e541071700900f5",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyconnectors-0.19.3-windows-11-arm64.zip",
            sha256: "02a7e82e452c9190db68b5f2eef49b6fad73bf067dbb8f13ab3688c003dedf61",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyconnectors-0.19.3-windows-2022-x86_64.zip",
            sha256: "cf0d99f72732d934b24428cfa2ef414a649d41c612de330ea76af4e037641822",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyconnectors-0.19.3-windows-2025-x86_64.zip",
            sha256: "271bd3855a1565fedd041a02bbca2555d418bcd16d764b55385f6896d8e9682b",
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
