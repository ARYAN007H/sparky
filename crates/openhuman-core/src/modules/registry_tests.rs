use super::{find, ALL};
use tinybus::module::platform::candidates_for;

#[test]
fn tinycomputer_registry_matches_bus_contract_and_published_release() {
    let desktop = find("tinycomputer").expect("compiled computer module");
    assert_eq!(desktop.bus_name, tinycomputer_bus::names::INTERFACE);
    assert_eq!(desktop.object_path, tinycomputer_bus::names::OBJECT_PATH);
    assert_eq!(desktop.version, "0.13.0");
    assert_eq!(desktop.assets.len(), 17);
    assert_eq!(tinycomputer_bus::CONTRACT_VERSION, (2, 17));
    assert_eq!(
        desktop.asset_for("ubuntu-24.04-x86_64").unwrap().sha256,
        "bf96e87011844af29aa065f6ad95fd0f9bfd1d3a904dca4843ab41bb59b9c873"
    );
    assert_eq!(
        desktop.asset_for("macos-26-arm64").unwrap().sha256,
        "6b4965e63e3c8f666f9aa9abf12fb11d172c23097e95aee08688629dc6fdacd5"
    );
}

#[test]
#[cfg(feature = "web3")]
fn tinywallet_registry_matches_the_verified_v022_release_manifest() {
    let wallet = find("tinywallet").expect("compiled TinyWallet module");
    assert_eq!(wallet.version, "0.22.0");
    #[cfg(feature = "web3")]
    assert_eq!(tinywallet_bus::CONTRACT_VERSION, (1, 14));
    assert_eq!(
        wallet.release_url,
        "https://github.com/tinyhumansai/tinywallet/releases/tag/v0.22.0"
    );

    let mut actual = wallet
        .assets
        .iter()
        .map(|asset| (asset.host_key, asset.archive, asset.sha256))
        .collect::<Vec<_>>();
    assert_eq!(wallet.assets.len(), 17);
    let mut published = [
        (
            "archlinux-arm64",
            "tinywallet-module-0.23.0-archlinux-arm64.tar.gz",
            "6435cbb45096a872566f62eda0f44a2edfe1fa6f7508c196edfb54826413e4c5",
        ),
        (
            "archlinux-x86_64",
            "tinywallet-module-0.23.0-archlinux-x86_64.tar.gz",
            "5a03ffb3fc897928e3c1d9fed1341ae2ea58d55b4862a8b9817d6d46fcb2e664",
        ),
        (
            "fedora-43-arm64",
            "tinywallet-module-0.23.0-fedora-43-arm64.tar.gz",
            "4fa874a5ff4a9a601dcb14d0d253039fdd225d416066223051dd2b70d84b3e48",
        ),
        (
            "fedora-43-x86_64",
            "tinywallet-module-0.23.0-fedora-43-x86_64.tar.gz",
            "0fe11d2975af2a0f6a5ac6b56a30972d847532a478e11ce166eda55898d4702e",
        ),
        (
            "fedora-44-arm64",
            "tinywallet-module-0.23.0-fedora-44-arm64.tar.gz",
            "891dd20c7a652e29e01560cc1eb6a9558a7bab3558b5a70bb320251253c2ec1f",
        ),
        (
            "fedora-44-x86_64",
            "tinywallet-module-0.23.0-fedora-44-x86_64.tar.gz",
            "b8c3c4b6dab634f4b7f7f9c2688c12dd82e3ffec645e3d55a4e20efe134debf1",
        ),
        (
            "macos-15-arm64",
            "tinywallet-module-0.23.0-macos-15-arm64.tar.gz",
            "f7ce0b8fe8d72862cc6f950fc8f7275f1f54c6538b032f3d2b6e5063326535e5",
        ),
        (
            "macos-15-x86_64",
            "tinywallet-module-0.23.0-macos-15-x86_64.tar.gz",
            "11df5ec9a04ade1656aca9fdee9d8a22d9fa83d182dda9266b85bd92f70d07c5",
        ),
        (
            "macos-26-arm64",
            "tinywallet-module-0.23.0-macos-26-arm64.tar.gz",
            "2416f50a383fda73a2cb62c1c2d6e882374652a284c780e68dc810a0ebacf424",
        ),
        (
            "macos-26-x86_64",
            "tinywallet-module-0.23.0-macos-26-x86_64.tar.gz",
            "4bc5e4ee29955af0343804388920420d909f29c67f9a3402b4bb3efd9217c60f",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinywallet-module-0.23.0-ubuntu-22.04-arm64.tar.gz",
            "9725cc2f1a9be84eb5da5b988aaebcd7679954399f04eaa4c62018cf9f4976dd",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinywallet-module-0.23.0-ubuntu-22.04-x86_64.tar.gz",
            "b5392ae75c355f798906e13cc3118f8f781b8615f0890d50881c85cdc64ffbf8",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinywallet-module-0.23.0-ubuntu-24.04-arm64.tar.gz",
            "ae7d06f16b585d9d52a08c1575bf6f2d4b6f3d5cc57cb686c0ba6ebc4eeac23b",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinywallet-module-0.23.0-ubuntu-24.04-x86_64.tar.gz",
            "44ed3136bb30e79caaf54f7fff155666866891fd92f9443bb8e9050eb803857b",
        ),
        (
            "windows-11-arm64",
            "tinywallet-module-0.23.0-windows-11-arm64.zip",
            "494b3961c801b26ce61a09f1d78be7cf3f575511e692afbb6c349472a52455ec",
        ),
        (
            "windows-2022-x86_64",
            "tinywallet-module-0.23.0-windows-2022-x86_64.zip",
            "057a8b62b5122f487bac2915ce1ee53804005c5987b28551b8d9fac71d196c38",
        ),
        (
            "windows-2025-x86_64",
            "tinywallet-module-0.23.0-windows-2025-x86_64.zip",
            "a194b36f04fcf5c3f719d453b7bff9763c6211d646ad99a71bd56babd801b14a",
        ),
    ];
    actual.sort_unstable();
    published.sort_unstable();
    assert_eq!(actual, published);
}

