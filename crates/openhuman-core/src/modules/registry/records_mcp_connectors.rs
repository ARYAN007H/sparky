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
    version: "0.17.0",
    release_url: "https://github.com/tinyhumansai/tinymcp/releases/tag/v0.17.0",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-rolling-arm64",
            archive: "tinymcp-0.17.0-archlinux-arm64.tar.gz",
            sha256: "88c65f478ecf53aa37f5248121745fe310d9b1576a66ffa521ccb980aca635cc",
        },
        PlatformAsset {
            host_key: "archlinux-rolling-x86_64",
            archive: "tinymcp-0.17.0-archlinux-x86_64.tar.gz",
            sha256: "5c3d094ad4745a465ced1b7df972c5eea01765e0f4d3593e976745ca94b6a605",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinymcp-0.17.0-fedora-43-arm64.tar.gz",
            sha256: "933afa785a361df8b87e019fab437754fbd848743e988b7770841102a5bfc95a",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinymcp-0.17.0-fedora-43-x86_64.tar.gz",
            sha256: "f70327181078492876918a5f71be757b744787f6c3193581d5a3e074b1273401",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinymcp-0.17.0-fedora-44-arm64.tar.gz",
            sha256: "9f66da175a9af69dbe3b7ebee12cd80aa3479161c155571392c60565da48fefe",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinymcp-0.17.0-fedora-44-x86_64.tar.gz",
            sha256: "136a0e83f7b74dc3d0ba73f90c2a1da0beae141737c92ddcb82299449b6ef9a7",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinymcp-0.17.0-macos-15-arm64.tar.gz",
            sha256: "477d9f30f3e597d6e49d6e8d4f529ebfd0e905886c603408b5a6802e08633e37",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinymcp-0.17.0-macos-15-x86_64.tar.gz",
            sha256: "bb247d2962b13569aee3a62707f29f8b6f5856aff3decb0822d7895d07b00150",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinymcp-0.17.0-macos-26-arm64.tar.gz",
            sha256: "d0d850fc380f630d18a5b8252e10547fc71b803cc7c390bf8b919ba1daca27c6",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinymcp-0.17.0-macos-26-x86_64.tar.gz",
            sha256: "7a86fafa1508bbc60697350c47e7030ac9023d0167ec3587dcbae5b1bab99127",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinymcp-0.17.0-ubuntu-22.04-arm64.tar.gz",
            sha256: "d5eded8771352505ac7b9293ed4775a441b1b8c8c51ea1b95db4be63ca845176",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinymcp-0.17.0-ubuntu-22.04-x86_64.tar.gz",
            sha256: "0ca32f8a2dc4b68cf4fb44ad63268076ff3f15cfd2dd6786bff512db2c44c3f8",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinymcp-0.17.0-ubuntu-24.04-arm64.tar.gz",
            sha256: "1feea2871e4eea2f56287705897f26732e443c97aef5c6aabcf59e21f8d32c6f",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinymcp-0.17.0-ubuntu-24.04-x86_64.tar.gz",
            sha256: "338db73b3fa17df8d15cb09a6788048150f6a12a076f0d148995e9307eb66f24",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinymcp-0.17.0-windows-11-arm64.zip",
            sha256: "1b1613eaf4861fd98cc5321bc7c01851c151eac16395f6d8accfac1eb96f5d90",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinymcp-0.17.0-windows-2022-x86_64.zip",
            sha256: "b88290610640406e09be9d188e2751b2aea1ad35b2456b69da360e0ab431dc39",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinymcp-0.17.0-windows-2025-x86_64.zip",
            sha256: "4f8893e48ec0b2258b5093312d282499653769e5417cc4060c53542a0cf710a7",
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
