use super::{find, ALL};
use tinybus::module::platform::candidates_for;

#[test]
fn tinycomputer_registry_matches_bus_contract_and_published_release() {
    let desktop = find("tinycomputer").expect("compiled computer module");
    assert_eq!(desktop.bus_name, tinycomputer_bus::names::INTERFACE);
    assert_eq!(desktop.object_path, tinycomputer_bus::names::OBJECT_PATH);
    assert_eq!(desktop.version, "0.11.0");
    assert_eq!(desktop.assets.len(), 16);
    assert_eq!(tinycomputer_bus::CONTRACT_VERSION, (2, 12));
    assert_eq!(
        desktop.asset_for("ubuntu-24.04-x86_64").unwrap().sha256,
        "430426bdbc6f8ad8cc5ed17d6f3bd05f73b4d4fe747a8fc9b77f0b3f4fd3506b"
    );
    assert_eq!(
        desktop.asset_for("macos-26-arm64").unwrap().sha256,
        "3a8e20a809262be2154c1b7339fed2a627c13c32b2a7b9de52e4d54ced94a994"
    );
}

#[test]
fn tinywallet_registry_matches_the_verified_v0141_release_manifest() {
    let wallet = find("tinywallet").expect("compiled TinyWallet module");
    assert_eq!(wallet.version, "0.14.1");
    assert_eq!(tinywallet_bus::CONTRACT_VERSION, (1, 8));
    assert_eq!(
        wallet.release_url,
        "https://github.com/tinyhumansai/tinywallet/releases/tag/v0.14.1"
    );

    let actual = wallet
        .assets
        .iter()
        .map(|asset| (asset.host_key, asset.archive, asset.sha256))
        .collect::<Vec<_>>();
    let published = [
        (
            "ubuntu-24.04-x86_64",
            "tinywallet-module-0.14.1-ubuntu-24.04-x86_64.tar.gz",
            "4165b555daaa88305aedffc58aaa05cbb9349b2024ae88d1f27b85497d12f34c",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinywallet-module-0.14.1-ubuntu-24.04-arm64.tar.gz",
            "36ef748e79eca9b3ef38b8fbc76f1cd084ee68b1db776d783ee9ec0be00fa1eb",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinywallet-module-0.14.1-ubuntu-22.04-x86_64.tar.gz",
            "313f6d7fc9e744236ec7a923070b8a3267798b4834a501935dfe108dde5cf3e8",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinywallet-module-0.14.1-ubuntu-22.04-arm64.tar.gz",
            "3f47a5eaa3db37a3ecb846ba93d83630a6c62188224248557b230f60dbfa3000",
        ),
        (
            "macos-26-arm64",
            "tinywallet-module-0.14.1-macos-26-arm64.tar.gz",
            "06154b2fe6fb222e2332a8891c7384acb0b42b45e442d9832932351cd1d95bcc",
        ),
        (
            "macos-26-x86_64",
            "tinywallet-module-0.14.1-macos-26-x86_64.tar.gz",
            "d51d954794620fc106c5bf90ca67b2f35b5fcf7d54752c777af14e1c4189ede2",
        ),
        (
            "macos-15-arm64",
            "tinywallet-module-0.14.1-macos-15-arm64.tar.gz",
            "aa0e77605cd7da33aed7f3c535f6dca3d9eadc0ba01b7030d5d675097562c502",
        ),
        (
            "macos-15-x86_64",
            "tinywallet-module-0.14.1-macos-15-x86_64.tar.gz",
            "25cbba1d1898d3bfdff2f7d7d8bbccb3a8d55bc2b25fcb5e800bbbb3cbbc63e4",
        ),
        (
            "windows-2025-x86_64",
            "tinywallet-module-0.14.1-windows-2025-x86_64.zip",
            "628b67ebf7e52dc2c40bb58e09b56af6cd1bb4698634297b15023bbfd092eb78",
        ),
        (
            "windows-2022-x86_64",
            "tinywallet-module-0.14.1-windows-2022-x86_64.zip",
            "b11b0a92daba13b2fa1afe0c9cefb5632c085a3ddea61ef5477b97cfd335d6b0",
        ),
        (
            "windows-11-arm64",
            "tinywallet-module-0.14.1-windows-11-arm64.zip",
            "7459aec5c7f0e0e8ab0c3dd11461938b0d7a99c8ba7e9a0ee4beaeb894db8907",
        ),
        (
            "archlinux-arm64",
            "tinywallet-module-0.14.1-archlinux-arm64.tar.gz",
            "4e8eddcedbab8dd7f4d03ffdba9d1f8f7f00003240ff55abb9b22381d4ebcc00",
        ),
        (
            "archlinux-x86_64",
            "tinywallet-module-0.14.1-archlinux-x86_64.tar.gz",
            "1e267ec642d89fdc9a630d6ab963abdd1d311fea054b2fae5a2b1055be8261bc",
        ),
        (
            "fedora-43-arm64",
            "tinywallet-module-0.14.1-fedora-43-arm64.tar.gz",
            "df9f1ad060cb4282a369f8c353e2859b88f92dc5511b328922f14efd7411d6bd",
        ),
        (
            "fedora-43-x86_64",
            "tinywallet-module-0.14.1-fedora-43-x86_64.tar.gz",
            "4b178adbd9d1ff7e59d9066299f5cd8fef9d686f8a35d1afed6410162fa1a0bd",
        ),
        (
            "fedora-44-arm64",
            "tinywallet-module-0.14.1-fedora-44-arm64.tar.gz",
            "e3da4a7f315d3d4d90563a37ae72d617dbd4c2f0042f835222e7fa86e04eb82e",
        ),
        (
            "fedora-44-x86_64",
            "tinywallet-module-0.14.1-fedora-44-x86_64.tar.gz",
            "d7ddd11719ab9b6431d6a4c264d6bd8d24764037f2724cb6e280b714793db6fc",
        ),
    ];
    assert_eq!(actual, published);
}

