//! Registry records for the `tinydocs` and `tinywallet` modules.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset};

/// The `tinydocs` module: document synthesis, bounded extraction, image facts,
/// and PDF rendering.
///
/// Lazy, because a user who never asks for a document should not pay a download,
/// a `dlopen`, and the resident cost of a library that is never unloaded.
pub(crate) const TINYDOCS: ModuleRecord = ModuleRecord {
    id: "tinydocs",
    description: "Document synthesis, Office/PDF extraction, image inspection, and PDF rendering",
    bus_name: "ai.tinyhumans.tinydocs.Documents",
    object_path: "/ai/tinyhumans/tinydocs/Documents",
    version: "0.2.0",
    release_url: "https://github.com/tinyhumansai/tinydocs/releases/tag/v0.2.0",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinydocs-module-0.2.0-ubuntu-24.04-x86_64.tar.gz",
            sha256: "2db0db216f6bd25e07d66cd13485672dd2edcd84bed9c4f5ee383f603ced7c95",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinydocs-module-0.2.0-ubuntu-24.04-arm64.tar.gz",
            sha256: "9d77120bceac02ac607192f12884023c8f10a18baecef18acb3694ecb8b42217",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinydocs-module-0.2.0-ubuntu-22.04-x86_64.tar.gz",
            sha256: "d3a84df5f375a9774183dd5c1fd344ac373752b54473a6d2fb611c7cab5142d2",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinydocs-module-0.2.0-ubuntu-22.04-arm64.tar.gz",
            sha256: "fa094051f186e085db8bdc304215aa5553c0f29c958b78cc337623815e62c39a",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinydocs-module-0.2.0-macos-26-arm64.tar.gz",
            sha256: "94553eef6029b61ad135d406000e7ccfb03857e68be03fed990d8912b9b1995f",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinydocs-module-0.2.0-macos-26-x86_64.tar.gz",
            sha256: "b01bba79f64731534ca93c89d045cfdd8cd8d6ce34249810a99e76735e4e3b59",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinydocs-module-0.2.0-macos-15-arm64.tar.gz",
            sha256: "b92f0b31d942f56784bf9fd6897736595f5c1bf39db34a7115e5d9ddafb0fe72",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinydocs-module-0.2.0-macos-15-x86_64.tar.gz",
            sha256: "2f6ff91ff580462a234e7476bb0dc3c6fda975786e2ea920b0a85ece3df760d8",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinydocs-module-0.2.0-windows-2025-x86_64.zip",
            sha256: "e799aca9485cff97eceb4a3f8f6882c5e92abb17e639a68d2f774fcf57f035c2",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinydocs-module-0.2.0-windows-2022-x86_64.zip",
            sha256: "5a271fa3a9c472cdd531d259db92dc04512199ac1cd586fdf31c3af14c3299fb",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinydocs-module-0.2.0-windows-11-arm64.zip",
            sha256: "15541820df3ff69621485846d328495d42d06e7d9b2e73c42eea5af2242a9a4c",
        },
    ],
    load: LoadPolicy::Lazy,
};

