use super::*;

#[test]
fn tinychannels_registry_matches_scoped_runtime_release() {
    let record = find("tinychannels").expect("compiled TinyChannels module");
    assert_eq!(record.version, "0.1.43");
    assert_eq!(tinychannels_bus::CONTRACT_VERSION, 33);
    assert_eq!(
        tinychannels_bus::module_relay::MODULE_OPERATION_VERSION,
        (1, 25)
    );
    let expected = [
        (
            "archlinux-arm64",
            "tinychannels-module-0.1.43-archlinux-arm64.tar.gz",
            "4b9d759be6ded94e07884ea4cd5a92ec12211b6fd274ff958b24b816af43d3a5",
        ),
        (
            "archlinux-x86_64",
            "tinychannels-module-0.1.43-archlinux-x86_64.tar.gz",
            "b68bf10234ed7b3bb5b2ae9f57ceb3377e65facca47450c2c3542094f0a7f883",
        ),
        (
            "fedora-43-arm64",
            "tinychannels-module-0.1.43-fedora-43-arm64.tar.gz",
            "a9f331f5768dad8370e63002084efc1bef2f1e6c3bcbe8b063e8df8a1519e09a",
        ),
        (
            "fedora-43-x86_64",
            "tinychannels-module-0.1.43-fedora-43-x86_64.tar.gz",
            "ef807b7c271bc5c35ac9a91029275d4511f31d13c053bb5999413aff542a4a77",
        ),
        (
            "fedora-44-arm64",
            "tinychannels-module-0.1.43-fedora-44-arm64.tar.gz",
            "fa37cde5a84986babde4916a61b5ed38380a7d68ad08fb0a519d24195a5d8886",
        ),
        (
            "fedora-44-x86_64",
            "tinychannels-module-0.1.43-fedora-44-x86_64.tar.gz",
            "c6d2acd0334ccffb465bc420bc558ed30edbbb8f4d6d03d8b5409ca1ecce5388",
        ),
        (
            "macos-15-arm64",
            "tinychannels-module-0.1.43-macos-15-arm64.tar.gz",
            "57748164f05e662d4cc6b01d52a1dfd1524fb114e85c382dd9c067cdbef098b5",
        ),
        (
            "macos-15-x86_64",
            "tinychannels-module-0.1.43-macos-15-x86_64.tar.gz",
            "850c369dfe6001dfc8d76b4f1f39866c62e4b74d5e2a03eeb4ce90a035a33e15",
        ),
        (
            "macos-26-arm64",
            "tinychannels-module-0.1.43-macos-26-arm64.tar.gz",
            "542a209ed3f1fff22811a2f8e1150e44471e28350da40613573ca548f3a8ddbf",
        ),
        (
            "macos-26-x86_64",
            "tinychannels-module-0.1.43-macos-26-x86_64.tar.gz",
            "4be169ce377ffd8b30761b473f9ea9cd6574769c9720934f6ae1b268963a7a47",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinychannels-module-0.1.43-ubuntu-22.04-arm64.tar.gz",
            "3025c71abf61c2a339898563a36d1a7280b15d2a44dcdca27551594751197fda",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinychannels-module-0.1.43-ubuntu-22.04-x86_64.tar.gz",
            "98ae46d1b6e7869bf7e569ddd26c4b1acc488679593a475435d43db00d8e391f",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinychannels-module-0.1.43-ubuntu-24.04-arm64.tar.gz",
            "e1f1c77712467bee8397c7cb9445f7e3c8a138998405955c64407cb03e68eb6b",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinychannels-module-0.1.43-ubuntu-24.04-x86_64.tar.gz",
            "da2d6283331e35e485779ca1a7f6d62b4f84fef9a8cc151f5b76dd508fa565e3",
        ),
        (
            "windows-11-arm64",
            "tinychannels-module-0.1.43-windows-11-arm64.zip",
            "e6edde84f041182775a1549de4d370aad5422055a0ef95363f741d2bea0d009e",
        ),
        (
            "windows-2022-x86_64",
            "tinychannels-module-0.1.43-windows-2022-x86_64.zip",
            "a4bbef8b92a4cd3318e2fd70f250334fba3fe4ff203ee1add104973c60181062",
        ),
        (
            "windows-2025-x86_64",
            "tinychannels-module-0.1.43-windows-2025-x86_64.zip",
            "77155d6f38a2b60d17f316dc03d43393b4c94349ce81d1bd06c033d258a75bba",
        ),
    ];
    assert_eq!(record.assets.len(), expected.len());
    for (key, archive, sha256) in expected {
        let asset = record.asset_for(key).expect("supported platform asset");
        assert_eq!(asset.archive, archive);
        assert_eq!(asset.sha256, sha256);
    }
}