#[test]
fn tinybox_registry_matches_the_published_v0119_release_manifest() {
    let record = find("tinybox").expect("compiled TinyBox module");
    assert_eq!(record.version, "0.1.19");
    assert_eq!(
        record.release_url,
        "https://github.com/tinyhumansai/tinybox/releases/tag/v0.1.19"
    );

    let actual = record
        .assets
        .iter()
        .map(|asset| (asset.host_key, asset.archive, asset.sha256))
        .collect::<Vec<_>>();
    let published = [
        (
            "ubuntu-24.04-x86_64",
            "tinybox-0.1.19-ubuntu-24.04-x86_64.tar.gz",
            "a1dae0e4af4a1ba84fc37b048ad1d1ef0e226ec2c2b312d7607a8ec08d09d134",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinybox-0.1.19-ubuntu-24.04-arm64.tar.gz",
            "245eaf3ab5465e9e01da207ef63bf930c46069b3858fe9ce9da8e71559c0faf8",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinybox-0.1.19-ubuntu-22.04-x86_64.tar.gz",
            "c23102c295b8e5ceef428d1ed178ac910031a23640eaebceb7ebfe3c19a2e05c",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinybox-0.1.19-ubuntu-22.04-arm64.tar.gz",
            "b81e147875a5f21b9b681dffa633c480cfa0bf0121e3ea309feb859d7206d866",
        ),
        (
            "macos-26-arm64",
            "tinybox-0.1.19-macos-26-arm64.tar.gz",
            "437ae2b09337b30ad1af85f3fe95be5cc0d8134986f401a231448a37c340f54b",
        ),
        (
            "macos-26-x86_64",
            "tinybox-0.1.19-macos-26-x86_64.tar.gz",
            "24c3397596e20e3661e6fc6d0ba28f2f36a8c3d7f4e5106030f674d5d3703f6d",
        ),
        (
            "macos-15-arm64",
            "tinybox-0.1.19-macos-15-arm64.tar.gz",
            "61145d8dbb56d866d85610702938973b7c8f523093154ec3a27790dc060565bd",
        ),
        (
            "macos-15-x86_64",
            "tinybox-0.1.19-macos-15-x86_64.tar.gz",
            "8e68322fe3ff762ad6b31a55413cfd7772174b2bdadc0b5391d1816d36a6b373",
        ),
        (
            "windows-2025-x86_64",
            "tinybox-0.1.19-windows-2025-x86_64.zip",
            "8ac5b73f23deee35d2f61209b801a5fcb6465277e35afc45dac4e91cfe528ce0",
        ),
        (
            "windows-2022-x86_64",
            "tinybox-0.1.19-windows-2022-x86_64.zip",
            "f2682b28908c9736382fab57c0f579db4a17ae3e037d9422f185a0f949e69143",
        ),
        (
            "windows-11-arm64",
            "tinybox-0.1.19-windows-11-arm64.zip",
            "50873a041c05d009f8f65812e6ac95fef001b1d71a897383da1f34fac1f2a7d0",
        ),
    ];

    assert_eq!(actual, published);
}

