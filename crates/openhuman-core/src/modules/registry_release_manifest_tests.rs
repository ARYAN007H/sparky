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
    assert_eq!(record.version, "0.4.2");
    assert_eq!(tinyvoice_bus::CONTRACT_VERSION, (1, 12));
    let expected = [
        (
            "archlinux-arm64",
            "tinyvoice-module-0.4.2-archlinux-arm64.tar.gz",
            "0cef54f6248019db21282fa76db4f9663fbe830c3b18c2804ab28833d60329e1",
        ),
        (
            "archlinux-x86_64",
            "tinyvoice-module-0.4.2-archlinux-x86_64.tar.gz",
            "707838c2fb8c3e4ad861d56e37418fb71c1b21bc56826fb159fd46057f1af9f0",
        ),
        (
            "fedora-43-arm64",
            "tinyvoice-module-0.4.2-fedora-43-arm64.tar.gz",
            "51c1d67747872016127c3633fda9ea7111eaa8142782e6b6da4ef15ed9c4a171",
        ),
        (
            "fedora-43-x86_64",
            "tinyvoice-module-0.4.2-fedora-43-x86_64.tar.gz",
            "01aac694078952c8dea1090bc99b584930c4a8a043ff8defb0512110f4aa9433",
        ),
        (
            "fedora-44-arm64",
            "tinyvoice-module-0.4.2-fedora-44-arm64.tar.gz",
            "21bb4a50fe65aaa6a5d09f94d15933c3dd7878b7cf919daa06f2f0939ea421be",
        ),
        (
            "fedora-44-x86_64",
            "tinyvoice-module-0.4.2-fedora-44-x86_64.tar.gz",
            "7113c298e9abbbfacc55a45e237b4962ce9b0256f9ffe12e901ee45cddb226d0",
        ),
        (
            "macos-15-arm64",
            "tinyvoice-module-0.4.2-macos-15-arm64.tar.gz",
            "1c0cf8391e8afb35a13763b6c05fe811c14a640fdf9a79e17bae7f32952009d0",
        ),
        (
            "macos-15-x86_64",
            "tinyvoice-module-0.4.2-macos-15-x86_64.tar.gz",
            "72c6d9fb1aad0b4ce3f7cceaad45111d4450e9a1285140945dcc585a69e16bde",
        ),
        (
            "macos-26-arm64",
            "tinyvoice-module-0.4.2-macos-26-arm64.tar.gz",
            "2b37df831a1196f5d379994a1b85e9f69015d7cde47564470afbb2ff31a85b77",
        ),
        (
            "macos-26-x86_64",
            "tinyvoice-module-0.4.2-macos-26-x86_64.tar.gz",
            "1f9ffd9307505693be31df2f6eaa0e6de32f104bd46640c06dc82bffa3e03fa7",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinyvoice-module-0.4.2-ubuntu-22.04-arm64.tar.gz",
            "ad63d27d2c40f8388d93a9d3556bfc8539c4b5ffe8839efaa0c1b7a1b7a61d43",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinyvoice-module-0.4.2-ubuntu-22.04-x86_64.tar.gz",
            "c2523bf5066757ce0c81e7f5c97a4b902312007ce079606d005e008ca8be66a5",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinyvoice-module-0.4.2-ubuntu-24.04-arm64.tar.gz",
            "78da28da3a483731bec8d53bc561219cc6dec793d66f27eb908755d88118461e",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinyvoice-module-0.4.2-ubuntu-24.04-x86_64.tar.gz",
            "1b542b8b39223525f40f02245ef21b7d995a039eef9cb8a2586bb46cce9668f9",
        ),
        (
            "windows-11-arm64",
            "tinyvoice-module-0.4.2-windows-11-arm64.zip",
            "734db41e5c5cd5580c5aa4fd7a9c3e5281aee62ea17c31d8c7037c7356360e82",
        ),
        (
            "windows-2022-x86_64",
            "tinyvoice-module-0.4.2-windows-2022-x86_64.zip",
            "47701888511b5b8ac71a5f92affcb04334b43e479b43e2b5a0937f2134f4cc5e",
        ),
        (
            "windows-2025-x86_64",
            "tinyvoice-module-0.4.2-windows-2025-x86_64.zip",
            "46b29e886d3ec82b996ca7138a7a60577fe6236383df59cb34aedfd3c670e9b0",
        ),
    ];
    assert_eq!(record.assets.len(), expected.len());
    for (key, archive, digest) in expected {
        let asset = record.asset_for(key).unwrap();
        assert_eq!(asset.archive, archive);
        assert_eq!(asset.sha256, digest);
    }
}
