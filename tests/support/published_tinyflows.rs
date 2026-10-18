//! Digest gate for tests that load the published TinyFlows v0.8.26 module.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const V0_8_26_MODULE_HASHES: &[&str] = &[
    "39996055002225d4819fff1c7d2cc6254ba9626398c36e9e14764f936466ee0c",
    "2c84ecd7f719317a2bb982266398e462387f184cfeb3efd6c51a7f6e933ca27f",
    "15a6272c6fff593ba463288d977ca5579314cf2d7e54b611d3ba18706621a38b",
    "25bfa6a09824abd08f34933071812860d2bc591ab7efacecbd6332e591269581",
    "869e0779aaba9cb53d257010371abbaa39c7f0959e37adbc6b3a5405ba9809fa",
    "e30709b829a756e61bbb798b67456fb85cafa4682824eaa5a058cf9b076d2bf4",
    "3daf156281200da96281e746ea008f226fbb5e34b7b60651cb90dbf0c0c30a9d",
    "7c994b91e38043e7c58be208857e7bfcaa7d1bfb4075ab5b4b7906d5fdc9d9bd",
    "e491f23f7b88bf62ba443c4ea41755dbba01d540890a22ab83ff144ec1260954",
    "deb068e630730659e73f6de5a48e6260f181b1bb8421c2b790ad69f94841a818",
    "2e41d87a0951bafda28d6f26c4c339c41503144d873a0be6c9052263ceedeb6e",
];

/// Finds the staged platform artifact and confirms it matches a published v0.8.26 bundle.
pub fn staged_v0_8_26_module() -> (PathBuf, String) {
    let artifact = openhuman_core::modules::local_artifact("tinyflows")
        .expect("stage the published TinyFlows v0.8.26 module on OPENHUMAN_MODULE_PATH");
    let digest = verify_v0_8_26_module(&artifact);
    (artifact, digest)
}

/// Confirms `artifact` is one of the public platform bundles for v0.8.26.
pub fn verify_v0_8_26_module(artifact: &Path) -> String {
    let digest = hex::encode(Sha256::digest(
        std::fs::read(artifact).expect("read staged TinyFlows native artifact"),
    ));
    assert!(
        V0_8_26_MODULE_HASHES.contains(&digest.as_str()),
        "staged TinyFlows library is not a published v0.8.26 module: {} sha256={digest}",
        artifact.display()
    );
    digest
}
