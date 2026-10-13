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
    version: "0.2.4",
    release_url: "https://github.com/tinyhumansai/tinydocs/releases/tag/v0.2.4",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinydocs-module-0.2.4-ubuntu-24.04-x86_64.tar.gz",
            sha256: "2e9d3772a4434ea825914788f865476bc532fdc885097b35a95211202716fd9e",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinydocs-module-0.2.4-ubuntu-24.04-arm64.tar.gz",
            sha256: "9b63791a363a25f3f26d2bb155bb782b2ceb95c2f74ce6910fc650c2614c5a86",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinydocs-module-0.2.4-ubuntu-22.04-x86_64.tar.gz",
            sha256: "ac2f0aa79ae402103726b6f5adf23f90762071e23576e5dc6701293d49ad9427",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinydocs-module-0.2.4-ubuntu-22.04-arm64.tar.gz",
            sha256: "ff8bb22f796fcab37a0a4345f7e09b64cae99d81a5622f86cd940729f83994af",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinydocs-module-0.2.4-macos-26-arm64.tar.gz",
            sha256: "5b6105f050812c3331bdac8a52dbac21c734c5239b96642b4c2bd26dcd3fa727",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinydocs-module-0.2.4-macos-26-x86_64.tar.gz",
            sha256: "2b8eab7e244e720afa47525e4207e77cbbceb77cea416defe8be2a0c0abf661a",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinydocs-module-0.2.4-macos-15-arm64.tar.gz",
            sha256: "1038a30888e002b9c8cd1083be36b13fc99903a847060ad1b690eb0c80b6081b",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinydocs-module-0.2.4-macos-15-x86_64.tar.gz",
            sha256: "3ced33ab2ef2279848f1db10e6387cb557f3fab9f1221b25724c4fbc3709a9b2",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinydocs-module-0.2.4-windows-2025-x86_64.zip",
            sha256: "1b7fefa4a704f38dec8221182ca9f94f07e5dceaf8b66bd8acc72876fe39cb0f",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinydocs-module-0.2.4-windows-2022-x86_64.zip",
            sha256: "2145b031a68ac2b62daccfda2c723781073db38130e6dbed42055fb21dd23933",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinydocs-module-0.2.4-windows-11-arm64.zip",
            sha256: "2e2b91ea44f03cd026487cfbf94b7e3e3ea8525bb05b14d31bec56a8eb8ce4d9",
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
/// The pinned v0.20.0 release (contract 1.14) carries the native wallet, Web3,
/// and x402 bus operations, including mnemonic word-count validation. Secret-bearing
/// operations remain confidential and require both attestation and a matching
/// pinned digest.
pub(crate) const TINYWALLET: ModuleRecord = ModuleRecord {
    id: "tinywallet",
    description: "Transaction building and assembly for Bitcoin, EVM, Solana and Tron",
    bus_name: tinywallet_bus::names::BUS_NAME,
    object_path: tinywallet_bus::names::OBJECT_PATH,
    version: "0.22.0",
    release_url: "https://github.com/tinyhumansai/tinywallet/releases/tag/v0.22.0",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinywallet-module-0.22.0-archlinux-arm64.tar.gz",
            sha256: "c0a7911452b1bf1461c945d45170305122dcb791c0ea3378411b1332cc4e7013",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinywallet-module-0.22.0-archlinux-x86_64.tar.gz",
            sha256: "f15aa217405a41a7d4b53462e2f9f3e3745184ec1de9e54f79deca54a4162177",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinywallet-module-0.22.0-fedora-43-arm64.tar.gz",
            sha256: "c9a92837299aa2a43883341390ecceb54e2e8567c23fe9963be51077d9726c1e",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinywallet-module-0.22.0-fedora-43-x86_64.tar.gz",
            sha256: "a19486e72204f0150466e06372538f70b761ddd7ff49fab5bd736f7bc48581a4",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinywallet-module-0.22.0-fedora-44-arm64.tar.gz",
            sha256: "5ac8b865224d229d3d7bd199cd651e3862d4658fd125aae4121a0a0e709edde2",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinywallet-module-0.22.0-fedora-44-x86_64.tar.gz",
            sha256: "d404e7e484d09835c843731ddbbc5a93354071d8ed900792e42f73e56c4ba13f",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinywallet-module-0.22.0-macos-15-arm64.tar.gz",
            sha256: "1a173add7bd67565583f0cd557584dd79b2d64b9330aca93a5ded27446fc24c7",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinywallet-module-0.22.0-macos-15-x86_64.tar.gz",
            sha256: "99a6d43a5f9668b36c5d98ec51daf3c343515af870dbf4745abdc6cf4ef2cfe7",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinywallet-module-0.22.0-macos-26-arm64.tar.gz",
            sha256: "4da19b9a12677db4187ef2076a44341066bf96976dbe89845fff32ff7990b0dd",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinywallet-module-0.22.0-macos-26-x86_64.tar.gz",
            sha256: "a00ec245f3b72810e7c25d513bb5976dc9ada07265f8bdec2217d217049c4399",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinywallet-module-0.22.0-ubuntu-22.04-arm64.tar.gz",
            sha256: "c37c0c477e89b38013cbe6e8d8f62ddff3cf08c94e0452f441ae041265f3e7ab",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinywallet-module-0.22.0-ubuntu-22.04-x86_64.tar.gz",
            sha256: "df6c0b4ad6b560242186357f10d29f490ac278fb2f94c6f80934c8723f64130d",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinywallet-module-0.22.0-ubuntu-24.04-arm64.tar.gz",
            sha256: "c31bf29e7f6004be268e484651621a8e2c9efc3221a3d91285318888079dbb41",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinywallet-module-0.22.0-ubuntu-24.04-x86_64.tar.gz",
            sha256: "d0ef3d6833c972c18a917f965b71e7df61087c6eb61496ff6af9ca76605325e2",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinywallet-module-0.22.0-windows-11-arm64.zip",
            sha256: "02969de77cd63c2d889dc65231de6327fa39b4b39ffbacee2c4524dc34f44512",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinywallet-module-0.22.0-windows-2022-x86_64.zip",
            sha256: "401cc74695b52a40806713ece3f31c971d23ee3cd0529b231aa36a809cb015a0",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinywallet-module-0.22.0-windows-2025-x86_64.zip",
            sha256: "880c52be4634c0693b64769d58462605075954430b36a7dc7fff7dc49ecdf4fe",
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
