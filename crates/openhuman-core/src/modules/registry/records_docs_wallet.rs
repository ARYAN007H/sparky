//! Registry records for the `tinydocs` and `tinywallet` modules.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

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
    version: "0.2.2",
    release_url: "https://github.com/tinyhumansai/tinydocs/releases/tag/v0.2.2",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinydocs-module-0.2.2-ubuntu-24.04-x86_64.tar.gz",
            sha256: "2b306aa16ae103575ae71326caf66deccd20123e8eaef5a098b72cb96bcd0853",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinydocs-module-0.2.2-ubuntu-24.04-arm64.tar.gz",
            sha256: "6d7c478fd5c9da86e526ec59868a3d0fec14352fc4fa294a828790757e020c2a",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinydocs-module-0.2.2-ubuntu-22.04-x86_64.tar.gz",
            sha256: "ed36393921c3f289454d2ef2441e792f03548c3cba38f94e69cfa07ee21b967e",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinydocs-module-0.2.2-ubuntu-22.04-arm64.tar.gz",
            sha256: "e819f6a8fd63667f11e3e8d9402198ecf477ebf3891674fe6f8c9f58e59398f7",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinydocs-module-0.2.2-macos-26-arm64.tar.gz",
            sha256: "076e732a88f14557203cc03737e401bfc44028c59041d819b6ea94143835d07b",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinydocs-module-0.2.2-macos-26-x86_64.tar.gz",
            sha256: "ba5992c8e3036cebc21c0e29ba96541aebc34807426d4587f29b1a703bde09a0",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinydocs-module-0.2.2-macos-15-arm64.tar.gz",
            sha256: "9e4aa34bb39dadbc1d8dd52e25b339e2e79bd16420b022a2ae27d09e6750d113",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinydocs-module-0.2.2-macos-15-x86_64.tar.gz",
            sha256: "d2e055f0bdff577c9a14d29668751dcca1208243c2c63b00a806237aac478273",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinydocs-module-0.2.2-windows-2025-x86_64.zip",
            sha256: "8aa6f230a1b456146914383ab64fbdceccde02c493f7f78e26fec201f6c59bbd",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinydocs-module-0.2.2-windows-2022-x86_64.zip",
            sha256: "7c318d43f2989b3e73972cd58b03087763114457d3805cb33d78ef1604a834cb",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinydocs-module-0.2.2-windows-11-arm64.zip",
            sha256: "171ea11a6eb11ca67f228f46a0e3818fb9b360344e5004c6b913c9d55d44a15e",
        },
    ],
    load: LoadPolicy::Lazy,
    source: SourceSpec {
        submodule: "tinydocs",
        manifest: "crates/tinydocs-module/Cargo.toml",
        package: "tinydocs-module",
        lib_name: "tinydocs_module",
        features: &[],
    },
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
/// The pinned v0.19.2 release carries the native wallet, Web3, and x402 bus
/// operations, including typed configured-endpoint references and native
/// Solana transfer facts verified before execution. Secret-bearing operations
/// remain confidential and require both attestation and a matching pinned
/// digest.
pub(crate) const TINYWALLET: ModuleRecord = ModuleRecord {
    id: "tinywallet",
    description: "Transaction building and assembly for Bitcoin, EVM, Solana and Tron",
    bus_name: "ai.tinyhumans.tinywallet.Wallet",
    object_path: "/ai/tinyhumans/tinywallet/Wallet",
    version: "0.19.2",
    release_url: "https://github.com/tinyhumansai/tinywallet/releases/tag/v0.19.2",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinywallet-module-0.19.2-ubuntu-24.04-x86_64.tar.gz",
            sha256: "0116da89b2031532b6eefa60f0d77335514c77dd7b5c49c6f556c180b3b24780",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinywallet-module-0.19.2-ubuntu-24.04-arm64.tar.gz",
            sha256: "c68d3b8b2257377aed4d1b852a1eb28c26b4a7c34fb7eb7359720f098dc6216e",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinywallet-module-0.19.2-ubuntu-22.04-x86_64.tar.gz",
            sha256: "016c5e6efc1bb63ce60572edc1ffd61699d3006601d5dc52e51604a91a225bf8",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinywallet-module-0.19.2-ubuntu-22.04-arm64.tar.gz",
            sha256: "9311690732558c3347ff4a3bbe4a29a09cfd999ce53561080d58f3963d143a54",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinywallet-module-0.19.2-macos-26-arm64.tar.gz",
            sha256: "f5e36e43bf05321e0fa503f591e1f51471eb6ee06bc6dc39a2857a35aeecc24f",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinywallet-module-0.19.2-macos-26-x86_64.tar.gz",
            sha256: "37845264453470cdfee9fea998d4664e1ab948a566e1da2ea585fac53d13b9eb",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinywallet-module-0.19.2-macos-15-arm64.tar.gz",
            sha256: "4ab4d3b350c2a6684f06dc67f7eb8c54957a5b15390d7b37fb4c38a5a3d3fb6a",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinywallet-module-0.19.2-macos-15-x86_64.tar.gz",
            sha256: "2bb7d195371382030c5894e14a57f5a792e3e41b36246bd754174e26b250bb94",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinywallet-module-0.19.2-windows-2025-x86_64.zip",
            sha256: "c16c1071651cd493dd56af5fdf63992c8667b8becaeac2ad0be4f32a7b6c493a",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinywallet-module-0.19.2-windows-2022-x86_64.zip",
            sha256: "99a36e75a71212b22991564d10e18e0bbe3c1c0acae2d17bbe95df14eb6d2bdb",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinywallet-module-0.19.2-windows-11-arm64.zip",
            sha256: "977e2051dfadc29a7a66884198ac25fc6d4b2d375d45a174889158c6ca7e15f4",
        },
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinywallet-module-0.19.2-archlinux-arm64.tar.gz",
            sha256: "df51cab1066c4ea5b6964343a136769b53b61de392a8dcf56f9455b5370a72af",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinywallet-module-0.19.2-archlinux-x86_64.tar.gz",
            sha256: "e7b0717c82a530b2ca0f44c55938d66f19bb98b0f314032cf8f3336cf23840cc",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinywallet-module-0.19.2-fedora-43-arm64.tar.gz",
            sha256: "c42776335a866fda439fc2d05ac70c6dd120fa0d56359d910bf5fe58e51b354b",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinywallet-module-0.19.2-fedora-43-x86_64.tar.gz",
            sha256: "4a26baaabad59fa96c0a36b766d8352d8eb72a09d188f7313b136379db34cf73",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinywallet-module-0.19.2-fedora-44-arm64.tar.gz",
            sha256: "6ac6974fd601959eb8b8726d99abe64389445d53b71f263f5af58e05877ffbb5",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinywallet-module-0.19.2-fedora-44-x86_64.tar.gz",
            sha256: "7589757b00ce3e01e88f01c4d479a6ef2795bf3f361dab3972cde51939e6ef97",
        },
    ],
    load: LoadPolicy::Lazy,
    source: SourceSpec {
        submodule: "tinywallet",
        manifest: "crates/tinywallet-module/Cargo.toml",
        package: "tinywallet-module",
        lib_name: "tinywallet_module",
        features: &[],
    },
};
