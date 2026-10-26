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
    version: "0.20.3",
    release_url: "https://github.com/tinyhumansai/tinymcp/releases/tag/v0.20.3",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-rolling-arm64",
            archive: "tinymcp-0.20.3-archlinux-arm64.tar.gz",
            sha256: "aee29238dae7f08173daba386dcf7630890051f9793d20444cc01ccb07e995ae",
        },
        PlatformAsset {
            host_key: "archlinux-rolling-x86_64",
            archive: "tinymcp-0.20.3-archlinux-x86_64.tar.gz",
            sha256: "73cef881d21b0506fb1dc0b4ee9fcc3148dd536a8719e6c7106a02a93d41c8fc",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinymcp-0.20.3-fedora-43-arm64.tar.gz",
            sha256: "cd57e0c664e4dd5085dfea368a2b3528431e5d5999b04b0b0875c9fa2d3102df",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinymcp-0.20.3-fedora-43-x86_64.tar.gz",
            sha256: "78e93ea6d918a8296b9af5489fd6135d5dc2a4600adcff845e6cc7e0442b3022",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinymcp-0.20.3-fedora-44-arm64.tar.gz",
            sha256: "42ea550016832503bf2e342a8207d3e0d582d7c69b161b1ceaca33a10f2c495c",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinymcp-0.20.3-fedora-44-x86_64.tar.gz",
            sha256: "0f9bfb3602b9153bcbc7a1c70fb29aea3d6f7873ab024b6791f7d571b749d7a3",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinymcp-0.20.3-macos-15-arm64.tar.gz",
            sha256: "3efc9a2fbaca40415733d3e22c2521e037d6d3e0d84109c12e9e82317fd2545b",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinymcp-0.20.3-macos-15-x86_64.tar.gz",
            sha256: "34481bf3379612686fc5c1dc296b441f52f437b3ce7bc74ab74ef31389623230",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinymcp-0.20.3-macos-26-arm64.tar.gz",
            sha256: "cc073af4aa19fbaf7ed7b3c2749affd4bdff3f0c2856970e07da1f30a710b964",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinymcp-0.20.3-macos-26-x86_64.tar.gz",
            sha256: "2f7cccc732d091b87d884263ee18149b490dd522969e31eb3ec6295c874a31e7",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinymcp-0.20.3-ubuntu-22.04-arm64.tar.gz",
            sha256: "960b06810b7164fcc7d32139cfe0a0d9f5e144268bbe59c0d8e7d9f2841810fa",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinymcp-0.20.3-ubuntu-22.04-x86_64.tar.gz",
            sha256: "0911cfcfb2647d1a41d8a87e3959e84e4336f54b7939f473e0bf8e6eb2ec5432",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinymcp-0.20.3-ubuntu-24.04-arm64.tar.gz",
            sha256: "6e93b86b9b9a550625c46a78a93e10369749e576ccba903a63731aec2b0f94ae",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinymcp-0.20.3-ubuntu-24.04-x86_64.tar.gz",
            sha256: "a955333c3c4255db62bd77b8cc54f2a0a510b62a41f1e3f37265aa8da82af390",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinymcp-0.20.3-windows-11-arm64.zip",
            sha256: "33e96f7cb8feb23025e80f8af19d55dc53f9443ffa119370266fc44389e6f660",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinymcp-0.20.3-windows-2022-x86_64.zip",
            sha256: "df48dd51c1f9e4648df2b2e199bdb6cc5defcf9733b9767505f43f2aab23753d",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinymcp-0.20.3-windows-2025-x86_64.zip",
            sha256: "53004de45083fd1b4b8b578ce1b24388c8ac05b2b4f4a761c7885b4d5edbcb88",
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
    version: "0.22.0",
    release_url: "https://github.com/tinyhumansai/tinyconnectors/releases/tag/v0.22.0",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinyconnectors-0.22.0-archlinux-arm64.tar.gz",
            sha256: "0661f0a56ec90a77fe463dc318570b5a1e449409ab315af9194fa7dd9bc8fcf7",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinyconnectors-0.22.0-archlinux-x86_64.tar.gz",
            sha256: "46fd93121bc93e144cb58d76084f477875158321f0b376d6bcc99e7d83f81939",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinyconnectors-0.22.0-fedora-43-arm64.tar.gz",
            sha256: "d1529cc6cb012b1b8822fcc12314a5f8c1fab071253355eafde1a6da5c277199",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinyconnectors-0.22.0-fedora-43-x86_64.tar.gz",
            sha256: "b0ddba48640ad92ccebc986e1fbc700f3cf0459d7b78a6de6101d0e3dd8db035",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinyconnectors-0.22.0-fedora-44-arm64.tar.gz",
            sha256: "255b22015a2429323b05a4713564b1d3253657dcbb39ec2c5fefbaea40f524e4",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinyconnectors-0.22.0-fedora-44-x86_64.tar.gz",
            sha256: "97f011d44f06d7f10d3ed79219cc3dcdb0dd0fa902d93a2e28819adc64f83bc6",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyconnectors-0.22.0-macos-15-arm64.tar.gz",
            sha256: "71e1f21119d461a26bd5b3d58bd6c3a726bc0f12624bb352ea34cc91b0666c82",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyconnectors-0.22.0-macos-15-x86_64.tar.gz",
            sha256: "856e821f651284ae6233fdfefa8f78cba01e1ffc9bd809c09e52e121926daf80",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyconnectors-0.22.0-macos-26-arm64.tar.gz",
            sha256: "8d4f65247b87a37cea3ad3483c1af7084310c9d1c26cba2ab204a7c97d673840",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyconnectors-0.22.0-macos-26-x86_64.tar.gz",
            sha256: "b3f2c2b37123bb55db986706c52ca534105d393454cbb9b78380159dfb3a1bc9",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyconnectors-0.22.0-ubuntu-22.04-arm64.tar.gz",
            sha256: "e797288750066bbbf59846c38d6dd92d6b1f83dfbfdd959b1694f69c9fc7cbb3",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyconnectors-0.22.0-ubuntu-22.04-x86_64.tar.gz",
            sha256: "c6a25a8a7605f0d88e5969b7da7ddd3cb8a6b33abbd9eee380a895e0e992c036",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyconnectors-0.22.0-ubuntu-24.04-arm64.tar.gz",
            sha256: "195d3dd23f8b40fe378e28e41089920c105baa357668a9dc9cf275a3e89134cb",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyconnectors-0.22.0-ubuntu-24.04-x86_64.tar.gz",
            sha256: "7493f7bf799d0e7f9667e21a956aab826141cefad738e2bfd3d71d928b9b4142",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyconnectors-0.22.0-windows-11-arm64.zip",
            sha256: "57d11f8f8f413ce61dc3a4ca0b93f1bde3fb0cd62dcc3a583775be6d5c76323a",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyconnectors-0.22.0-windows-2022-x86_64.zip",
            sha256: "5fb9b593b3545aeeb63dba116a2eb79946aea536182c08a6cabe59632a6d8f69",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyconnectors-0.22.0-windows-2025-x86_64.zip",
            sha256: "15b2a32e869768f58e9205fefee88193f0fc0847080e8ad90b52314905f83aef",
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
