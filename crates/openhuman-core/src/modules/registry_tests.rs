use super::{find, ALL};
use tinybus::module::platform::candidates_for;

#[test]
fn tinycomputer_registry_matches_bus_contract_and_published_release() {
    let desktop = find("tinycomputer").expect("compiled computer module");
    assert_eq!(desktop.bus_name, tinycomputer_bus::names::INTERFACE);
    assert_eq!(desktop.object_path, tinycomputer_bus::names::OBJECT_PATH);
    assert_eq!(desktop.version, "0.13.1");
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
fn tinywallet_registry_matches_the_verified_v0233_release_manifest() {
    let wallet = find("tinywallet").expect("compiled TinyWallet module");
    assert_eq!(wallet.version, "0.23.3");
    #[cfg(feature = "web3")]
    assert_eq!(tinywallet_bus::CONTRACT_VERSION, (1, 14));
    assert_eq!(
        wallet.release_url,
        "https://github.com/tinyhumansai/tinywallet/releases/tag/v0.23.3"
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
            "tinywallet-module-0.23.3-archlinux-arm64.tar.gz",
            "e2adef42c2fcc9794cad1eefd0d5676cf2961a7772d0d3d5ca03890094f89924",
        ),
        (
            "archlinux-x86_64",
            "tinywallet-module-0.23.3-archlinux-x86_64.tar.gz",
            "2a35c9b79cfc15e1917e7e1957588511ffc0ec21b2ad167cbf9e53213ca4511c",
        ),
        (
            "fedora-43-arm64",
            "tinywallet-module-0.23.3-fedora-43-arm64.tar.gz",
            "4038195b53b79eda71a7716ab3e64b63904911915b7f44a967ceff274074a947",
        ),
        (
            "fedora-43-x86_64",
            "tinywallet-module-0.23.3-fedora-43-x86_64.tar.gz",
            "419a9a57d9ef245a852c49b32be4c07f61d58dd6315ae8c23d27ba1493615368",
        ),
        (
            "fedora-44-arm64",
            "tinywallet-module-0.23.3-fedora-44-arm64.tar.gz",
            "0a84fb19f6c1386b35b71677b52f30bf5524f0689a9afc9e79d17b1889d24ad7",
        ),
        (
            "fedora-44-x86_64",
            "tinywallet-module-0.23.3-fedora-44-x86_64.tar.gz",
            "4a4b8e13468ade4ef23303af53e9f5121cd3547faa9cf8accb086c0290ab98fb",
        ),
        (
            "macos-15-arm64",
            "tinywallet-module-0.23.3-macos-15-arm64.tar.gz",
            "aee3d767944171ed68cab981092d4ae5fa0678dc6d85f3e450bc618c42682af9",
        ),
        (
            "macos-15-x86_64",
            "tinywallet-module-0.23.3-macos-15-x86_64.tar.gz",
            "cc617f3443a15c69e1c3a2b3f94b1129262a4c2dbc76d3e76ae07b1153024bab",
        ),
        (
            "macos-26-arm64",
            "tinywallet-module-0.23.3-macos-26-arm64.tar.gz",
            "a3e16793b25abafbb2a26ed6b2223ce499f3f16da66bc01898307ca6fc049581",
        ),
        (
            "macos-26-x86_64",
            "tinywallet-module-0.23.3-macos-26-x86_64.tar.gz",
            "f2dc9871afcffdf7bac9f8fcbf9f4e13776e992ad94833855fd9d7944acc132d",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinywallet-module-0.23.3-ubuntu-22.04-arm64.tar.gz",
            "58e0105f0c151bbe3aa44aa0e29539d6762a71be73bff9875f8e1d389c13500b",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinywallet-module-0.23.3-ubuntu-22.04-x86_64.tar.gz",
            "4d9d732760e95e9b8bb614a3282746053b4b7e2f3c599a395215836505c15c91",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinywallet-module-0.23.3-ubuntu-24.04-arm64.tar.gz",
            "cec6a9eb6ca864140187463e29f65a7f374cb192d75edfcbead8a101b7bb7fff",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinywallet-module-0.23.3-ubuntu-24.04-x86_64.tar.gz",
            "f2ae10459327d55d837b2016fff448fb855cf57eeb139d7e9a173066c9b00921",
        ),
        (
            "windows-11-arm64",
            "tinywallet-module-0.23.3-windows-11-arm64.zip",
            "21d7c857b98359d27715c70ed9983ae4dd4138648b1aa55fa7a726c5896dc528",
        ),
        (
            "windows-2022-x86_64",
            "tinywallet-module-0.23.3-windows-2022-x86_64.zip",
            "ba0c2b7c3ba8e7dbe51c854643616d27c6a9433600265f7e7d0a6cf9a03a7a7a",
        ),
        (
            "windows-2025-x86_64",
            "tinywallet-module-0.23.3-windows-2025-x86_64.zip",
            "ce3748d7d885356040d2121357ad5b8945f28d05bde1ec65540c06524a9c328b",
        ),
    ];
    actual.sort_unstable();
    published.sort_unstable();
    assert_eq!(actual, published);
}