#[test]
fn tinymcp_registry_matches_the_published_v081_release_manifest() {
    let record = find("tinymcp").expect("compiled TinyMCP module");
    assert_eq!(record.version, "0.8.1");
    assert_eq!(tinymcp_bus::CONTRACT_VERSION, (1, 12));
    assert_eq!(
        record.release_url,
        "https://github.com/tinyhumansai/tinymcp/releases/tag/v0.8.1"
    );

    let actual = record
        .assets
        .iter()
        .map(|asset| (asset.host_key, asset.archive, asset.sha256))
        .collect::<Vec<_>>();
    let published = [
        (
            "ubuntu-24.04-x86_64",
            "tinymcp-0.8.1-ubuntu-24.04-x86_64.tar.gz",
            "54e05304cc088f83aeb858780426d06aebe4ed604c8a92d8d66136c532807c21",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinymcp-0.8.1-ubuntu-24.04-arm64.tar.gz",
            "78bfa9a7e6751b6f7b7c2d6242f6e5f9369421e4ff54e5aef990f1324a386325",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinymcp-0.8.1-ubuntu-22.04-x86_64.tar.gz",
            "eb190c7af70801d17592027dba60c4669e804244b3111859444ca641c209a588",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinymcp-0.8.1-ubuntu-22.04-arm64.tar.gz",
            "59dffc24b69552854bc9c6dd1f2d4c07293d15e44d3a4de44a86387b1afb0725",
        ),
        (
            "macos-26-arm64",
            "tinymcp-0.8.1-macos-26-arm64.tar.gz",
            "05df5adc80296ba4a6882d5aa519fcdbd7ccd8671ee94ad8b647268bc0cd0523",
        ),
        (
            "macos-26-x86_64",
            "tinymcp-0.8.1-macos-26-x86_64.tar.gz",
            "9e4047d0fc1b54f9ec9aa531a604771e2037390a5178413e652248a9e64eef91",
        ),
        (
            "macos-15-arm64",
            "tinymcp-0.8.1-macos-15-arm64.tar.gz",
            "cdf409f7e2e93e9d75f35d8f4e7186ddd090a583a48eddf33ef62b1d83c2af89",
        ),
        (
            "macos-15-x86_64",
            "tinymcp-0.8.1-macos-15-x86_64.tar.gz",
            "0bc9156d0c2bcf9ba6e9277ee9ad3846185dc2192624f5989ab1331ed12ddaf6",
        ),
        (
            "windows-2025-x86_64",
            "tinymcp-0.8.1-windows-2025-x86_64.zip",
            "f2a32c36fe35c7d24e1bcccf489a686e01c3fb875ec10662be45c2858a24e280",
        ),
        (
            "windows-2022-x86_64",
            "tinymcp-0.8.1-windows-2022-x86_64.zip",
            "e9b14fd191d2269bb6cc088c76e3ff96a8d97197be9554b0ac8fddd9365c92cc",
        ),
        (
            "windows-11-arm64",
            "tinymcp-0.8.1-windows-11-arm64.zip",
            "6fe315376f651a9f425c437d76f4bf50f0a8838d783a94d0fecf61fb2adb71a9",
        ),
    ];

    assert_eq!(actual, published);
}

#[test]
fn ids_and_bus_names_are_unique() {
    // Two records claiming one bus name is a conflict tinybus would only
    // surface at load time, on whichever one happened to be second.
    for (i, record) in ALL.iter().enumerate() {
        for other in &ALL[i + 1..] {
            assert_ne!(record.id, other.id, "duplicate module id");
            assert_ne!(record.bus_name, other.bus_name, "duplicate bus name");
        }
    }
}

