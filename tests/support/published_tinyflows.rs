//! Digest gate for tests that load the published TinyFlows v0.8.27 module.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const V0_8_27_MODULE_HASHES: &[&str] = &[
    "505ea178c0a9c34809f9b2c50f85496d17c60f33b7050fe7899a45045a11ee1c",
    "caa5cd3065b97e55d90d7e44269c2a1037070099cb96660d7ac12ce54f7a6664",
    "3a8421906ca9e15c9efb955b264d4a92ef88c10fda081ccb9c6636a203bba17d",
    "eb6421123118d332d2e86ad427c2477d979f1c8153231ca3561daaeb6fc88492",
    "a175f30274cc4246104066cef701a12acdbb6d438ecfa98e40bfcdf06f3ff334",
    "01a1e879289d89beb45c24b980f7864cbe96ef15a2c1a1988cd85f1ab4181cb4",
    "091df8d832faef0b06348aecda53adce939ad5dab3cfcd1e2a4b629a09c0e03a",
    "f8eb80928f2ea769b8f9a5f03c3fbf6d61940273b5c4a0af5fa05a9f0d64475b",
    "8f3e4d87835c92a21f1b843dcbdc1e4e6c0beac06e837a6fca3c75834017b996",
    "edd24bcc25c2d2a95de6b9f4356c4e19a2c50ad634148d485b59040f6e81d1d3",
    "12a72926b08d28139b7856c0b125dc08a7c7002a8e021a8eec7b082087620e77",
];

/// Finds the staged platform artifact and confirms it matches a published v0.8.27 bundle.
pub fn staged_v0_8_27_module() -> (PathBuf, String) {
    let artifact = openhuman_core::modules::local_artifact("tinyflows")
        .expect("stage the published TinyFlows v0.8.27 module on OPENHUMAN_MODULE_PATH");
    let digest = verify_v0_8_27_module(&artifact);
    (artifact, digest)
}

/// Confirms `artifact` is one of the public platform bundles for v0.8.27.
pub fn verify_v0_8_27_module(artifact: &Path) -> String {
    let digest = hex::encode(Sha256::digest(
        std::fs::read(artifact).expect("read staged TinyFlows native artifact"),
    ));
    assert!(
        V0_8_27_MODULE_HASHES.contains(&digest.as_str()),
        "staged TinyFlows library is not a published v0.8.27 module: {} sha256={digest}",
        artifact.display()
    );
    digest
}
