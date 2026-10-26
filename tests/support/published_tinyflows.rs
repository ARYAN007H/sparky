//! Digest gate for tests that load the published TinyFlows v0.9.3 module.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const V0_9_3_MODULE_HASHES: &[&str] = &[
    // tinyflows-module-0.9.3-macos-15-arm64.tar.gz
    "2f357cdbf2c8084b83f119ca4e437a65eb5646463a594c1c94f8f1cdf78bb7a1",
    // tinyflows-module-0.9.3-macos-15-x86_64.tar.gz
    "e1478ccf87aa76a658daeb57fe8ad9e3b5e03acc92fd546edf8b24ae484ce562",
    // tinyflows-module-0.9.3-macos-26-arm64.tar.gz
    "1e405f7f54cf52b70649c2bdfed601e34840caba275787c4f45f37aabaee5909",
    // tinyflows-module-0.9.3-macos-26-x86_64.tar.gz
    "1560979b76adccc764fbd825a66b7451b46c3e1ece6e98e2fcd34ab813b396fa",
    // tinyflows-module-0.9.3-ubuntu-22.04-arm64.tar.gz
    "68dfbc3cca78e9f056dbbec798dee8ded5ad4ab85dce34fb77bc5ab582c5e2b9",
    // tinyflows-module-0.9.3-ubuntu-22.04-x86_64.tar.gz
    "3a4b172b900b3005d5530e12723177c9796800ae0645b83ae67a15d306deab43",
    // tinyflows-module-0.9.3-ubuntu-24.04-arm64.tar.gz
    "a07c7ec7051c83c4099c7cf2481e7b27ed02171a515a51df90e4a04a3e710dee",
    // tinyflows-module-0.9.3-ubuntu-24.04-x86_64.tar.gz
    "879048f5665eab35bf6758e23837efcaa9670e99266403e6effd2157dbd4fced",
    // tinyflows-module-0.9.3-windows-11-arm64.zip
    "72484cb37a3e0d58fc85639f4d04e4e6cfb14e16326d6162bb16922aefbb8c1a",
    // tinyflows-module-0.9.3-windows-2022-x86_64.zip
    "e5bd42cf8c7c8d756bf69a699151fb45e13511920673fce422e48f2548766719",
    // tinyflows-module-0.9.3-windows-2025-x86_64.zip
    "a5232f185173b04fa2f8c5892b8aa4501669b0a5c0deec2adf9879ef627bc178",
];

/// Finds the staged platform artifact and confirms it matches a published v0.9.3 bundle.
pub fn staged_v0_9_3_module() -> (PathBuf, String) {
    let artifact = openhuman_core::modules::local_artifact("tinyflows")
        .expect("stage the published TinyFlows v0.9.3 module on OPENHUMAN_MODULE_PATH");
    let digest = verify_v0_9_3_module(&artifact);
    (artifact, digest)
}

/// Confirms `artifact` is one of the public platform bundles for v0.9.3.
pub fn verify_v0_9_3_module(artifact: &Path) -> String {
    let digest = hex::encode(Sha256::digest(
        std::fs::read(artifact).expect("read staged TinyFlows native artifact"),
    ));
    assert!(
        V0_9_3_MODULE_HASHES.contains(&digest.as_str()),
        "staged TinyFlows library is not a published v0.9.3 module: {} sha256={digest}",
        artifact.display()
    );
    digest
}
