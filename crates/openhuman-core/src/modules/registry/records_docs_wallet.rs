//! Registry records for the `tinydocs` and `tinywallet` modules.

#[cfg(any(feature = "documents", feature = "web3"))]
use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

#[cfg(feature = "documents")]
/// The `tinydocs` module: document synthesis, bounded extraction, image facts,
/// and PDF rendering.
///
/// Lazy, because a user who never asks for a document should not pay a download,
/// a `dlopen`, and the resident cost of a library that is never unloaded.
pub(crate) const TINYDOCS: ModuleRecord = ModuleRecord {
    id: "tinydocs",
    description: "Document synthesis, Office/PDF extraction, image inspection, and PDF rendering",
    bus_name: tinydocs_bus::names::BUS_NAME,
    object_path: tinydocs_bus::names::OBJECT_PATH,
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

#[cfg(feature = "web3")]
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
/// The pinned v0.19.0 release carries the native wallet, Web3, and x402 bus
/// operations, including typed configured-endpoint references. Secret-bearing
/// operations remain confidential and require both attestation and a matching
/// pinned digest.
pub(crate) const TINYWALLET: ModuleRecord = ModuleRecord {
    id: "tinywallet",
    description: "Transaction building and assembly for Bitcoin, EVM, Solana and Tron",
    bus_name: tinywallet_bus::names::BUS_NAME,
    object_path: tinywallet_bus::names::OBJECT_PATH,
    version: "0.20.0",
    release_url: "https://github.com/tinyhumansai/tinywallet/releases/tag/v0.20.0",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinywallet-module-0.20.0-ubuntu-24.04-x86_64.tar.gz",
            sha256: "6c092e41f8053851c5d4919e3c8ff6445cac4a9f8526889aa40a500a77ae40ba",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinywallet-module-0.20.0-ubuntu-24.04-arm64.tar.gz",
            sha256: "1ca76d32b4d66c68b31963f4aa9cfdc6585276fff660d4ab29726dbb29e04499",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinywallet-module-0.20.0-ubuntu-22.04-x86_64.tar.gz",
            sha256: "8712d00897266d0e2b0889ac76fe4da12d1917ffb8e2b3fcc8eb95c1c6f8570e",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinywallet-module-0.20.0-ubuntu-22.04-arm64.tar.gz",
            sha256: "cce338941314005f54584e567dc5d6c2f9a8a4189c4b47908ead8ceed19ae328",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinywallet-module-0.20.0-macos-26-arm64.tar.gz",
            sha256: "65955f1a0c921da47e1a2cecc7ba4f19cd775c536f7039878dd6c60c3804b017",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinywallet-module-0.20.0-macos-26-x86_64.tar.gz",
            sha256: "5fa92699fed3cb00c57f29c04fa101d94b24f85aedae91f1b6776d57f7162604",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinywallet-module-0.20.0-macos-15-arm64.tar.gz",
            sha256: "496e61f6ce4a431ee77bf730a713c3d1c8d76b6666f99d15972f7f62623e632f",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinywallet-module-0.20.0-macos-15-x86_64.tar.gz",
            sha256: "427b9882eb1ef52be3ba28ed9e292790c60a1f769148b3bbafe4ef5389e1144a",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinywallet-module-0.20.0-windows-2025-x86_64.zip",
            sha256: "c56914add49d2529dc4620e1c1a4846961d16da9b4d8fb9005c3d6d23989d51f",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinywallet-module-0.20.0-windows-2022-x86_64.zip",
            sha256: "80d5ae3272ae0d32fcc1fcb727573554dffcc34f793d35c9b2603d0a4f8cdb28",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinywallet-module-0.20.0-windows-11-arm64.zip",
            sha256: "171172d90020470dfbe83d3be480b0d6127b123694fdbf600218acc7cba6f929",
        },
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinywallet-module-0.20.0-archlinux-arm64.tar.gz",
            sha256: "04641e9a3e69a28d7b29b7055cd6b6f1e710f117e37392426a22eb1867e1a0c8",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinywallet-module-0.20.0-archlinux-x86_64.tar.gz",
            sha256: "50dbe4f6b296941971712eacc831a9ff5947dfaca3be664687e6a1ae120f9b9d",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinywallet-module-0.20.0-fedora-43-arm64.tar.gz",
            sha256: "3802fd3681bfeac1ce90e34c98600258993cc832fa89285895aabea22f0d2b81",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinywallet-module-0.20.0-fedora-43-x86_64.tar.gz",
            sha256: "9767aa07631a03adaed05430c03726f4e19e16b3561f1fc149264eb52332dbc5",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinywallet-module-0.20.0-fedora-44-arm64.tar.gz",
            sha256: "3974790d265b775f968e5dcaa7e4206d08da52187cad09e438cc278aba9eb540",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinywallet-module-0.20.0-fedora-44-x86_64.tar.gz",
            sha256: "83ff87f0d71985e63ac73c95944e534286fde1a04b0a7daa499753e1fe943269",
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