#[cfg(feature = "voice")]
#[test]
fn voice_capture_release_pins_summary_contract_and_every_platform_digest() {
    let record = find("tinyvoice").unwrap();
    assert_eq!(record.version, "0.4.1");
    assert_eq!(tinyvoice_bus::CONTRACT_VERSION, (1, 12));
    let expected = [
        (
            "archlinux-arm64",
            "tinyvoice-module-0.4.1-archlinux-arm64.tar.gz",
            "b6bb17e6097f1b089f014c2392e028a481b0c6f8543652d7fc8ae890c86777a7",
        ),
        (
            "archlinux-x86_64",
            "tinyvoice-module-0.4.1-archlinux-x86_64.tar.gz",
            "fde049b4112de018799a91215cbc84572a9b4bd598bb44cfa61d8fcd1b96c9f9",
        ),
        (
            "fedora-43-arm64",
            "tinyvoice-module-0.4.1-fedora-43-arm64.tar.gz",
            "58a9bd07922770610b7f42e6f3708940ca501f430d27cd3ebbb6115402fdb9c8",
        ),
        (
            "fedora-43-x86_64",
            "tinyvoice-module-0.4.1-fedora-43-x86_64.tar.gz",
            "e8c95d1d85f4d89a1b8f8fa1740f8e174c835dab3ad856a99184749ca846bca7",
        ),
        (
            "fedora-44-arm64",
            "tinyvoice-module-0.4.1-fedora-44-arm64.tar.gz",
            "15370fb697a4ffdafe9264728b45efbbe155a91cf00aeba55471b0daa85c4768",
        ),
        (
            "fedora-44-x86_64",
            "tinyvoice-module-0.4.1-fedora-44-x86_64.tar.gz",
            "c7ee0327548bd05079ba30af0108385bea723bb8080f846c4def31ddbab57fcd",
        ),
        (
            "macos-15-arm64",
            "tinyvoice-module-0.4.1-macos-15-arm64.tar.gz",
            "77332a814521ac6ae0a5fd36671088ab6596b981544130b852431fe1d08e9f49",
        ),
        (
            "macos-15-x86_64",
            "tinyvoice-module-0.4.1-macos-15-x86_64.tar.gz",
            "2b4556461e3a30cb06fa5b548cc7e9ee8aee63475de05d16d4809140c44d9f4c",
        ),
        (
            "macos-26-arm64",
            "tinyvoice-module-0.4.1-macos-26-arm64.tar.gz",
            "17c1a6a80c1ae7a46fb19ce129e4b0dffb3d6391747bf9d1a5df0fab6ac75c48",
        ),
        (
            "macos-26-x86_64",
            "tinyvoice-module-0.4.1-macos-26-x86_64.tar.gz",
            "1cf0fbd9f20ffc449a9df938cdc15be4d9c2d4077a839ddeba4423ed8ec2f175",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinyvoice-module-0.4.1-ubuntu-22.04-arm64.tar.gz",
            "001b8b76b920dcab616a1f504efb2893c0ce87a251513dfd8e52c6414b239c7f",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinyvoice-module-0.4.1-ubuntu-22.04-x86_64.tar.gz",
            "c6f6a764747324a2789d9a6cd3764b6b5a5dc08a4a1ca305e157f65df762ae0d",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinyvoice-module-0.4.1-ubuntu-24.04-arm64.tar.gz",
            "8cb7d71e60213896326b85a551dfdd8af2ff1ee8b1a6d6fab27a53ed0d457f57",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinyvoice-module-0.4.1-ubuntu-24.04-x86_64.tar.gz",
            "7755ee1652735ac081bedbdc6dd6a80f8171639cd480f3a0d044230f5da2b0f3",
        ),
        (
            "windows-11-arm64",
            "tinyvoice-module-0.4.1-windows-11-arm64.zip",
            "ea575671e6d119886ec09ea43eed5bd35a7c9445aa8d54fa4621e26100856c6c",
        ),
        (
            "windows-2022-x86_64",
            "tinyvoice-module-0.4.1-windows-2022-x86_64.zip",
            "b472b4ef877bca793b45fca87bb5edbc43f119748cff85a3a25ea6d83815e254",
        ),
        (
            "windows-2025-x86_64",
            "tinyvoice-module-0.4.1-windows-2025-x86_64.zip",
            "c8c4cd75139c09a31082747d9ad4c1b656d9f6d4f8572370ee8017779c3fc1c0",
        ),
    ];
    assert_eq!(record.assets.len(), expected.len());
    for (key, archive, digest) in expected {
        let asset = record.asset_for(key).unwrap();
        assert_eq!(asset.archive, archive);
        assert_eq!(asset.sha256, digest);
    }
}
