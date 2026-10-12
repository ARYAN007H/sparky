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
    version: "0.19.0",
    release_url: "https://github.com/tinyhumansai/tinywallet/releases/tag/v0.19.0",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinywallet-module-0.19.0-ubuntu-24.04-x86_64.tar.gz",
            sha256: "55c27a4c85631aee0509e9c74ae1eceb5f3fb3aa43540294bdd4991eac5500af",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinywallet-module-0.19.0-ubuntu-24.04-arm64.tar.gz",
            sha256: "aa79e84f60892f2ff624f42619f928b2699bdbea613f7bc2a0595f769c7f1d68",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinywallet-module-0.19.0-ubuntu-22.04-x86_64.tar.gz",
            sha256: "4511bf4a03f9163f5493472c1b294eccd8087e44efbde6b7e50c2ee4867b707c",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinywallet-module-0.19.0-ubuntu-22.04-arm64.tar.gz",
            sha256: "d981e334615d4a4d2ea003a4c3d59ee8f631d6f52532cfa70dc97143e97a6fe8",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinywallet-module-0.19.0-macos-26-arm64.tar.gz",
            sha256: "bff93e7c33fcf663c0b69117add0e249c461233a58eb5ba2f1b753a9a8944350",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinywallet-module-0.19.0-macos-26-x86_64.tar.gz",
            sha256: "72e1ef5c1047c0ac848abb9cd1cf69e6639ac4e8527496979406922852a4d2d3",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinywallet-module-0.19.0-macos-15-arm64.tar.gz",
            sha256: "15397175e39dffbfe372bb40a4dccd6e03440ed5925043d41df058a335e1427d",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinywallet-module-0.19.0-macos-15-x86_64.tar.gz",
            sha256: "389e68114a4d1243afe04e8143e7f6ebd321c1cc24b54be2d768c7ee8643c7b1",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinywallet-module-0.19.0-windows-2025-x86_64.zip",
            sha256: "17856ed57fb14840c3a4c5cde9167a68d9a8d578ee63728852b894d3d1e8fec1",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinywallet-module-0.19.0-windows-2022-x86_64.zip",
            sha256: "398bce97e33fa44faa3e92eb20cdd448089882233858a952b7710ad4d1806546",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinywallet-module-0.19.0-windows-11-arm64.zip",
            sha256: "f91ef12a88c324a4c890be0ff8114239d835770541ce8cfd6059f015d3720764",
        },
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinywallet-module-0.19.0-archlinux-arm64.tar.gz",
            sha256: "59863d52205fabe486a16dfbf573a007d19c3237236cee3a8ff236281cdf6ba9",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinywallet-module-0.19.0-archlinux-x86_64.tar.gz",
            sha256: "645679737effe617f566af7349dc7153f29dac21fa7a377a610ccf72054e7454",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinywallet-module-0.19.0-fedora-43-arm64.tar.gz",
            sha256: "c05ff2da4101246b7a50a37a44632a8502a5747f5e7cc416258fb728782d91b6",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinywallet-module-0.19.0-fedora-43-x86_64.tar.gz",
            sha256: "c4399bbb1c9d3c42545fb78474f548337cfc4a96ebb49ce50912ca4c85dd3aa4",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinywallet-module-0.19.0-fedora-44-arm64.tar.gz",
            sha256: "d17075dd175d0bd3bebe31350a3bcc58320c698f4c54f69691f85dd54297fdb9",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinywallet-module-0.19.0-fedora-44-x86_64.tar.gz",
            sha256: "ef64e56e536a2661ae0ddcd799632819c02f391a5e7f98431adc184f7df45adb",
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