#[test]
fn every_object_path_matches_its_bus_name() {
    // tinybus derives a module's object path from its bus name by replacing
    // dots with slashes, and admission compares the two. A mismatch here is
    // a module that downloads and then refuses to load.
    for record in ALL {
        assert_eq!(
            record.object_path,
            format!("/{}", record.bus_name.replace('.', "/")),
            "{} object path does not match its bus name",
            record.id
        );
    }
}

#[test]
fn every_digest_is_a_lowercase_sha256() {
    // An uppercase or truncated digest is refused by tinybus at download
    // time, which is a slow way to find a typo in this file.
    for record in ALL {
        for asset in record.assets {
            assert_eq!(
                asset.sha256.len(),
                64,
                "{} / {} digest is not 64 characters",
                record.id,
                asset.host_key
            );
            assert!(
                asset
                    .sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
                "{} / {} digest is not lowercase hex",
                record.id,
                asset.host_key
            );
        }
    }
}

#[test]
fn every_asset_name_carries_its_host_key_and_a_known_extension() {
    // tinybus selects the asset by exact name and requires a `.tar.gz` or
    // `.zip` archive, so a name that does not match its key is a module that
    // loads the wrong platform's library.
    for record in ALL {
        for asset in record.assets {
            assert!(
                asset.archive.contains(asset.host_key),
                "{} asset {} does not name its host key {}",
                record.id,
                asset.archive,
                asset.host_key
            );
            let windows = asset.host_key.starts_with("windows");
            assert_eq!(
                windows,
                asset.archive.ends_with(".zip"),
                "{} asset {} has the wrong archive format for its host",
                record.id,
                asset.archive
            );
            if !windows {
                assert!(asset.archive.ends_with(".tar.gz"));
            }
        }
    }
}

#[test]
fn every_asset_name_carries_the_pinned_version() {
    // The digests and the version have to describe one release; an asset
    // left behind at an older version would download bytes the digest
    // beside it never matched.
    for record in ALL {
        for asset in record.assets {
            assert!(
                asset.archive.contains(record.version),
                "{} asset {} is not from version {}",
                record.id,
                asset.archive,
                record.version
            );
        }
    }
}

#[test]
fn the_release_url_is_a_tag_on_github() {
    // tinybus refuses a URL that is not a tag, because a branch URL names
    // bytes that can change under a digest that was checked once.
    for record in ALL.iter().filter(|record| !record.assets.is_empty()) {
        assert!(
            record
                .release_url
                .starts_with("https://github.com/tinyhumansai/"),
            "{} release url is not an upstream GitHub URL",
            record.id
        );
        assert!(
            record.release_url.contains("/releases/tag/"),
            "{} release url is not a tag",
            record.id
        );
        assert!(
            record.release_url.ends_with(record.version),
            "{} release url does not name version {}",
            record.id,
            record.version
        );
    }
}

/// Every host key `platform` can produce, across the supported triples.
fn every_host_key() -> Vec<String> {
    let hosts = [
        ("linux", "x86_64", Some((2, 39))),
        ("linux", "aarch64", Some((2, 39))),
        ("linux", "x86_64", Some((2, 35))),
        ("linux", "aarch64", Some((2, 35))),
        ("macos", "x86_64", None),
        ("macos", "aarch64", None),
        ("windows", "x86_64", None),
        ("windows", "aarch64", None),
    ];
    let mut keys: Vec<String> = hosts
        .into_iter()
        .flat_map(|(os, arch, glibc)| candidates_for(os, arch, glibc))
        .collect();
    keys.sort();
    keys.dedup();
    keys
}

fn supported_host_keys(_record: &super::ModuleRecord) -> Vec<String> {
    every_host_key()
}

#[test]
fn a_record_that_pins_a_release_covers_every_host_the_platform_table_offers() {
    // The two tables are written independently and would drift silently:
    // `platform` offering a key no release publishes turns a supported host
    // into an "unsupported host" at first use.
    //
    // Scoped to records that pin a release at all. A record with no assets
    // is a module this build knows but has no published artifact for; it
    // loads from a developer build or the module search path, and asserting
    // release coverage for a release that does not exist would only assert
    // that it does not exist. The partial-coverage case — the one that is
    // actually a bug — is caught below.
    for record in ALL.iter().filter(|record| !record.assets.is_empty()) {
        for key in supported_host_keys(record) {
            assert!(
                record.asset_for(&key).is_some(),
                "{} publishes no asset for {key}, which the platform table would ask for",
                record.id
            );
        }
    }
}

