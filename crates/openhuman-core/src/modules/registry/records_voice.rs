//! Registry record for the `tinyvoice` module.

#[cfg(feature = "voice")]
use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

#[cfg(feature = "voice")]
/// The `tinyvoice` module: the host-agnostic half of the voice pipeline.
///
/// Wake-word gating, fast-path command routing, STT hallucination detection,
/// and the capture-side audio work (downmix, resample, silence gate, WAV
/// framing).
///
/// Lazy, and more clearly so than the others: voice is opt-in twice over — a
/// user has to enable dictation or always-on listening before any of this runs
/// — so a session that never speaks should not pay a download or a `dlopen`.
///
/// Devices and audio processing live in the compiled module. Hosts poll bounded
/// capture batches and apply product permission, privacy and transcription policy.
pub(crate) const TINYVOICE: ModuleRecord = ModuleRecord {
    id: "tinyvoice",
    description: "Wake-word gating, command routing, hallucination detection, capture audio",
    bus_name: tinyvoice_bus::names::BUS_NAME,
    object_path: tinyvoice_bus::names::OBJECT_PATH,
    version: "0.3.1",
    release_url: "https://github.com/tinyhumansai/tinyvoice/releases/tag/v0.3.1",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyvoice-module-0.3.1-ubuntu-24.04-x86_64.tar.gz",
            sha256: "777de11fe8f2cdee80648304acd87af80079cda1a63c245a0cc96f67b89d5bd7",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyvoice-module-0.3.1-ubuntu-24.04-arm64.tar.gz",
            sha256: "30cf03c708ef3d768037232fde20299e6979782effd105fd498c091307da189c",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyvoice-module-0.3.1-ubuntu-22.04-x86_64.tar.gz",
            sha256: "0640c5aeab726295d273ec97e8f20b5c4418ae2cf6d7c6936b4a389ce5e23e92",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyvoice-module-0.3.1-ubuntu-22.04-arm64.tar.gz",
            sha256: "fad4e788618bf6b7607da543d56e0ec5396ccd2ca46735f9c04a3014ee7db79f",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyvoice-module-0.3.1-macos-26-arm64.tar.gz",
            sha256: "17548346d4788a731aff976e2ce01b6ba3d11ab531b429110887f0ab968ca331",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyvoice-module-0.3.1-macos-26-x86_64.tar.gz",
            sha256: "303c4dd8a90522582effe021b8965264caf96753ad8a3d9c4d01ec733b569d6f",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyvoice-module-0.3.1-macos-15-arm64.tar.gz",
            sha256: "514588cd04dc44fc5b9acea91c36a0190c9e7c4d041b0e356107d9750279c463",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyvoice-module-0.3.1-macos-15-x86_64.tar.gz",
            sha256: "31a3219b53b72666bd4fe06a99ec07ee9b11f7402ca0234a4bbd13a7eef961c4",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyvoice-module-0.3.1-windows-2025-x86_64.zip",
            sha256: "c361cf0fac55cad45f11f704ddbf13183edbb665b05a21802fb3cf9f7c3809ae",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyvoice-module-0.3.1-windows-2022-x86_64.zip",
            sha256: "766087bd7aa0406b85c169b207a1e78b40cf2a58235b39e357d04028ed905f2c",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyvoice-module-0.3.1-windows-11-arm64.zip",
            sha256: "b63c2f3b47a97b36cc9de7c56a82988ff7d9734143ff0c1d892dae395cdfcbd3",
        },
    ],
    load: LoadPolicy::Lazy,
    source: SourceSpec {
        submodule: "tinyvoice",
        manifest: "crates/tinyvoice-module/Cargo.toml",
        package: "tinyvoice-module",
        lib_name: "tinyvoice_module",
        features: &[],
    },
};
