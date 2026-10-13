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
    version: "0.2.0",
    release_url: "https://github.com/tinyhumansai/tinymemes/releases/tag/v0.2.0",
    assets: &[
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinymemes-module-0.2.0-macos-15-arm64.tar.gz",
            sha256: "a45604e66dccc4c8248625802993ab1b006f09b1a75adf032d4d38632d5267e3",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinymemes-module-0.2.0-macos-15-x86_64.tar.gz",
            sha256: "6483c103b9917f52116387ee200d53402cf9da92da6d9b2d5c83a6accebffbc6",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinymemes-module-0.2.0-macos-26-arm64.tar.gz",
            sha256: "cb988214f5a425ccae93d0e620e3bb4ae903b45a62aa4cfa38a597415f0751cd",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinymemes-module-0.2.0-macos-26-x86_64.tar.gz",
            sha256: "4684d8db160f641ab77c51ecd85d9b5f0628a273fd887458f87b90102621aa52",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinymemes-module-0.2.0-ubuntu-22.04-arm64.tar.gz",
            sha256: "d78634c208af6373122025523b0ae823eb723c164f699457aa5850c3f2fc5e54",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinymemes-module-0.2.0-ubuntu-22.04-x86_64.tar.gz",
            sha256: "ccd814cef8b8ec6e55dbfb57d053ec3675f24c0c23fc07e3aa2a8e5ce17f48fe",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinymemes-module-0.2.0-ubuntu-24.04-arm64.tar.gz",
            sha256: "0f0be32e3e46072ffd0ce53cd982463af4cb4649e57a7d376c3dc49f8ff21e81",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinymemes-module-0.2.0-ubuntu-24.04-x86_64.tar.gz",
            sha256: "c1eed5ba2bcf8b0e5ce02fca8163020ee8709c8efe7f345d55863321c7dcaf62",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinymemes-module-0.2.0-windows-11-arm64.zip",
            sha256: "4fa51df9fc8f61b4b7a3255e52f363bcd658e1296f7bcf382b8d616d8290e4de",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinymemes-module-0.2.0-windows-2022-x86_64.zip",
            sha256: "7ae4f69e0c88c6435c24762342a22b356ede4a7935370c2ceb7f3f156c5089c9",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinymemes-module-0.2.0-windows-2025-x86_64.zip",
            sha256: "09e43b36f20c0746389b6a607966253a36100bd562d7bd9535f91979751e42a5",
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