#[test]
fn a_record_publishes_for_every_host_or_for_none() {
    // Partial coverage is the drift that hurts: it looks supported until a
    // user on the missing platform reaches the feature. All-or-nothing keeps
    // "not published yet" distinguishable from "published and incomplete".
    for record in ALL {
        let host_keys = supported_host_keys(record);
        let covered = host_keys
            .into_iter()
            .filter(|key| record.asset_for(key).is_some())
            .count();
        assert!(
            covered == 0 || covered == supported_host_keys(record).len(),
            "{} publishes assets for {covered} of {} host keys",
            record.id,
            supported_host_keys(record).len()
        );
    }
}

#[test]
fn find_resolves_known_ids_only() {
    assert!(find("tinydocs").is_some());
    assert!(find("not-a-module").is_none());
}

#[test]
fn tinychannels_registry_matches_prepared_connection_release() {
    let record = find("tinychannels").expect("compiled TinyChannels module");
    assert_eq!(record.version, "0.1.17");
    assert_eq!(tinychannels_bus::CONTRACT_VERSION, 7);
    assert_eq!(
        tinychannels_bus::module_relay::MODULE_OPERATION_VERSION,
        (1, 8)
    );
    let expected = [
        (
            "macos-15-arm64",
            "tinychannels-module-0.1.17-macos-15-arm64.tar.gz",
            "79a0c1f14258d9a8230eb6a9bd962f243d84ca57381e48e0840e6696b3d86ce2",
        ),
        (
            "macos-15-x86_64",
            "tinychannels-module-0.1.17-macos-15-x86_64.tar.gz",
            "9beeb0697a0eee6016aae0aeb27999ffa115945c20a7786742ade1c09dfb3d84",
        ),
        (
            "macos-26-arm64",
            "tinychannels-module-0.1.17-macos-26-arm64.tar.gz",
            "bc08c3d3d2ec1ba7f07593fd67e7e2b394757478e4de9e8e66dc6ba9a9cb2413",
        ),
        (
            "macos-26-x86_64",
            "tinychannels-module-0.1.17-macos-26-x86_64.tar.gz",
            "0eeb60e6f55f4d7f6ded815ea95d1722917285e13ebad6fa5c1e2211766c335b",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinychannels-module-0.1.17-ubuntu-22.04-arm64.tar.gz",
            "fa5f5aeedeca8ab03cfd31a5865c1f69e0ee7a935a7461c3f87c7fc491f6765d",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinychannels-module-0.1.17-ubuntu-22.04-x86_64.tar.gz",
            "ab6c61dc697637e9ef07067d17d369e99f4bc3284815320f68048ce456271355",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinychannels-module-0.1.17-ubuntu-24.04-arm64.tar.gz",
            "cae106ae240d9f7d5022e397a33df4f851c8114c205292c4b5d84d817a9aba1a",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinychannels-module-0.1.17-ubuntu-24.04-x86_64.tar.gz",
            "896af998236d9a27df67da8d99badee8a17b971aca3a7158700679baea267c49",
        ),
        (
            "windows-11-arm64",
            "tinychannels-module-0.1.17-windows-11-arm64.zip",
            "534415e3d59bed642e0b4e9ca623e2706a4f53e544fb2331dbcc5d27d4fb1833",
        ),
        (
            "windows-2022-x86_64",
            "tinychannels-module-0.1.17-windows-2022-x86_64.zip",
            "5a1be7f484d67282612f98325a8a72503b70f5ea5c9b4fba01dc92077624f759",
        ),
        (
            "windows-2025-x86_64",
            "tinychannels-module-0.1.17-windows-2025-x86_64.zip",
            "9fb1d787045bd835c3422b555133db34b158bb5f0ec2a7033249ebf713a16c46",
        ),
    ];
    assert_eq!(record.assets.len(), expected.len());
    for (key, archive, sha256) in expected {
        let asset = record.asset_for(key).expect("supported platform asset");
        assert_eq!(asset.archive, archive);
        assert_eq!(asset.sha256, sha256);
    }
}
