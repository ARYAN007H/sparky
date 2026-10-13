//! Registry record for the `tinymemes` module.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

/// The `tinymemes` module: final-reply slang and meme remixing, loaded on
/// first use. The host serves the callbacks it asks for (inference, Jev,
/// search, index storage, delivery).
pub(crate) const TINYMEMES: ModuleRecord = ModuleRecord {
    id: "tinymemes",
    description: "Final-reply slang and meme remixing through TinyMemes",
    bus_name: tinymemes_bus::STATE_INTERFACE,
    object_path: tinymemes_bus::STATE_PATH,
    version: "0.3.0",
    release_url: "https://github.com/tinyhumansai/tinymemes/releases/tag/v0.3.0",
    assets: &[
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinymemes-module-0.3.0-macos-15-arm64.tar.gz",
            sha256: "f461bdf0757e4b991da764a723e375ca5bd54997e848b454b8935fde0473ed50",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinymemes-module-0.3.0-macos-15-x86_64.tar.gz",
            sha256: "d25345d6094fa8c5717f0f19dd8dcc322c9fecb5570a9032a6f0bdadf6b4459c",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinymemes-module-0.3.0-macos-26-arm64.tar.gz",
            sha256: "ac98997b8a839b4bf07113e17490a3cf5253c17c861557218f1b2b63ae20ff76",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinymemes-module-0.3.0-macos-26-x86_64.tar.gz",
            sha256: "866c54e66397c4fd815d814479e15d4b5bb24a86839196fc152677ab46563640",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinymemes-module-0.3.0-ubuntu-22.04-arm64.tar.gz",
            sha256: "46e9e553b88e7f6a36e667e36b3bf2a0d4f2d12e750ddbd86ea7be4508488e55",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinymemes-module-0.3.0-ubuntu-22.04-x86_64.tar.gz",
            sha256: "fdab4d9869650af6141f04f008e5442eb5629fe522b901e12be03ec1128dd64a",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinymemes-module-0.3.0-ubuntu-24.04-arm64.tar.gz",
            sha256: "c7a45498d94c437632664ecdab250cc0e04741c3548dbb74a1028b8d50d08425",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinymemes-module-0.3.0-ubuntu-24.04-x86_64.tar.gz",
            sha256: "4ddebe94d749edb1e97a8e9a086b3056398ffef0c81ec1ecf3ade16ac692d670",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinymemes-module-0.3.0-windows-11-arm64.zip",
            sha256: "38ec450fb7af348deec1e12241c0799c018b7602a3075f40062f7f101eaa6a7c",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinymemes-module-0.3.0-windows-2022-x86_64.zip",
            sha256: "0392a9db7f1de397360c800ccfd71ad57ff7b1f442f05335bb483ed63bcdde01",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinymemes-module-0.3.0-windows-2025-x86_64.zip",
            sha256: "9f44359a93661505318a4237d2d8705e18301e8651671c81368984c2a4eed7e5",
        },
    ],
    load: LoadPolicy::Lazy,
    source: SourceSpec {
        submodule: "tinymemes",
        manifest: "crates/tinymemes-module/Cargo.toml",
        package: "tinymemes-module",
        lib_name: "tinymemes_module",
        features: &[],
    },
};