/// The `tinywallet` module: native wallet, Web3 quote, and x402 operations.
///
/// Lazy for the same reason as [`TINYDOCS`], and more so: most sessions never
/// touch a wallet, and this artifact carries `bitcoin` and a native `secp256k1`
/// build that would otherwise be resident for all of them.
///
/// **This host sends it the recovery phrase, over confidential calls, and never
/// derives or signs itself.** All four chains — Bitcoin, EVM, Solana and Tron —
/// derive and sign inside the module. Web3 quotes and x402 payments, budgets,
/// and ledger state also live there. The host supplies credential custody,
/// authenticated transports, and exact-facts approval callbacks. This binary
/// depends on `tinywallet-bus` for the wire contract rather than implementation
/// crates.
///
/// The phrase is only sent to a module tinybus has attested *and* whose digest
/// matches one of the entries below — `super::wallet::attested_proxy` checks
/// this table itself rather than trusting that some check happened.
///
/// The contract also exposes `ExportKey` for downstream hosts that must drive
/// a signer locally; OpenHuman itself does not call it.
///
/// The pinned v0.14.1 release carries the native wallet, Web3, and x402 bus
/// operations, including typed configured-endpoint references. Secret-bearing operations remain confidential and require both
/// attestation and a matching pinned digest.
pub(crate) const TINYWALLET: ModuleRecord = ModuleRecord {
    id: "tinywallet",
    description: "Transaction building and assembly for Bitcoin, EVM, Solana and Tron",
    bus_name: "ai.tinyhumans.tinywallet.Wallet",
    object_path: "/ai/tinyhumans/tinywallet/Wallet",
    version: "0.14.1",
    release_url: "https://github.com/tinyhumansai/tinywallet/releases/tag/v0.14.1",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinywallet-module-0.14.1-ubuntu-24.04-x86_64.tar.gz",
            sha256: "4165b555daaa88305aedffc58aaa05cbb9349b2024ae88d1f27b85497d12f34c",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinywallet-module-0.14.1-ubuntu-24.04-arm64.tar.gz",
            sha256: "36ef748e79eca9b3ef38b8fbc76f1cd084ee68b1db776d783ee9ec0be00fa1eb",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinywallet-module-0.14.1-ubuntu-22.04-x86_64.tar.gz",
            sha256: "313f6d7fc9e744236ec7a923070b8a3267798b4834a501935dfe108dde5cf3e8",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinywallet-module-0.14.1-ubuntu-22.04-arm64.tar.gz",
            sha256: "3f47a5eaa3db37a3ecb846ba93d83630a6c62188224248557b230f60dbfa3000",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinywallet-module-0.14.1-macos-26-arm64.tar.gz",
            sha256: "06154b2fe6fb222e2332a8891c7384acb0b42b45e442d9832932351cd1d95bcc",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinywallet-module-0.14.1-macos-26-x86_64.tar.gz",
            sha256: "d51d954794620fc106c5bf90ca67b2f35b5fcf7d54752c777af14e1c4189ede2",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinywallet-module-0.14.1-macos-15-arm64.tar.gz",
            sha256: "aa0e77605cd7da33aed7f3c535f6dca3d9eadc0ba01b7030d5d675097562c502",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinywallet-module-0.14.1-macos-15-x86_64.tar.gz",
            sha256: "25cbba1d1898d3bfdff2f7d7d8bbccb3a8d55bc2b25fcb5e800bbbb3cbbc63e4",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinywallet-module-0.14.1-windows-2025-x86_64.zip",
            sha256: "628b67ebf7e52dc2c40bb58e09b56af6cd1bb4698634297b15023bbfd092eb78",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinywallet-module-0.14.1-windows-2022-x86_64.zip",
            sha256: "b11b0a92daba13b2fa1afe0c9cefb5632c085a3ddea61ef5477b97cfd335d6b0",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinywallet-module-0.14.1-windows-11-arm64.zip",
            sha256: "7459aec5c7f0e0e8ab0c3dd11461938b0d7a99c8ba7e9a0ee4beaeb894db8907",
        },
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinywallet-module-0.14.1-archlinux-arm64.tar.gz",
            sha256: "4e8eddcedbab8dd7f4d03ffdba9d1f8f7f00003240ff55abb9b22381d4ebcc00",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinywallet-module-0.14.1-archlinux-x86_64.tar.gz",
            sha256: "1e267ec642d89fdc9a630d6ab963abdd1d311fea054b2fae5a2b1055be8261bc",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinywallet-module-0.14.1-fedora-43-arm64.tar.gz",
            sha256: "df9f1ad060cb4282a369f8c353e2859b88f92dc5511b328922f14efd7411d6bd",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinywallet-module-0.14.1-fedora-43-x86_64.tar.gz",
            sha256: "4b178adbd9d1ff7e59d9066299f5cd8fef9d686f8a35d1afed6410162fa1a0bd",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinywallet-module-0.14.1-fedora-44-arm64.tar.gz",
            sha256: "e3da4a7f315d3d4d90563a37ae72d617dbd4c2f0042f835222e7fa86e04eb82e",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinywallet-module-0.14.1-fedora-44-x86_64.tar.gz",
            sha256: "d7ddd11719ab9b6431d6a4c264d6bd8d24764037f2724cb6e280b714793db6fc",
        },
    ],
    load: LoadPolicy::Lazy,
};
