//! TinyFlows module: the scheduler runtime, the cron job and run store, and the
//! schedule maths behind `tinyflows_bus::schedule` (contract v3), plus workflow
//! authoring. Published digests are added only from its release manifest.
use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

// TODO(repin v0.8.22): the pins below are the v0.8.21 `checksum.toml` (schedule
// contract v2). The host speaks contract v3 (`tinyflows-bus` at the vendored
// gitlink), so release tinyflows v0.8.22 and run
// `node scripts/modules/release-and-repin.mjs tinyflows` (or paste the new
// `checksum.toml` verbatim) before this ships. Until then a `release` source
// answers the v3-only members (`NextRun`, `ValidateSchedule`, ...) with
// unknown-method errors; `OPENHUMAN_MODULE_SOURCE=source` / `linked` serve the
// gitlink build.
pub(crate) const TINYFLOWS: ModuleRecord = ModuleRecord {
    id: "tinyflows",
    description:
        "Scheduled jobs (runtime, job store, schedule maths) and workflow authoring through TinyFlows",
    bus_name: tinyflows_bus::BUS_NAME,
    object_path: tinyflows_bus::OBJECT_PATH,
    version: "0.8.21",
    release_url: "https://github.com/tinyhumansai/tinyflows/releases/tag/v0.8.21",
    // Verbatim from the published v0.8.21 checksum.toml; the same host set as
    // the other native modules (macOS, Ubuntu, Windows).
    assets: &[
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyflows-module-0.8.21-macos-26-arm64.tar.gz",
            sha256: "9d67fa849c1b9c910db6a5bf33436d7a64e4b85a99364f6f996fd75dca90730d",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyflows-module-0.8.21-macos-26-x86_64.tar.gz",
            sha256: "d43ccfc00caae221bf290c67170a03fbe15b1275e34837d7cebf7dc9833f4fa9",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyflows-module-0.8.21-macos-15-arm64.tar.gz",
            sha256: "ab5986d9f60969197eff22213fa72dd61ab53ad6c0172be2975101aec9957cc9",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyflows-module-0.8.21-macos-15-x86_64.tar.gz",
            sha256: "773955db768e99d68646992aca7e413362c509e12f28467c1832925681282293",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyflows-module-0.8.21-ubuntu-24.04-x86_64.tar.gz",
            sha256: "5be120f8148757fb60640a8bef6c41d4613c5f539d3b6b1b4837c809c17d7b45",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyflows-module-0.8.21-ubuntu-24.04-arm64.tar.gz",
            sha256: "d8fb392cf2dadf8b9cbc9fdcaa4c6a1d192ec1fe0c835bb614a72f89ce3a937a",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyflows-module-0.8.21-ubuntu-22.04-x86_64.tar.gz",
            sha256: "6997dd8c519e3fa1115f2692614a7c1ea0c64bd8cf014c3dec52f76b38f3fcc3",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyflows-module-0.8.21-ubuntu-22.04-arm64.tar.gz",
            sha256: "98e088bc0604f59678b12ac34349bcdc269afaac5c0ddee4b5b3d457df8f1780",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyflows-module-0.8.21-windows-2025-x86_64.zip",
            sha256: "e28bab1c01028e15fa60ff62f4acf8d15dc1fd75f11009bf374c8f9ca596d8b8",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyflows-module-0.8.21-windows-2022-x86_64.zip",
            sha256: "bdc017298948a108a3d444f7ee1b8c60c4b1e919834d3f2a4e170fa33c4d120f",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyflows-module-0.8.21-windows-11-arm64.zip",
            sha256: "71d53323f32d253929960825f01daefa4af78b5918ab72feb3a1193ffd1ccdd1",
        },
    ],
    load: LoadPolicy::Lazy,
    source: SourceSpec {
        submodule: "tinyflows",
        manifest: "crates/tinyflows-module/Cargo.toml",
        package: "tinyflows-module",
        lib_name: "tinyflows_module",
        features: &[],
    },
};
