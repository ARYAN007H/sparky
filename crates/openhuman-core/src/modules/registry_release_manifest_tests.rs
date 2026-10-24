use super::*;

#[test]
fn tinychannels_registry_matches_scoped_runtime_release() {
    let record = find("tinychannels").expect("compiled TinyChannels module");
    assert_eq!(record.version, "0.1.40");
    assert_eq!(tinychannels_bus::CONTRACT_VERSION, 22);
    assert_eq!(
        tinychannels_bus::module_relay::MODULE_OPERATION_VERSION,
        (1, 23)
    );
    let expected = [
        (
            "archlinux-arm64",
            "tinychannels-module-0.1.40-archlinux-arm64.tar.gz",
            "2643b5aefb51a882739640666e456022921304b1ff3cf935005803e1baeae2db",
        ),
        (
            "archlinux-x86_64",
            "tinychannels-module-0.1.40-archlinux-x86_64.tar.gz",
            "b65e7e2e99af174166ce18acce7b3e9948b03f93644ac34958a2dd1d45b9783f",
        ),
        (
            "fedora-43-arm64",
            "tinychannels-module-0.1.40-fedora-43-arm64.tar.gz",
            "06470acac32353419b9e2c8eed0b13244de9957bc6d7575932b8bf5989579780",
        ),
        (
            "fedora-43-x86_64",
            "tinychannels-module-0.1.40-fedora-43-x86_64.tar.gz",
            "5956996b8fdc829e530a5cf808a8f9561dba7f91aead25df4f091a6c5ead605f",
        ),
        (
            "fedora-44-arm64",
            "tinychannels-module-0.1.40-fedora-44-arm64.tar.gz",
            "e449125869811077cb020b6dbd9f5dffc25fbf1526b8ee2402f0fac0b2f53bbc",
        ),
        (
            "fedora-44-x86_64",
            "tinychannels-module-0.1.40-fedora-44-x86_64.tar.gz",
            "f5ab13f94fe850ec90a95f1d527ee9c55ab3fdeb427f3c1c2e390c35b300d76e",
        ),
        (
            "macos-15-arm64",
            "tinychannels-module-0.1.40-macos-15-arm64.tar.gz",
            "98d31e633601c365200c679f288b13f9d7aef2023ebb9853bcbad690ddbbe1a0",
        ),
        (
            "macos-15-x86_64",
            "tinychannels-module-0.1.40-macos-15-x86_64.tar.gz",
            "4ac740b410102b29ba469f37f450b3e03fac3ce31ccb9d3538c1633025a7bd4d",
        ),
        (
            "macos-26-arm64",
            "tinychannels-module-0.1.40-macos-26-arm64.tar.gz",
            "ed2aab10a198193e5bebb5957095acad664a5a66a02d6142563815df6c353b53",
        ),
        (
            "macos-26-x86_64",
            "tinychannels-module-0.1.40-macos-26-x86_64.tar.gz",
            "210a7904dd942629af46d7ad36d0b0312f3a1cf0aa238b667ca6309ebf86c8e2",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinychannels-module-0.1.40-ubuntu-22.04-arm64.tar.gz",
            "48d360aea7849bb93050dbb56f9c28de6848572632908b635fb77df2a65c3bb1",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinychannels-module-0.1.40-ubuntu-22.04-x86_64.tar.gz",
            "6026081ac8ceb7b91daf0101364c92f99c10c5072a270b91eaeed6acad5d1f90",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinychannels-module-0.1.40-ubuntu-24.04-arm64.tar.gz",
            "0b4ef6b3c9c2c81f1efdbc5cbe8713478ff4e2f76682625465ee8c7120dfb798",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinychannels-module-0.1.40-ubuntu-24.04-x86_64.tar.gz",
            "bec22856f0b5f36f8c48f893c53d83015961a596d4ad56a33714ca378c2b4c98",
        ),
        (
            "windows-11-arm64",
            "tinychannels-module-0.1.40-windows-11-arm64.zip",
            "5f463711be6f8c856e13f77ba5d376b1b4e5b9519551411ae7bfd94a196bd633",
        ),
        (
            "windows-2022-x86_64",
            "tinychannels-module-0.1.40-windows-2022-x86_64.zip",
            "5df4ce49573c7d1e03a93b50cf092bf62dc6b92f50bcfe1471aff50a844268ce",
        ),
        (
            "windows-2025-x86_64",
            "tinychannels-module-0.1.40-windows-2025-x86_64.zip",
            "e8dd10cc102a7889d7d9ba8f50416b2ee3cec810f203555bf40cd0f157765552",
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
