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
    version: "0.2.0",
    release_url: "https://github.com/tinyhumansai/tinyvoice/releases/tag/v0.2.0",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyvoice-module-0.2.0-ubuntu-24.04-x86_64.tar.gz",
            sha256: "7993badf940bfbc988c078f9c1ba40af29a6e4177ead8680b78e7a98545cc5e4",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyvoice-module-0.2.0-ubuntu-24.04-arm64.tar.gz",
            sha256: "d9cfea6d2a910e98f6609dbca65d3bbdcc3950f40615545a0ce14d9117ce34cd",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyvoice-module-0.2.0-ubuntu-22.04-x86_64.tar.gz",
            sha256: "57330db85525148a78352882181ab7008fd79c18528289ed43f11cdae4f01c31",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyvoice-module-0.2.0-ubuntu-22.04-arm64.tar.gz",
            sha256: "e59e98a6844e6e202da9df539d63dedc0cb2c7c733b8cd91ff6fc6b9a2baf9a0",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyvoice-module-0.2.0-macos-26-arm64.tar.gz",
            sha256: "7a7babe46af562e9833d59a594f66babe76bdc0a7b925754d04bde8579bb1299",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyvoice-module-0.2.0-macos-26-x86_64.tar.gz",
            sha256: "a29df2511d63eec3c1cb97bdee2a13a7da94f3c53f5f36cb217c271363e6c2fa",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyvoice-module-0.2.0-macos-15-arm64.tar.gz",
            sha256: "340e799db919f145382b372c45d237ef1edbf69afa2fa0ab976eccd850b62f03",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyvoice-module-0.2.0-macos-15-x86_64.tar.gz",
            sha256: "f5bd0ab69258b5fda58074768c959df41f49ed1fda34a8c2d32027b7feb0348e",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyvoice-module-0.2.0-windows-2025-x86_64.zip",
            sha256: "57c802f2e1c07c66aae10de26013f3ddb6f02d5309fb6ea5f0580d048501bd86",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyvoice-module-0.2.0-windows-2022-x86_64.zip",
            sha256: "15190a7cd8a3d49d4b4c9b4e3e39247fa936ecea9a7d5f7becd2911e97a8636b",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyvoice-module-0.2.0-windows-11-arm64.zip",
            sha256: "d7be3ffd3e6bd2d3deb7f334d447250cb31228f49dfec86e3a24c4d3e4d93754",
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
