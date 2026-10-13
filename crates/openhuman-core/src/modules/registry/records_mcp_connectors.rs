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
    version: "0.18.1",
    release_url: "https://github.com/tinyhumansai/tinymcp/releases/tag/v0.18.1",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-rolling-arm64",
            archive: "tinymcp-0.18.1-archlinux-arm64.tar.gz",
            sha256: "63e062c14993fef556d770ba977bc932ddf7843e44fbbb6487036356713b0be1",
        },
        PlatformAsset {
            host_key: "archlinux-rolling-x86_64",
            archive: "tinymcp-0.18.1-archlinux-x86_64.tar.gz",
            sha256: "6fcd6daf2dc56ee9f8cc5465d13465e195c3c0e6a3675475f7bdb1395542ca38",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinymcp-0.18.1-fedora-43-arm64.tar.gz",
            sha256: "4dd2b74371d8dbc4d6860eb07c9e5033185ff8e79f63df14df029109f695272f",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinymcp-0.18.1-fedora-43-x86_64.tar.gz",
            sha256: "b99bf1911458e8fe2ff498c02ae68390d0d4ef5fcecb60a9199ae3f0a0d12800",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinymcp-0.18.1-fedora-44-arm64.tar.gz",
            sha256: "491c2d27998a7edf392af2ca4c98c81bc318efce4bc4c9d6fda5cb70ff5ca967",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinymcp-0.18.1-fedora-44-x86_64.tar.gz",
            sha256: "f5c225716ea256bb1714ff507e38abad5355e4ad768a559537e609cf29951aea",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinymcp-0.18.1-macos-15-arm64.tar.gz",
            sha256: "594a6ec5b05f0f85e456787f234b25ccf09cf47f7109b73277270582c3caedde",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinymcp-0.18.1-macos-15-x86_64.tar.gz",
            sha256: "ded08986747d819de9e71c5582eafa790d7f92c474cd8be1880ea7b97d191205",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinymcp-0.18.1-macos-26-arm64.tar.gz",
            sha256: "0f98865ec04c5b862ceeda8f87362fc1b5db98280a095cb5cddd1a34531a5240",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinymcp-0.18.1-macos-26-x86_64.tar.gz",
            sha256: "8c6cb2daf759056b7dc24036dcd89ba9012f24a0bb6fd7bf73d2cef323c7198b",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinymcp-0.18.1-ubuntu-22.04-arm64.tar.gz",
            sha256: "07eaf68ab296f8de059e90988160d5c6c29accad4830f5b228d3f932df000372",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinymcp-0.18.1-ubuntu-22.04-x86_64.tar.gz",
            sha256: "2e7a1890281ec795978e046a56162acc751d239d9baf985d3c05655d7edeee9b",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinymcp-0.18.1-ubuntu-24.04-arm64.tar.gz",
            sha256: "db44d547ffd4a22b572e2033e9c93f87315891107525b46e4deec79fadc72865",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinymcp-0.18.1-ubuntu-24.04-x86_64.tar.gz",
            sha256: "bf518e7f6d9f9b110ee191dde68dc4fe9f192e8a51523e2b8dfbc835cb6ecf55",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinymcp-0.18.1-windows-11-arm64.zip",
            sha256: "57024764c301d6cec0bca39af7da3edfd4d64105264280f6f33e05eeff07fb60",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinymcp-0.18.1-windows-2022-x86_64.zip",
            sha256: "fc74f8d217ddafd2394677b2bb0cd601b82230c6e3702994b095056a65dc10c5",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinymcp-0.18.1-windows-2025-x86_64.zip",
            sha256: "6ac5ea9cb2822d19261a6609ad65ef040114c61b99218496882cab10bf4b0e1b",
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
