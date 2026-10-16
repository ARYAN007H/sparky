//! TinyFlows module: the scheduler runtime, the cron job and run store, and the
//! schedule maths behind `tinyflows_bus::schedule` (contract v3), plus workflow
//! authoring. Published digests are added only from its release manifest.
use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

pub(crate) const TINYFLOWS: ModuleRecord = ModuleRecord {
    id: "tinyflows",
    description:
        "Scheduled jobs (runtime, job store, schedule maths) and workflow authoring through TinyFlows",
    bus_name: tinyflows_bus::BUS_NAME,
    object_path: tinyflows_bus::OBJECT_PATH,
    version: "0.8.22",
    release_url: "https://github.com/tinyhumansai/tinyflows/releases/tag/v0.8.22",
    // Verbatim from the published v0.8.22 checksum.toml; the same host set as
    // the other native modules (macOS, Ubuntu, Windows).
    assets: &[
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyflows-module-0.8.22-macos-26-arm64.tar.gz",
            sha256: "c61b9847e26368cdf70ebfda2fd53f0a7e4847655e4fc47b4342b42f7550da14",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyflows-module-0.8.22-macos-26-x86_64.tar.gz",
            sha256: "e548e5ca9923a4ec05293e215300ff2c2481d4628d95f9f8216b1faddbca54aa",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyflows-module-0.8.22-macos-15-arm64.tar.gz",
            sha256: "7520988f152fab5c3c0bc28bf18269d5dc64d44d6ef3f2d8c369cd331e12d30a",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyflows-module-0.8.22-macos-15-x86_64.tar.gz",
            sha256: "4eaedf98348b3c6b63906e0f905c984c5cb134758bbaf57f3e20cc3a556be1c6",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyflows-module-0.8.22-ubuntu-24.04-x86_64.tar.gz",
            sha256: "2556fe259e9f6041cb667e08a888a1ba74789967559bf8598664aeb328afb5ce",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyflows-module-0.8.22-ubuntu-24.04-arm64.tar.gz",
            sha256: "8ee3475419faa0102dbd854995c267313344d2ca9d19527bf1b115acae3ed0f9",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyflows-module-0.8.22-ubuntu-22.04-x86_64.tar.gz",
            sha256: "cc6d07cc50d30c9118ed9f0a55671cd79279353659b25e7ef3ffcf05b367b3a0",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyflows-module-0.8.22-ubuntu-22.04-arm64.tar.gz",
            sha256: "7fa2dbc61f7d9332dc72e283805263f4b3579695c6c6669c8ab5fdbe6cdc2f10",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyflows-module-0.8.22-windows-2025-x86_64.zip",
            sha256: "33117f79ae5ed20dd0aab8d34dd6d511606609ef57d66e2f908301f7a585c634",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyflows-module-0.8.22-windows-2022-x86_64.zip",
            sha256: "c7b40743a7173b0bf01b697d5dcbb02145a4202c4d334ed8558f04051312592d",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyflows-module-0.8.22-windows-11-arm64.zip",
            sha256: "53fff97e890d7fd7f80d2f3dd8f60a9d12db38b28df593a7ec9007df5b25a742",
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
