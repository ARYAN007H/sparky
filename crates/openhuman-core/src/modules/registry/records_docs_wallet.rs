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
    version: "0.23.1",
    release_url: "https://github.com/tinyhumansai/tinywallet/releases/tag/v0.23.1",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinywallet-module-0.23.1-archlinux-arm64.tar.gz",
            sha256: "63f561a54ac72dd5d05b5bb4ed88d0dd75d57957d4ba77529f72611fd73eb958",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinywallet-module-0.23.1-archlinux-x86_64.tar.gz",
            sha256: "bf47d50a2d9c9d33a65048af7fddfeb5863b130e4581ecb2b062454d58e4c873",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinywallet-module-0.23.1-fedora-43-arm64.tar.gz",
            sha256: "c0f656035bf491da0d2f628409cba85a35d37922056b62d0dc699fd32549f3b2",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinywallet-module-0.23.1-fedora-43-x86_64.tar.gz",
            sha256: "b6c89126b043a0dc6dca26e1a901d0359933a84a6076be6e4624a76fd761f832",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinywallet-module-0.23.1-fedora-44-arm64.tar.gz",
            sha256: "6b874de4e364c0899f8687a7188286b2fc4982a843f7bd87abf22feb9746cd9f",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinywallet-module-0.23.1-fedora-44-x86_64.tar.gz",
            sha256: "6b1203f11f605fe3f4ef06b6247b8a6b93e6f2ea66ec931a74f4897b55318505",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinywallet-module-0.23.1-macos-15-arm64.tar.gz",
            sha256: "fb0b18a0050be7915708b5d19b128d0068194e59b215ce3dffafef0d7224de8b",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinywallet-module-0.23.1-macos-15-x86_64.tar.gz",
            sha256: "f0939b246dd958b0a679c3915cfae8fa7069e09e1a83ceb06b74af6ccea4f1b8",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinywallet-module-0.23.1-macos-26-arm64.tar.gz",
            sha256: "7d3038ef40cdb160fbfd7bdbd0825594ada1f7eb098546c3d48d3ec2594f37d7",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinywallet-module-0.23.1-macos-26-x86_64.tar.gz",
            sha256: "f780cb7169e8116c595d5bdaae52d6a650d7ae9dc11be2df0a3f48c74640d30f",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinywallet-module-0.23.1-ubuntu-22.04-arm64.tar.gz",
            sha256: "2aebd102b08c16d4a0b2b50f26afc1024522d7a3d3baf5944ad52d388e2c298c",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinywallet-module-0.23.1-ubuntu-22.04-x86_64.tar.gz",
            sha256: "ff16c343dac63ad54b71a82c378533059799257e2448ee1938fd4610f7949ffd",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinywallet-module-0.23.1-ubuntu-24.04-arm64.tar.gz",
            sha256: "56b715ff30e132a55ab15b0215780cecaeb4190aca7e41a2db270e0ec6bfc2b4",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinywallet-module-0.23.1-ubuntu-24.04-x86_64.tar.gz",
            sha256: "3ebe2672fe5c275c58ee00425719da879940a51f4ad9baa7dde3d94276bdad7b",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinywallet-module-0.23.1-windows-11-arm64.zip",
            sha256: "cbd1e60e0e64844f45b2d3d13fbc7b55e4634b309b06db6f992671af6555a105",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinywallet-module-0.23.1-windows-2022-x86_64.zip",
            sha256: "8b2c6128f25a630793b7ac8c57c0821aa06f434f3bf3350137f9fa61d7d03508",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinywallet-module-0.23.1-windows-2025-x86_64.zip",
            sha256: "7e83f7d0fcbca31bb5efd529adaa7cae3031f94aba192918afe4f690abc293f1",
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
