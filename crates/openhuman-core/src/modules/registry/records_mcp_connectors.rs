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
    version: "0.19.2",
    release_url: "https://github.com/tinyhumansai/tinyconnectors/releases/tag/v0.19.2",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinyconnectors-0.19.2-archlinux-arm64.tar.gz",
            sha256: "b47376cfce7b8e9f72848eea420fdfb39de2a224a4093bac9134f2b29a29252a",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinyconnectors-0.19.2-archlinux-x86_64.tar.gz",
            sha256: "ffe6f0fefff0ac1d0d2495d10f59538b318ccaea63f0843aa0d85398e0954751",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinyconnectors-0.19.2-fedora-43-arm64.tar.gz",
            sha256: "69a2198c10a4691874f2207d24f6ebc24ccd35fc6b35c2fef3350f1646d92977",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinyconnectors-0.19.2-fedora-43-x86_64.tar.gz",
            sha256: "47f7645663333338541e5f315809e9d2e56eec7e50146112198f5e029052058d",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinyconnectors-0.19.2-fedora-44-arm64.tar.gz",
            sha256: "061521ffec706e498087b111147f51140466c5bfbf4869af396c5bb1266da3fb",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinyconnectors-0.19.2-fedora-44-x86_64.tar.gz",
            sha256: "8d0eb0c66fc93edfbb10718196e8f92309348004c42cddcc3ac20df5b3097f2f",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyconnectors-0.19.2-macos-15-arm64.tar.gz",
            sha256: "8de84e0d0fd802a890fe4c6d9cc2cbdf3dae5b1c83d31bdc72a8dc22ae32ccd7",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyconnectors-0.19.2-macos-15-x86_64.tar.gz",
            sha256: "df3e1075fbfff0d2b8b51e9ceb49d993c212f97e6359038aaee03ece683a09e9",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyconnectors-0.19.2-macos-26-arm64.tar.gz",
            sha256: "a62b4687d9cd8fc3580d04fa6114f24c14e133b32aa11edf3ad8df386ed68c75",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyconnectors-0.19.2-macos-26-x86_64.tar.gz",
            sha256: "5fd930cc6d3a760965a5377b238b2b466d1060c7ac0c544d5e09613b8705e060",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyconnectors-0.19.2-ubuntu-22.04-arm64.tar.gz",
            sha256: "9bffe56deb7a094245bfde56a6fe97ab26a8cab123a1afe2efefda50302ebf0f",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyconnectors-0.19.2-ubuntu-22.04-x86_64.tar.gz",
            sha256: "d22cdb177ff84af72fc19ca55ceab5f4fe7380389061811ee93fe01410a05b0d",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyconnectors-0.19.2-ubuntu-24.04-arm64.tar.gz",
            sha256: "f5ebc74a9f326990123884c190528cab0e7e80f26ebb138b4d0f78d2882b1006",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyconnectors-0.19.2-ubuntu-24.04-x86_64.tar.gz",
            sha256: "9d4e7d255197e7ec4e41e4fc68d53714dad3b4bc3dd17381bdd779d7e7454d48",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyconnectors-0.19.2-windows-11-arm64.zip",
            sha256: "bbc136ad5800739a171b01b2eb578c5102c4cedc40de849172b77c0f0929a38c",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyconnectors-0.19.2-windows-2022-x86_64.zip",
            sha256: "10a3412df6549fb3e8ecc7f69703509bfacb6679b6c82b28fa0227ea8e09a145",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyconnectors-0.19.2-windows-2025-x86_64.zip",
            sha256: "ff4f7b4c68bc2799695ab8990673bcfd1124a43ad2f43dfdba57a06763c2f83a",
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
