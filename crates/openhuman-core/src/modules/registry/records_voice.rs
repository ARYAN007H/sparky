//! Registry record for the `tinyvoice` module.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset};

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
    bus_name: "ai.tinyhumans.tinyvoice.Voice",
    object_path: "/ai/tinyhumans/tinyvoice/Voice",
    version: "0.1.15",
    release_url: "https://github.com/tinyhumansai/tinyvoice/releases/tag/v0.1.15",
    assets: &[
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyvoice-module-0.1.15-ubuntu-24.04-x86_64.tar.gz",
            sha256: "3d56893a20cdb50d305f1d35d535d026a3acd9d92931afc01349f131bc7108e8",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyvoice-module-0.1.15-ubuntu-24.04-arm64.tar.gz",
            sha256: "397409478b8739670f2bd256e9b29120015e366df11189a7484e464b4c0531df",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyvoice-module-0.1.15-ubuntu-22.04-x86_64.tar.gz",
            sha256: "412501462ee421e065507023b94451a10559435955954f317969849a4efc8bd9",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyvoice-module-0.1.15-ubuntu-22.04-arm64.tar.gz",
            sha256: "daa9a5c660a3ec5d6dd3ae9ad08842060383d8c4421bf58934fa070d34c90fa1",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyvoice-module-0.1.15-macos-26-arm64.tar.gz",
            sha256: "c25aa3250062f6d02a9442047ea6cb9ab50919923a08bcafea08819a160dd4c7",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyvoice-module-0.1.15-macos-26-x86_64.tar.gz",
            sha256: "747c2e62f08233cf6657863b1ff9696a0bb9d166126fdde7d717b3c6dac385a1",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyvoice-module-0.1.15-macos-15-arm64.tar.gz",
            sha256: "16ef3ee328fb2c51506e8f66c2f3ef5e66b87b3f9505d85834b67bbc3e4b98ff",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyvoice-module-0.1.15-macos-15-x86_64.tar.gz",
            sha256: "1e819f986c59849649c53d75f1a2488a148c1fda3659cd967eaf53d63f2c492b",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyvoice-module-0.1.15-windows-2025-x86_64.zip",
            sha256: "0cd3476aba36d6aefb7f318a45cbdf5a47efd0668e9a28425aaef2d5bfd72048",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyvoice-module-0.1.15-windows-2022-x86_64.zip",
            sha256: "ebbde34149317cb306ace5cc61c01abdaa601cecb339c3f2a517f5441858ca46",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyvoice-module-0.1.15-windows-11-arm64.zip",
            sha256: "2701528845188b10f7bcb6b05fc5354d0b733a257325da39b06c68fa1c995398",
        },
    ],
    load: LoadPolicy::Lazy,
};