#[test]
fn tinybox_registry_matches_the_published_v0133_release_manifest() {
    let record = find("tinybox").expect("compiled TinyBox module");
    assert_eq!(record.version, "0.1.33");
    assert_eq!(tinybox_bus::CONTRACT_VERSION, (1, 18));
    assert_eq!(
        record.release_url,
        "https://github.com/tinyhumansai/tinybox/releases/tag/v0.1.33"
    );

    let actual = record
        .assets
        .iter()
        .map(|asset| (asset.host_key, asset.archive, asset.sha256))
        .collect::<Vec<_>>();
    let published = [
        (
            "archlinux-arm64",
            "tinybox-0.1.33-archlinux-arm64.tar.gz",
            "b3857f408972b21ad2e894820d2e4bdbcd43c380b2f4da89d633740417a72943",
        ),
        (
            "archlinux-x86_64",
            "tinybox-0.1.33-archlinux-x86_64.tar.gz",
            "203c8fbc59b104e0f0e499909fbbe2c98355fedc01f00369e01531da3cb40eec",
        ),
        (
            "fedora-43-arm64",
            "tinybox-0.1.33-fedora-43-arm64.tar.gz",
            "97fd2fceaea5d5d13ab244458e7226c9a7ace87cc41d2b72361b10a06aff9610",
        ),
        (
            "fedora-43-x86_64",
            "tinybox-0.1.33-fedora-43-x86_64.tar.gz",
            "61228f1c18e0729d0e5e6d1b1b8020c2e0c6378396d85f908f38a3b728a1ca76",
        ),
        (
            "fedora-44-arm64",
            "tinybox-0.1.33-fedora-44-arm64.tar.gz",
            "11e11b442a64185c0bc21e7b67db61eff7a1654ce549a09b25111a4115b2a54f",
        ),
        (
            "fedora-44-x86_64",
            "tinybox-0.1.33-fedora-44-x86_64.tar.gz",
            "2103c93dec85979f53c81ed5f6d844eae15b638de5242aa755041fa549abe9f4",
        ),
        (
            "macos-15-arm64",
            "tinybox-0.1.32-macos-15-arm64.tar.gz",
            "db20c950d43e77d2255110cc289594371a957ffb073dea24de10e4ec68949158",
        ),
        (
            "macos-15-x86_64",
            "tinybox-0.1.32-macos-15-x86_64.tar.gz",
            "93b5f3296fe2b41c40343cf845289705cf02081a5fca26fafea5c0ebb161c843",
        ),
        (
            "macos-26-arm64",
            "tinybox-0.1.32-macos-26-arm64.tar.gz",
            "b387f65e0206821edfa0b4ff1459a407261c97d2fa7ee332e5ce9a48e9f667e1",
        ),
        (
            "macos-26-x86_64",
            "tinybox-0.1.32-macos-26-x86_64.tar.gz",
            "1258a4ee08335a402989c265a66421fa466a43fd19798fcd13a9672a44acb338",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinybox-0.1.32-ubuntu-22.04-arm64.tar.gz",
            "8fcec28c0c6c168b2ed2085e786d5eb63af783d2be4284d76e6366f5a0055b14",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinybox-0.1.32-ubuntu-22.04-x86_64.tar.gz",
            "641d5463d642bef40d25978d8e19f5bd0a2492e664a12d9d60b0ec9b2c8cf49a",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinybox-0.1.32-ubuntu-24.04-arm64.tar.gz",
            "52541665159e1a06b1102ee9fed198d8cc9822fa045b3e813e8e9b98a048b9cb",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinybox-0.1.32-ubuntu-24.04-x86_64.tar.gz",
            "a537795061172a57c9269a530ba84ee39e184f01aa433d2254b20b0c6408ec25",
        ),
        (
            "windows-11-arm64",
            "tinybox-0.1.32-windows-11-arm64.zip",
            "356b737eefaa6132bcb226692aa9681b6ead6b5841d83f0130be034a25bed225",
        ),
        (
            "windows-2022-x86_64",
            "tinybox-0.1.32-windows-2022-x86_64.zip",
            "172a69e61861a98be91245a82179292f564fb682d180af219f2dddbce54354c6",
        ),
        (
            "windows-2025-x86_64",
            "tinybox-0.1.32-windows-2025-x86_64.zip",
            "24c8fe29cb4659d79a1cb4a1ed9635fa0db8273f0e7dfe0880ee764032e4dc85",
        ),
    ];

    assert_eq!(actual.len(), 17);
    assert_eq!(actual, published);
}
#[test]
fn tinymcp_registry_matches_the_published_v0184_release_manifest() {
    let record = find("tinymcp").expect("compiled TinyMCP module");
    assert_eq!(record.version, "0.18.4");
    assert_eq!(tinymcp_bus::CONTRACT_VERSION, (1, 24));
    assert_eq!(
        record.release_url,
        "https://github.com/tinyhumansai/tinymcp/releases/tag/v0.18.4"
    );
    let actual = record
        .assets
        .iter()
        .map(|asset| (asset.host_key, asset.archive, asset.sha256))
        .collect::<Vec<_>>();
    let published = [
        (
            "archlinux-rolling-arm64",
            "tinymcp-0.18.4-archlinux-arm64.tar.gz",
            "8a434d356550b9ea7b11b2cddf5625e850cf48af69605710b2b4276ed5e75b7d",
        ),
        (
            "archlinux-rolling-x86_64",
            "tinymcp-0.18.4-archlinux-x86_64.tar.gz",
            "39c2c8d4948e5c875810211de5f959aaaf9494f817f9c93a2a9fdd4f99694684",
        ),
        (
            "fedora-43-arm64",
            "tinymcp-0.18.3-fedora-43-arm64.tar.gz",
            "7238a7eeb8828742f34fb0584ddd7ff9b3243f0e08a083849a2825a4babf2bd8",
        ),
        (
            "fedora-43-x86_64",
            "tinymcp-0.18.3-fedora-43-x86_64.tar.gz",
            "0ac31264b19d93d5a5a43893c9fd6af08c61bc87edc0309928cb11758911e231",
        ),
        (
            "fedora-44-arm64",
            "tinymcp-0.18.3-fedora-44-arm64.tar.gz",
            "1c388963a6440edbe5f19100c54b454b85f10e8d586a723131800375ecb64681",
        ),
        (
            "fedora-44-x86_64",
            "tinymcp-0.18.3-fedora-44-x86_64.tar.gz",
            "14c3fb5e0ef6bc98a35000a8bf628718347df52a40c116f3f76305c09e117768",
        ),
        (
            "macos-15-arm64",
            "tinymcp-0.18.3-macos-15-arm64.tar.gz",
            "6bdda01a7a74411591641f9e321a712b329fe1a4cf77e724386f8ac6b3cab79b",
        ),
        (
            "macos-15-x86_64",
            "tinymcp-0.18.3-macos-15-x86_64.tar.gz",
            "ffe5efb4148be1658e9fef091df86cdd7f6378dd54dd0bd9e41b70b564a659a9",
        ),
        (
            "macos-26-arm64",
            "tinymcp-0.18.3-macos-26-arm64.tar.gz",
            "62e74c411fa3d86638ea157c24efb8cbce0cef44b13862b3654150292186e0f5",
        ),
        (
            "macos-26-x86_64",
            "tinymcp-0.18.3-macos-26-x86_64.tar.gz",
            "625b0c4ea61c68c64aae80ecdcb73a1993d4a27dcf371c939ce58b879fc8558d",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinymcp-0.18.3-ubuntu-22.04-arm64.tar.gz",
            "02cc3f971c8e1e5225b34f64a8602c004f40aaede0ffa6fa7eb2f7cc1b6ebd48",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinymcp-0.18.3-ubuntu-22.04-x86_64.tar.gz",
            "388bda6840e622b6f0035b829a4f043400d2a0d611085390c2e3ff3bc07f0dab",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinymcp-0.18.3-ubuntu-24.04-arm64.tar.gz",
            "696294c34bf323b1a4723dd38548494de4096bc3c7b134a66d7e28cb67a1bc90",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinymcp-0.18.3-ubuntu-24.04-x86_64.tar.gz",
            "5448a1ca48e8409679900e704c161d411ef70754d3a1fd3f623540f6144be1c3",
        ),
        (
            "windows-11-arm64",
            "tinymcp-0.18.3-windows-11-arm64.zip",
            "8ce944ed35571d765059cac220c52fe8c81c87a1d5acd41fdfc935562463a10d",
        ),
        (
            "windows-2022-x86_64",
            "tinymcp-0.18.3-windows-2022-x86_64.zip",
            "1ed99a67ac1caf0bc492d39b2f98e2f4c55bdfc7165cdb96352defd9a2104936",
        ),
        (
            "windows-2025-x86_64",
            "tinymcp-0.18.3-windows-2025-x86_64.zip",
            "a0123aef817aef2e1ad5725fe989d6b0e49ebeefc0d32db1b31a7e43982d84c1",
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
    assert_eq!(record.version, "0.1.34");
    assert_eq!(tinychannels_bus::CONTRACT_VERSION, 21);
    assert_eq!(
        tinychannels_bus::module_relay::MODULE_OPERATION_VERSION,
        (1, 21)
    );
    let expected = [
        (
            "macos-15-arm64",
            "tinychannels-module-0.1.33-macos-15-arm64.tar.gz",
            "770cd74e7d6586aa484383d66d7b51fcd29cc5ab417c11bee0af69b0c6ab45ee",
        ),
        (
            "macos-15-x86_64",
            "tinychannels-module-0.1.33-macos-15-x86_64.tar.gz",
            "12e3a51b0e4bef0cd53b290daa950cbe1f3d01f7f5f38b69a72a0addda3d91e8",
        ),
        (
            "macos-26-arm64",
            "tinychannels-module-0.1.33-macos-26-arm64.tar.gz",
            "5a88af85efd608b13c7a5cb79153c66a417fb69bc35712a72c49bbb029ecf473",
        ),
        (
            "macos-26-x86_64",
            "tinychannels-module-0.1.33-macos-26-x86_64.tar.gz",
            "c2d632d72e0927d241fb14b35a587c4b9ce1ff4bd35fd3fb5c06817059ef7578",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinychannels-module-0.1.33-ubuntu-22.04-arm64.tar.gz",
            "742b1aebbe5e3d383302aeb7a05f4fcbb6c8822fb882adb7431140a720dd94b3",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinychannels-module-0.1.33-ubuntu-22.04-x86_64.tar.gz",
            "ebf0706cfcfd13f1099f225de279d9a28d290db3c3f1c93054a9048509759cc5",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinychannels-module-0.1.33-ubuntu-24.04-arm64.tar.gz",
            "4ff242d961bb7a43e0f077def8d19fa43426f4e101772a966a1ff9f448408256",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinychannels-module-0.1.33-ubuntu-24.04-x86_64.tar.gz",
            "e9e071303dab85fc5421c222ce1aa7e16f0af9301f3c7547fee0cb266b2f1da5",
        ),
        (
            "windows-11-arm64",
            "tinychannels-module-0.1.33-windows-11-arm64.zip",
            "19ba7ee1b91c5c519b3a65809d3a0459b7df572f3e1916c647e65292eebd18c4",
        ),
        (
            "windows-2022-x86_64",
            "tinychannels-module-0.1.33-windows-2022-x86_64.zip",
            "7a9499773c190d137ad31572947b7843b591efd45276675efeaa76c57d158f24",
        ),
        (
            "windows-2025-x86_64",
            "tinychannels-module-0.1.33-windows-2025-x86_64.zip",
            "fc1954c4258175bd3ff27780a18ad6fa064176ee84e19f076bd327e86ff87f3b",
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
    assert_eq!(record.version, "0.3.2");
    assert_eq!(tinyvoice_bus::CONTRACT_VERSION, (1, 10));
    let expected = [
        (
            "macos-15-arm64",
            "tinyvoice-module-0.3.2-macos-15-arm64.tar.gz",
            "316cb2b9b9a1630f2c815744121e3bbc1d0df58364bb012dd6f11f78978ba947",
        ),
        (
            "macos-15-x86_64",
            "tinyvoice-module-0.3.2-macos-15-x86_64.tar.gz",
            "51045754d67386db5b09a19714ec8b1f319ad9896bb3f936325d7935ce5dce0e",
        ),
        (
            "macos-26-arm64",
            "tinyvoice-module-0.3.2-macos-26-arm64.tar.gz",
            "2a4ee8f87229422019adfd28f07ac26d5e2e78165550b04f0e563764a66a429e",
        ),
        (
            "macos-26-x86_64",
            "tinyvoice-module-0.3.2-macos-26-x86_64.tar.gz",
            "4199913e36c12987a82606f98c93e2a85e086aaa17380469a9aa68b719c853d9",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinyvoice-module-0.3.2-ubuntu-22.04-arm64.tar.gz",
            "533a3715f5efad9911e7768e8280bbe724c60785e0ff67939867ae21a20f690e",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinyvoice-module-0.3.2-ubuntu-22.04-x86_64.tar.gz",
            "d173a63a6b20e0441d18d639906ec9c74f96178c50e274ebd32bf94abbf5260b",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinyvoice-module-0.3.2-ubuntu-24.04-arm64.tar.gz",
            "f2145cd4bf729b5b6ad563687b061557d502c786ba935a65433e4338adf75a7a",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinyvoice-module-0.3.2-ubuntu-24.04-x86_64.tar.gz",
            "6ee278898d33f3b3ca7a9bf8a313dc4ab4e207eabf970608d37f56d825587519",
        ),
        (
            "windows-11-arm64",
            "tinyvoice-module-0.3.2-windows-11-arm64.zip",
            "b3cbff9b477495027839b0266201299b0db5cb585a9e41b3110cf46f16c35c99",
        ),
        (
            "windows-2022-x86_64",
            "tinyvoice-module-0.3.2-windows-2022-x86_64.zip",
            "7b7a8a8e04a57adc306b2b840bff43ea6a27090e2b089b74cfcf451f18172456",
        ),
        (
            "windows-2025-x86_64",
            "tinyvoice-module-0.3.2-windows-2025-x86_64.zip",
            "04cf6b21ed7d73375181c6cf5a18fa52a50fb09049031f0926cd46c3bcf33431",
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
    assert_eq!(search.version, "0.5.1");
    assert_eq!(tinysearch_bus::CONTRACT_VERSION, (2, 2));
    assert_eq!(
        search.asset_for("ubuntu-24.04-x86_64").unwrap().sha256,
        "f078d63ada0c5051a830d803079051d9dee4c7c08c9b6e61100692ed2c63c887"
    );
}