#[test]
fn tinybox_registry_matches_the_published_v0129_release_manifest() {
    let record = find("tinybox").expect("compiled TinyBox module");
    assert_eq!(record.version, "0.1.29");
    assert_eq!(tinybox_bus::CONTRACT_VERSION, (1, 18));
    assert_eq!(
        record.release_url,
        "https://github.com/tinyhumansai/tinybox/releases/tag/v0.1.29"
    );

    let actual = record
        .assets
        .iter()
        .map(|asset| (asset.host_key, asset.archive, asset.sha256))
        .collect::<Vec<_>>();
    let published = [
        (
            "archlinux-arm64",
            "tinybox-0.1.30-archlinux-arm64.tar.gz",
            "1ed931f3bc8299cf11a67f01bebec2d498ba2d65b011779c39aad2559234523c",
        ),
        (
            "archlinux-x86_64",
            "tinybox-0.1.30-archlinux-x86_64.tar.gz",
            "942f202c6888f93fb55697f34e94088df736c2aa1d7627bd010d1fc82a8de6eb",
        ),
        (
            "fedora-43-arm64",
            "tinybox-0.1.30-fedora-43-arm64.tar.gz",
            "9d0f4114ffa740d88f020107e66048a24ddab87b595d392f56083e2b58695447",
        ),
        (
            "fedora-43-x86_64",
            "tinybox-0.1.30-fedora-43-x86_64.tar.gz",
            "29bdb2e3d629d6983cd81c75c48ffce09383071a02f8f6069c7f70bbfe0f15ee",
        ),
        (
            "fedora-44-arm64",
            "tinybox-0.1.30-fedora-44-arm64.tar.gz",
            "54d706bf29de5356c8709159461bb4e7b8a69f15a7437992b747938b73b26fbe",
        ),
        (
            "fedora-44-x86_64",
            "tinybox-0.1.30-fedora-44-x86_64.tar.gz",
            "2c2bd426ce201639527c69d73916f3c83eb1b086195340dc45c36c0426d76ba5",
        ),
        (
            "macos-15-arm64",
            "tinybox-0.1.30-macos-15-arm64.tar.gz",
            "b671dc6321939d189058724ad1d668e835026af44c6634837dc29b0285ef16fc",
        ),
        (
            "macos-15-x86_64",
            "tinybox-0.1.30-macos-15-x86_64.tar.gz",
            "1c09e0e25456c4f9b62199807a2d5ce77926ff73787636e42d3aced255f8b81c",
        ),
        (
            "macos-26-arm64",
            "tinybox-0.1.30-macos-26-arm64.tar.gz",
            "de601f633e842751d22c717135436fb4616cb6956f7f01556857e2d731c6a519",
        ),
        (
            "macos-26-x86_64",
            "tinybox-0.1.30-macos-26-x86_64.tar.gz",
            "8fdc30a945847689832b569d6ffe47f7a46bd913a297d85d4442eb9f946e1cb5",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinybox-0.1.30-ubuntu-22.04-arm64.tar.gz",
            "25236b2e0de828acdff256d23b80fc722a724c95b772ec37cb03b7ada7dce8c8",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinybox-0.1.30-ubuntu-22.04-x86_64.tar.gz",
            "757301472a233da55dd39d70a8688a0b4bf7c04c6c0604bd3cbc2df56f7506e2",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinybox-0.1.30-ubuntu-24.04-arm64.tar.gz",
            "5a277e2fd46453d961c4c9a09b9c33e5810d816f7dc8f057e620dd73532a2c76",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinybox-0.1.30-ubuntu-24.04-x86_64.tar.gz",
            "74787ae77f683cf6d01b4a337cb7515a60c5f2fe2ca79d33e21755477c4cdf80",
        ),
        (
            "windows-11-arm64",
            "tinybox-0.1.30-windows-11-arm64.zip",
            "2aede8c034cc37d0df38b2490aea4db2face458255322250841893af031ea190",
        ),
        (
            "windows-2022-x86_64",
            "tinybox-0.1.30-windows-2022-x86_64.zip",
            "514e9215c711f7e35c6d6fd5257b73109d1670db5297bb647e98d3882821aa2f",
        ),
        (
            "windows-2025-x86_64",
            "tinybox-0.1.30-windows-2025-x86_64.zip",
            "d5353e969d5a0557f25c9d4846b861e2c0a4e1dcc48f12f1b23de2dcb23b1a4b",
        ),
    ];

    assert_eq!(actual.len(), 17);
    assert_eq!(actual, published);
}
#[test]
fn tinymcp_registry_matches_the_published_v0180_release_manifest() {
    let record = find("tinymcp").expect("compiled TinyMCP module");
    assert_eq!(record.version, "0.18.0");
    assert_eq!(tinymcp_bus::CONTRACT_VERSION, (1, 24));
    assert_eq!(
        record.release_url,
        "https://github.com/tinyhumansai/tinymcp/releases/tag/v0.18.0"
    );
    let actual = record
        .assets
        .iter()
        .map(|asset| (asset.host_key, asset.archive, asset.sha256))
        .collect::<Vec<_>>();
    let published = [
        (
            "archlinux-rolling-arm64",
            "tinymcp-0.18.1-archlinux-arm64.tar.gz",
            "63e062c14993fef556d770ba977bc932ddf7843e44fbbb6487036356713b0be1",
        ),
        (
            "archlinux-rolling-x86_64",
            "tinymcp-0.18.1-archlinux-x86_64.tar.gz",
            "6fcd6daf2dc56ee9f8cc5465d13465e195c3c0e6a3675475f7bdb1395542ca38",
        ),
        (
            "fedora-43-arm64",
            "tinymcp-0.18.1-fedora-43-arm64.tar.gz",
            "4dd2b74371d8dbc4d6860eb07c9e5033185ff8e79f63df14df029109f695272f",
        ),
        (
            "fedora-43-x86_64",
            "tinymcp-0.18.1-fedora-43-x86_64.tar.gz",
            "b99bf1911458e8fe2ff498c02ae68390d0d4ef5fcecb60a9199ae3f0a0d12800",
        ),
        (
            "fedora-44-arm64",
            "tinymcp-0.18.1-fedora-44-arm64.tar.gz",
            "491c2d27998a7edf392af2ca4c98c81bc318efce4bc4c9d6fda5cb70ff5ca967",
        ),
        (
            "fedora-44-x86_64",
            "tinymcp-0.18.1-fedora-44-x86_64.tar.gz",
            "f5c225716ea256bb1714ff507e38abad5355e4ad768a559537e609cf29951aea",
        ),
        (
            "macos-15-arm64",
            "tinymcp-0.18.1-macos-15-arm64.tar.gz",
            "594a6ec5b05f0f85e456787f234b25ccf09cf47f7109b73277270582c3caedde",
        ),
        (
            "macos-15-x86_64",
            "tinymcp-0.18.1-macos-15-x86_64.tar.gz",
            "ded08986747d819de9e71c5582eafa790d7f92c474cd8be1880ea7b97d191205",
        ),
        (
            "macos-26-arm64",
            "tinymcp-0.18.1-macos-26-arm64.tar.gz",
            "0f98865ec04c5b862ceeda8f87362fc1b5db98280a095cb5cddd1a34531a5240",
        ),
        (
            "macos-26-x86_64",
            "tinymcp-0.18.1-macos-26-x86_64.tar.gz",
            "8c6cb2daf759056b7dc24036dcd89ba9012f24a0bb6fd7bf73d2cef323c7198b",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinymcp-0.18.1-ubuntu-22.04-arm64.tar.gz",
            "07eaf68ab296f8de059e90988160d5c6c29accad4830f5b228d3f932df000372",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinymcp-0.18.1-ubuntu-22.04-x86_64.tar.gz",
            "2e7a1890281ec795978e046a56162acc751d239d9baf985d3c05655d7edeee9b",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinymcp-0.18.1-ubuntu-24.04-arm64.tar.gz",
            "db44d547ffd4a22b572e2033e9c93f87315891107525b46e4deec79fadc72865",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinymcp-0.18.1-ubuntu-24.04-x86_64.tar.gz",
            "bf518e7f6d9f9b110ee191dde68dc4fe9f192e8a51523e2b8dfbc835cb6ecf55",
        ),
        (
            "windows-11-arm64",
            "tinymcp-0.18.1-windows-11-arm64.zip",
            "57024764c301d6cec0bca39af7da3edfd4d64105264280f6f33e05eeff07fb60",
        ),
        (
            "windows-2022-x86_64",
            "tinymcp-0.18.1-windows-2022-x86_64.zip",
            "fc74f8d217ddafd2394677b2bb0cd601b82230c6e3702994b095056a65dc10c5",
        ),
        (
            "windows-2025-x86_64",
            "tinymcp-0.18.1-windows-2025-x86_64.zip",
            "6ac5ea9cb2822d19261a6609ad65ef040114c61b99218496882cab10bf4b0e1b",
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
    // `.zip` archive. Most archive platform stems match their host keys;
    // TinyMCP's release uses the canonical `archlinux` stem for the rolling
    // Arch host keys.
    for record in ALL {
        for asset in record.assets {
            let archive_host_key = if record.id == "tinymcp" {
                asset
                    .host_key
                    .strip_prefix("archlinux-rolling-")
                    .map(|suffix| format!("archlinux-{suffix}"))
                    .unwrap_or_else(|| asset.host_key.to_owned())
            } else {
                asset.host_key.to_owned()
            };
            assert!(
                asset.archive.contains(&archive_host_key),
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
    assert!(find("tinymcp").is_some());
    assert!(find("not-a-module").is_none());
}

#[test]
fn tinychannels_registry_matches_scoped_runtime_release() {
    let record = find("tinychannels").expect("compiled TinyChannels module");
    assert_eq!(record.version, "0.1.30");
    assert_eq!(tinychannels_bus::CONTRACT_VERSION, 21);
    assert_eq!(
        tinychannels_bus::module_relay::MODULE_OPERATION_VERSION,
        (1, 21)
    );
    let expected = [
        (
            "macos-15-arm64",
            "tinychannels-module-0.1.31-macos-15-arm64.tar.gz",
            "cffe8ed587aba386971c10fb22a27f4088c88f94fe11551a947f3908e365b150",
        ),
        (
            "macos-15-x86_64",
            "tinychannels-module-0.1.31-macos-15-x86_64.tar.gz",
            "1c72499c0cb3ef19f39cc08df86ecedbfac5ff5c646407a3d097c2726f656e42",
        ),
        (
            "macos-26-arm64",
            "tinychannels-module-0.1.31-macos-26-arm64.tar.gz",
            "f921babea4035542d8459253090b077baa23d004abe21739af804537efb1e672",
        ),
        (
            "macos-26-x86_64",
            "tinychannels-module-0.1.31-macos-26-x86_64.tar.gz",
            "3b827f24d58a566d17869adc29f8478c1b0c6c1e26b4cc54faf2cdf0f5fceb63",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinychannels-module-0.1.31-ubuntu-22.04-arm64.tar.gz",
            "619887ff89c917155ccf2f3f76f67c2d20729dad4eba8f49ebc1cf48bdbe4f0f",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinychannels-module-0.1.31-ubuntu-22.04-x86_64.tar.gz",
            "747f5da64c9fd0225ffebb284df65480ea03aff5f4a3ba507190f8295fedde18",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinychannels-module-0.1.31-ubuntu-24.04-arm64.tar.gz",
            "d5c0fc807b26eb2d22996818510a1226f6a31cdcefa12353d0b35fb510effb90",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinychannels-module-0.1.31-ubuntu-24.04-x86_64.tar.gz",
            "22d3cf8f364bd3a14f6374488e493bd619bed338de4a02f635cee369472dcbc8",
        ),
        (
            "windows-11-arm64",
            "tinychannels-module-0.1.31-windows-11-arm64.zip",
            "e06f1a3495e1879a709b8eedc4389653dffc7a2ab7e1d19c6b4f6d534f37b5fb",
        ),
        (
            "windows-2022-x86_64",
            "tinychannels-module-0.1.31-windows-2022-x86_64.zip",
            "37501850dc658d7e6b1241d8e01a804492efd02e4c9b28a558734138206c3020",
        ),
        (
            "windows-2025-x86_64",
            "tinychannels-module-0.1.31-windows-2025-x86_64.zip",
            "8e3500ed6dc073ece37269b13351ceb4f4955cf0d5745c7fb1e2cc08528ca2b1",
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
    assert_eq!(record.version, "0.3.0");
    assert_eq!(tinyvoice_bus::CONTRACT_VERSION, (1, 10));
    let expected = [
        (
            "macos-15-arm64",
            "tinyvoice-module-0.3.1-macos-15-arm64.tar.gz",
            "514588cd04dc44fc5b9acea91c36a0190c9e7c4d041b0e356107d9750279c463",
        ),
        (
            "macos-15-x86_64",
            "tinyvoice-module-0.3.1-macos-15-x86_64.tar.gz",
            "31a3219b53b72666bd4fe06a99ec07ee9b11f7402ca0234a4bbd13a7eef961c4",
        ),
        (
            "macos-26-arm64",
            "tinyvoice-module-0.3.1-macos-26-arm64.tar.gz",
            "17548346d4788a731aff976e2ce01b6ba3d11ab531b429110887f0ab968ca331",
        ),
        (
            "macos-26-x86_64",
            "tinyvoice-module-0.3.1-macos-26-x86_64.tar.gz",
            "303c4dd8a90522582effe021b8965264caf96753ad8a3d9c4d01ec733b569d6f",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinyvoice-module-0.3.1-ubuntu-22.04-arm64.tar.gz",
            "fad4e788618bf6b7607da543d56e0ec5396ccd2ca46735f9c04a3014ee7db79f",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinyvoice-module-0.3.1-ubuntu-22.04-x86_64.tar.gz",
            "0640c5aeab726295d273ec97e8f20b5c4418ae2cf6d7c6936b4a389ce5e23e92",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinyvoice-module-0.3.1-ubuntu-24.04-arm64.tar.gz",
            "30cf03c708ef3d768037232fde20299e6979782effd105fd498c091307da189c",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinyvoice-module-0.3.1-ubuntu-24.04-x86_64.tar.gz",
            "777de11fe8f2cdee80648304acd87af80079cda1a63c245a0cc96f67b89d5bd7",
        ),
        (
            "windows-11-arm64",
            "tinyvoice-module-0.3.1-windows-11-arm64.zip",
            "b63c2f3b47a97b36cc9de7c56a82988ff7d9734143ff0c1d892dae395cdfcbd3",
        ),
        (
            "windows-2022-x86_64",
            "tinyvoice-module-0.3.1-windows-2022-x86_64.zip",
            "766087bd7aa0406b85c169b207a1e78b40cf2a58235b39e357d04028ed905f2c",
        ),
        (
            "windows-2025-x86_64",
            "tinyvoice-module-0.3.1-windows-2025-x86_64.zip",
            "c361cf0fac55cad45f11f704ddbf13183edbb665b05a21802fb3cf9f7c3809ae",
        ),
    ];
    assert_eq!(record.assets.len(), expected.len());
    for (key, archive, digest) in expected {
        let asset = record.asset_for(key).unwrap();
        assert_eq!(asset.archive, archive);
        assert_eq!(asset.sha256, digest);
    }
}

#[test]
fn every_record_names_a_source_manifest_that_exists_in_its_submodule() {
    let vendor = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor");
    for record in ALL {
        let spec = &record.source;
        assert!(!spec.package.is_empty(), "{}: package", record.id);
        assert!(!spec.lib_name.is_empty(), "{}: lib_name", record.id);
        assert_eq!(spec.submodule, record.id, "{}: submodule dir", record.id);
        let manifest = vendor.join(spec.submodule).join(spec.manifest);
        assert!(
            manifest.is_file(),
            "{}: source manifest {} is missing (is the submodule initialised?)",
            record.id,
            spec.manifest
        );
    }
}

#[test]
fn source_artifact_file_names_follow_the_platform_convention() {
    for record in ALL {
        let name = record.source.artifact_filename();
        assert!(name.contains(record.source.lib_name), "{}", record.id);
        if cfg!(target_os = "linux") {
            assert!(name.starts_with("lib") && name.ends_with(".so"), "{name}");
        }
    }
}

#[test]
fn tinysearch_registry_matches_bus_contract_and_published_release() {
    let search = find("tinysearch").expect("compiled search module");
    assert_eq!(search.bus_name, tinysearch_bus::names::INTERFACE);
    assert_eq!(search.object_path, tinysearch_bus::names::OBJECT_PATH);
    assert_eq!(search.version, "0.5.0");
    assert_eq!(tinysearch_bus::CONTRACT_VERSION, (2, 2));
    assert_eq!(
        search.asset_for("ubuntu-24.04-x86_64").unwrap().sha256,
        "f078d63ada0c5051a830d803079051d9dee4c7c08c9b6e61100692ed2c63c887"
    );
}
