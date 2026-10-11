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
/// The pinned v0.17.0 release carries the native wallet, Web3, and x402 bus
/// operations, including typed configured-endpoint references. Secret-bearing
/// operations remain confidential and require both attestation and a matching
/// pinned digest.
pub(crate) const TINYWALLET: ModuleRecord = ModuleRecord {
    id: "tinywallet",
    description: "Transaction building and assembly for Bitcoin, EVM, Solana and Tron",
    bus_name: "ai.tinyhumans.tinywallet.Wallet",
    object_path: "/ai/tinyhumans/tinywallet/Wallet",
    version: "0.17.0",
    release_url: "https://github.com/tinyhumansai/tinywallet/releases/tag/v0.17.0",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinywallet-module-0.17.0-ubuntu-24.04-x86_64.tar.gz",
            sha256: "36f52398fe78de4401277e8071c37352872bedb2949c123288c10fddee38c76b",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinywallet-module-0.17.0-ubuntu-24.04-arm64.tar.gz",
            sha256: "b57b18ded530946c5308794f9f52ae6dff2c47ee9930913d77a93bd77e375369",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinywallet-module-0.17.0-ubuntu-22.04-x86_64.tar.gz",
            sha256: "115881175089340f3acd59e8387342ea8dbc7189775b4879daeba35abb46fe97",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinywallet-module-0.17.0-ubuntu-22.04-arm64.tar.gz",
            sha256: "a6e740acd889550b8e4f2995087372403eed390edf5fb3b6f3d64b4ee8790a71",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinywallet-module-0.17.0-macos-26-arm64.tar.gz",
            sha256: "615385e65c54c50f098ff3bc2089f5a2f3e5696074866d6d1b13ae05881b0a6f",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinywallet-module-0.17.0-macos-26-x86_64.tar.gz",
            sha256: "6328678f0c2a987b3ccc87311f424306652cbcabbcf8d5fc30e4253b23910bc8",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinywallet-module-0.17.0-macos-15-arm64.tar.gz",
            sha256: "138b050186ec8a207d0e176d4fca06a9ff356296e7db227a3e6a2bdaa103fb41",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinywallet-module-0.17.0-macos-15-x86_64.tar.gz",
            sha256: "a8fe61bb53df771c6d062fb3d1b27062eeb7020ea2a2daea63789c54c2d804be",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinywallet-module-0.17.0-windows-2025-x86_64.zip",
            sha256: "4fc71d2516fd31225ba75bc800882c0680e2260fff200e25e35904532c0f85ed",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinywallet-module-0.17.0-windows-2022-x86_64.zip",
            sha256: "5f0a2c56b6e49e194fdd5e88153ff2cfbbaf293a476ee728844d9e5bb56a9754",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinywallet-module-0.17.0-windows-11-arm64.zip",
            sha256: "5782da496c32ae1ec30271c80e033fe5b7009b46be47ccbe30ac02242ed5347b",
        },
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinywallet-module-0.17.0-archlinux-arm64.tar.gz",
            sha256: "da0e1ef4885e1607c0ad0e7df2728b041b9dfc0df669c7bb2249f11ad863d687",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinywallet-module-0.17.0-archlinux-x86_64.tar.gz",
            sha256: "f3e0512bfa5f01e9426ebac9c2566d2cc583f7949b3fa1ca8501096342b327ee",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinywallet-module-0.17.0-fedora-43-arm64.tar.gz",
            sha256: "85f7a7a8da15923d8a48e926784c10782917073cda40321086dfa7f7dc58b3d6",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinywallet-module-0.17.0-fedora-43-x86_64.tar.gz",
            sha256: "cc427cec94b5bfbedcb9146259bde6a0b70c965c8d052f0e5292cdd35749eefe",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinywallet-module-0.17.0-fedora-44-arm64.tar.gz",
            sha256: "688fe8d557437cc767784eac28edbb1364b797abd773d74b37c9115b8fe85822",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinywallet-module-0.17.0-fedora-44-x86_64.tar.gz",
            sha256: "63737c9b0d80b9f333e36796446e4471d1217798daf6f43d47abc24b83dcf8ff",
        },
    ],
    load: LoadPolicy::Lazy,
};
