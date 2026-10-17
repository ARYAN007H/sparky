//! TinyFlows module: scheduled job runtime and workflow authoring through its bus contract.
use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

pub(crate) const TINYFLOWS: ModuleRecord = ModuleRecord {
    id: "tinyflows",
    description: "Scheduled jobs and workflow authoring through TinyFlows",
    bus_name: tinyflows_bus::BUS_NAME,
    object_path: tinyflows_bus::OBJECT_PATH,
    version: "0.8.24",
    release_url: "https://github.com/tinyhumansai/tinyflows/releases/tag/v0.8.24",
    // Verbatim from the published v0.8.24 checksum.toml.
    assets: &[
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyflows-module-0.8.24-macos-26-arm64.tar.gz",
            sha256: "c2af0e6c1188e3e7cb832539419897fef28045fb39d96abbb86de315afacc57c",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyflows-module-0.8.24-macos-26-x86_64.tar.gz",
            sha256: "d0519abfba02708e684951ede6864d380ffce8136e51b0efe9520660948f928a",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyflows-module-0.8.24-macos-15-arm64.tar.gz",
            sha256: "07b13fb23aa53f8b12ffc2275e40c8d98cc0297b14aace3445f28892316cd4f3",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyflows-module-0.8.24-macos-15-x86_64.tar.gz",
            sha256: "29b08d245c0ecfffbdeb4966cd38a9e4c4c9d014f06cbaec0c8b22499e629063",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyflows-module-0.8.24-ubuntu-24.04-x86_64.tar.gz",
            sha256: "1d864ef3c45aa5b918addfafff6755e750e94701c910b1a8c64d70ffe5fbb14e",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyflows-module-0.8.24-ubuntu-24.04-arm64.tar.gz",
            sha256: "e7279ae70e6dd90533b2c9e4f7a2497a1979c05fa7db472b1abdeb4f3b4209cc",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyflows-module-0.8.24-ubuntu-22.04-x86_64.tar.gz",
            sha256: "7794cc2693ea5c46728676939468269b73af56135944d74425c813e135b88180",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyflows-module-0.8.24-ubuntu-22.04-arm64.tar.gz",
            sha256: "37831b640f577d7e1ce5f1e70288cd536d4282a2d6b01d2efed1a5414863a1d6",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyflows-module-0.8.24-windows-2025-x86_64.zip",
            sha256: "25a44e6c31cb98bfcc5209091eedeaec7c861a02a5e6c5aeb69ae4d82c77ae30",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyflows-module-0.8.24-windows-2022-x86_64.zip",
            sha256: "77b3d1333d5eefb296df6745c7308c3d8b3e2c75cd74d690939c6a850d5ba782",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyflows-module-0.8.24-windows-11-arm64.zip",
            sha256: "f7f31f05154c5a6cdc726d31f8d28c33674ac53453a3c28c42ec758f813cbaf9",
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
