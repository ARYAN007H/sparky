use super::{find, ALL};
use tinybus::module::platform::candidates_for;

#[path = "registry_computer_release_tests.rs"]
mod computer_release_tests;

#[test]
#[cfg(feature = "web3")]
fn tinywallet_registry_matches_the_verified_v0241_release_manifest() {
    let wallet = find("tinywallet").expect("compiled TinyWallet module");
    assert_eq!(wallet.version, "0.24.1");
    #[cfg(feature = "web3")]
    assert_eq!(tinywallet_bus::CONTRACT_VERSION, (1, 14));
    assert_eq!(
        wallet.release_url,
        "https://github.com/tinyhumansai/tinywallet/releases/tag/v0.24.1"
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
            "tinywallet-module-0.24.1-archlinux-arm64.tar.gz",
            "f0e013d305d35744911e4e88ae391bd5b698ca9b5d4b5965ff2cf7e038022a55",
        ),
        (
            "archlinux-x86_64",
            "tinywallet-module-0.24.1-archlinux-x86_64.tar.gz",
            "e8feeacc13150230db39885471868f3cb983d32cdca3b54076cfa3b7b99ca291",
        ),
        (
            "fedora-43-arm64",
            "tinywallet-module-0.24.1-fedora-43-arm64.tar.gz",
            "a5eda11c12b04838101113bb691809cccaab4f02b63927ed87b9ae23354e8024",
        ),
        (
            "fedora-43-x86_64",
            "tinywallet-module-0.24.1-fedora-43-x86_64.tar.gz",
            "81806447a07fb1e5f2e2e11c44cc3bb673b9583836b468086abda33cac7d74c6",
        ),
        (
            "fedora-44-arm64",
            "tinywallet-module-0.24.1-fedora-44-arm64.tar.gz",
            "1b46b6df147080672ed2f87c9b560f3c4ab1d5edabe16f98e66e3cc1df0556fc",
        ),
        (
            "fedora-44-x86_64",
            "tinywallet-module-0.24.1-fedora-44-x86_64.tar.gz",
            "021973415b5e6ebde7f903e9ecd7727a3511fe22bf5f4452720eb6d6b8898a7f",
        ),
        (
            "macos-15-arm64",
            "tinywallet-module-0.24.1-macos-15-arm64.tar.gz",
            "018451ecd682ca8cdd45667d9d7a10e8200a666f1ce5e76c7784f1ed1fe2a59b",
        ),
        (
            "macos-15-x86_64",
            "tinywallet-module-0.24.1-macos-15-x86_64.tar.gz",
            "8c9887fe1a321e0f13b1af3f9129e6db02186fd791d6769e3a80bae964ddb240",
        ),
        (
            "macos-26-arm64",
            "tinywallet-module-0.24.1-macos-26-arm64.tar.gz",
            "d0896d7fbb8936542cdfc4c0d138a262982d599ec015401a6dbd1d4410a39d3a",
        ),
        (
            "macos-26-x86_64",
            "tinywallet-module-0.24.1-macos-26-x86_64.tar.gz",
            "19409a538aa0fd04f246ebcbaacd4aefc7c55279d49f3d066a44257432617314",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinywallet-module-0.24.1-ubuntu-22.04-arm64.tar.gz",
            "0b7da4811e31e2d7239c62fdb30c8ac4c45667b596b27033f4f3915655fccbe7",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinywallet-module-0.24.1-ubuntu-22.04-x86_64.tar.gz",
            "5b1e62a9fa88939cf1711dc0480a6298d240ce2e49cbd4a448a2f967a2f30255",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinywallet-module-0.24.1-ubuntu-24.04-arm64.tar.gz",
            "e50ffef6a61dfbfffc0be0ad32e8388ee72c8803f63b71f435ddc6c47c337650",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinywallet-module-0.24.1-ubuntu-24.04-x86_64.tar.gz",
            "11f66fa19923f1fd151252294b947becf6eb5edafe6a7750681daf74c028d28c",
        ),
        (
            "windows-11-arm64",
            "tinywallet-module-0.24.1-windows-11-arm64.zip",
            "ad8f432919fa78ca7a0828b8c3ad02fcd1b65855cbaf1f76355dcd0423ff7415",
        ),
        (
            "windows-2022-x86_64",
            "tinywallet-module-0.24.1-windows-2022-x86_64.zip",
            "03badd28042ff3a211a489bafb42ac1fa90bb320de66bb73ef5317c7b4c560d3",
        ),
        (
            "windows-2025-x86_64",
            "tinywallet-module-0.24.1-windows-2025-x86_64.zip",
            "53fc1c72e207769fa471373320b0ca73c067c2f6b32a65e73f8fc7ef18a0fa80",
        ),
    ];
    actual.sort_unstable();
    published.sort_unstable();
    assert_eq!(actual, published);
}

#[test]
fn tinybox_registry_matches_the_published_v021_release_manifest() {
    let record = find("tinybox").expect("compiled TinyBox module");
    assert_eq!(record.version, "0.2.1");
    assert_eq!(tinybox_bus::CONTRACT_VERSION, (2, 0));
    assert_eq!(
        record.release_url,
        "https://github.com/tinyhumansai/tinybox/releases/tag/v0.2.1"
    );

    let actual = record
        .assets
        .iter()
        .map(|asset| (asset.host_key, asset.archive, asset.sha256))
        .collect::<Vec<_>>();
    let published = [
        (
            "archlinux-arm64",
            "tinybox-0.2.1-archlinux-arm64.tar.gz",
            "ff2404f24840632a00d8107d02ea4ac258cc56b77e650653b6c440e769da282a",
        ),
        (
            "archlinux-x86_64",
            "tinybox-0.2.1-archlinux-x86_64.tar.gz",
            "3b8f14a175fb0e73578047192f5aeefe3f76014d5a1baf3fb85d63f300ee48fc",
        ),
        (
            "fedora-43-arm64",
            "tinybox-0.2.1-fedora-43-arm64.tar.gz",
            "8b53c7f60cfddfadda3f0269aee15676069d727c1fc8c10f7aa8211baa55292a",
        ),
        (
            "fedora-43-x86_64",
            "tinybox-0.2.1-fedora-43-x86_64.tar.gz",
            "aaafcd8b33a844e44362f7566d4c384c6a541abfd5888b0e415158c8d5bc0ea4",
        ),
        (
            "fedora-44-arm64",
            "tinybox-0.2.1-fedora-44-arm64.tar.gz",
            "5441fda12064b029e745229e0c9fd2fd64dcc8e30f3416f5cdbe877583524295",
        ),
        (
            "fedora-44-x86_64",
            "tinybox-0.2.1-fedora-44-x86_64.tar.gz",
            "1d48b6b734bba68dc3160b85f0cb9e8f7e96e36acab18afd30f71a20e002df2b",
        ),
        (
            "macos-15-arm64",
            "tinybox-0.2.1-macos-15-arm64.tar.gz",
            "8d59ecb80220913770a25ab32116c6ff8e55cdb6fac1db634eab3b8767e22690",
        ),
        (
            "macos-15-x86_64",
            "tinybox-0.2.1-macos-15-x86_64.tar.gz",
            "4c1f22592913ca5c57a840cca65a638493cb76b95cee4d88b7367d762a5afa01",
        ),
        (
            "macos-26-arm64",
            "tinybox-0.2.1-macos-26-arm64.tar.gz",
            "daaec8cbb2a8060783c53621794ae6c03b62c9f5295753c5040fe0bbdf75b98e",
        ),
        (
            "macos-26-x86_64",
            "tinybox-0.2.1-macos-26-x86_64.tar.gz",
            "de5c07812e54b5ae421bd28a0071566c36d124c07f87dde2eef6e0df83745350",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinybox-0.2.1-ubuntu-22.04-arm64.tar.gz",
            "488082b3619698a82dfe14d13547ffed90aabc9278f64384f39e7b2f3b8ec020",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinybox-0.2.1-ubuntu-22.04-x86_64.tar.gz",
            "d6dc0320b2944ebc3d1972d80d4a47e459c5ebb24d0c41ef0f64e5f442f90685",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinybox-0.2.1-ubuntu-24.04-arm64.tar.gz",
            "ff1f9e0b9658c463a4b8f1e6aeb36cc7f9bc8b97117e8d17fd8986245a8e5b97",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinybox-0.2.1-ubuntu-24.04-x86_64.tar.gz",
            "87f9017e0384babe52a43cc9002a23d13558cdbab2d1807caf6d48152684871a",
        ),
        (
            "windows-11-arm64",
            "tinybox-0.2.1-windows-11-arm64.zip",
            "86ab6d28269b9e7eb59b54f6534fdc3e9bff67ebfcf66311a0ce6fdd16eaf1d1",
        ),
        (
            "windows-2022-x86_64",
            "tinybox-0.2.1-windows-2022-x86_64.zip",
            "68b7089dd4141434d3dfc91b32fae688cae549a297d0a933797a3b2b43e92795",
        ),
        (
            "windows-2025-x86_64",
            "tinybox-0.2.1-windows-2025-x86_64.zip",
            "059e7fbc993627d866c6b57d59286fe80eefc4ebe6542d4535d0e0f7d08e22a9",
        ),
    ];

    assert_eq!(actual.len(), 17);
    assert_eq!(actual, published);
}
#[test]
fn tinymcp_registry_matches_the_published_v0201_release_manifest() {
    let record = find("tinymcp").expect("compiled TinyMCP module");
    assert_eq!(record.version, "0.20.1");
    assert_eq!(tinymcp_bus::CONTRACT_VERSION, (1, 28));
    assert_eq!(
        record.release_url,
        "https://github.com/tinyhumansai/tinymcp/releases/tag/v0.20.1"
    );
    let actual = record
        .assets
        .iter()
        .map(|asset| (asset.host_key, asset.archive, asset.sha256))
        .collect::<Vec<_>>();
    let published = [
        (
            "archlinux-rolling-arm64",
            "tinymcp-0.20.1-archlinux-arm64.tar.gz",
            "d902e2ac55e9ef1cb0e88dbcccdf22d2f544e0097517331b85e561664f1087aa",
        ),
        (
            "archlinux-rolling-x86_64",
            "tinymcp-0.20.1-archlinux-x86_64.tar.gz",
            "7530448fc721425da43e795b8bd1a9d679cc04fa8eb5d48363db92cc3916f37d",
        ),
        (
            "fedora-43-arm64",
            "tinymcp-0.20.1-fedora-43-arm64.tar.gz",
            "d88a6951ac5544afc136326e4756503c42f17038023a11684c67af74e56a6593",
        ),
        (
            "fedora-43-x86_64",
            "tinymcp-0.20.1-fedora-43-x86_64.tar.gz",
            "f410705d507e40686019f4fb35a2c254b5f5789da18aa2c3b287f1272a1228e6",
        ),
        (
            "fedora-44-arm64",
            "tinymcp-0.20.1-fedora-44-arm64.tar.gz",
            "3b58fd99d1062adad06c64d8c4ade1bef316ffaae3076b49cf312db82f5d8ffa",
        ),
        (
            "fedora-44-x86_64",
            "tinymcp-0.20.1-fedora-44-x86_64.tar.gz",
            "9ddba2d03dc08774fc9eb84a483c752499ea10b29e63b658ed45a21cd4c6fcb0",
        ),
        (
            "macos-15-arm64",
            "tinymcp-0.20.1-macos-15-arm64.tar.gz",
            "9f2794db8966cd1e9b04d3c9f0e00bb57646e094394ec468d23aa4e6f9c83a1e",
        ),
        (
            "macos-15-x86_64",
            "tinymcp-0.20.1-macos-15-x86_64.tar.gz",
            "e66f2c4419b9a407db3fb1a1feb906bb9e90259748e69addc6b815a8f97fab88",
        ),
        (
            "macos-26-arm64",
            "tinymcp-0.20.1-macos-26-arm64.tar.gz",
            "7a19543c53bef5f2b955b49216f9679418c7f9855870db281616991493c27408",
        ),
        (
            "macos-26-x86_64",
            "tinymcp-0.20.1-macos-26-x86_64.tar.gz",
            "dea4e31e951173a7a41e0ed6abe4f03aac894f70767d264a573f0d74cda7bf59",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinymcp-0.20.1-ubuntu-22.04-arm64.tar.gz",
            "e96f44729fa596ecafa42f99c7405963a652bc9862a38414eb9a0e2d9a774031",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinymcp-0.20.1-ubuntu-22.04-x86_64.tar.gz",
            "73746b2459c93129c6ad75cae79e0940d60b8b20e153c9ddb5dba8bd39d24412",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinymcp-0.20.1-ubuntu-24.04-arm64.tar.gz",
            "73e66fb2cb1460ea5b08ff9e9e70fae2d8f7e2fe2cc837adbd4e88e3e07290df",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinymcp-0.20.1-ubuntu-24.04-x86_64.tar.gz",
            "edc924e1ade72ca3c6e193048302a5f0aaadde10554826b95ddb01247c83ca2b",
        ),
        (
            "windows-11-arm64",
            "tinymcp-0.20.1-windows-11-arm64.zip",
            "839c7ab05574d544769bb8b1042d8313edf755da89484f2086db36e3095fc397",
        ),
        (
            "windows-2022-x86_64",
            "tinymcp-0.20.1-windows-2022-x86_64.zip",
            "4a3012be1c2c19a5f5c105e4f8cea23f6e5a636aa582322b7eba4e01b90cd26e",
        ),
        (
            "windows-2025-x86_64",
            "tinymcp-0.20.1-windows-2025-x86_64.zip",
            "996733bf31598c7e2103225315f7c4664a3cb566b44bda843aa6db2ba2e00a48",
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
    assert_eq!(record.version, "0.1.39");
    assert_eq!(tinychannels_bus::CONTRACT_VERSION, 21);
    assert_eq!(
        tinychannels_bus::module_relay::MODULE_OPERATION_VERSION,
        (1, 21)
    );
    let expected = [
        (
            "macos-15-arm64",
            "tinychannels-module-0.1.39-macos-15-arm64.tar.gz",
            "2a360880d72239bb370fd25c50f384f7c07a6a349d2da7854645b88fe2bc567f",
        ),
        (
            "macos-15-x86_64",
            "tinychannels-module-0.1.39-macos-15-x86_64.tar.gz",
            "672c0aa2a34954c0445fdcf532f4507468288f045925950a4c5cd726a34a16b8",
        ),
        (
            "macos-26-arm64",
            "tinychannels-module-0.1.39-macos-26-arm64.tar.gz",
            "2c89dee2d3b99f6879ede752b29ffce2532ac7b4c37f0d1cd9f51693d97ca807",
        ),
        (
            "macos-26-x86_64",
            "tinychannels-module-0.1.39-macos-26-x86_64.tar.gz",
            "725b3b02bf312523940551b8456a043a6b0fec56c4738ee751776206438d2b76",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinychannels-module-0.1.39-ubuntu-22.04-arm64.tar.gz",
            "a0356ad855de475cdb56bb60a501a75adceb07a860ac06c15ce4aa57dbc35ab2",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinychannels-module-0.1.39-ubuntu-22.04-x86_64.tar.gz",
            "d14e379f625ed1dad0f2f0d5ca6e76d57caa2c678a929f11de096a55df8ebbbc",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinychannels-module-0.1.39-ubuntu-24.04-arm64.tar.gz",
            "89dac4db8429b0eb3a99a5edc8fd56e1edeef76ffcfc996aa073810a05c6b5a4",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinychannels-module-0.1.39-ubuntu-24.04-x86_64.tar.gz",
            "04c53b7649b1f0323fc4af0bed6f7518c1feadbf570262c5a96a0c2879f51287",
        ),
        (
            "windows-11-arm64",
            "tinychannels-module-0.1.39-windows-11-arm64.zip",
            "e9da5f08300cf40e1f23f7bea1abb5322b69425178e0b59df6eaa3e75e16aed5",
        ),
        (
            "windows-2022-x86_64",
            "tinychannels-module-0.1.39-windows-2022-x86_64.zip",
            "b4fb4dfb2019593ddd1dc336d660bd3926ba39dc2e0e8849dc8c06053acea53b",
        ),
        (
            "windows-2025-x86_64",
            "tinychannels-module-0.1.39-windows-2025-x86_64.zip",
            "4adefa0e8589a52f2cd92d5839c78b52476f03734a6f7f534283f2b6b2ad436c",
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
    assert_eq!(record.version, "0.3.4");
    assert_eq!(tinyvoice_bus::CONTRACT_VERSION, (1, 10));
    let expected = [
        (
            "macos-15-arm64",
            "tinyvoice-module-0.3.4-macos-15-arm64.tar.gz",
            "fa2c380ce034e31f96f3faae4e7d7651e9f2df5614977cae7c5bf0339dd2a8ab",
        ),
        (
            "macos-15-x86_64",
            "tinyvoice-module-0.3.4-macos-15-x86_64.tar.gz",
            "29ec1e0097a6da61e2da5c6f6d2290ddb74217a72840fd8e22d51cacdd288f6f",
        ),
        (
            "macos-26-arm64",
            "tinyvoice-module-0.3.4-macos-26-arm64.tar.gz",
            "c9c729ba646a5069b30e4b84cdeb53f3ec274406f10c10955411862eacc246f1",
        ),
        (
            "macos-26-x86_64",
            "tinyvoice-module-0.3.4-macos-26-x86_64.tar.gz",
            "5ad4ca051265063338510d891d894c347d76356828605f8c5f793986acddfdda",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinyvoice-module-0.3.4-ubuntu-22.04-arm64.tar.gz",
            "bc31815f65fff0e44b47f0da8eac10a4a404fb516d0c093649ed4e1de005e925",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinyvoice-module-0.3.4-ubuntu-22.04-x86_64.tar.gz",
            "35e5529cd3e6ab3d6ca062bc110181526851ab13d59c34d8a3febd93d21262fa",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinyvoice-module-0.3.4-ubuntu-24.04-arm64.tar.gz",
            "1ed67f4f36538e5ad5052d796a5c2bc647b0edd2cf78366c8d9be521e4e516b7",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinyvoice-module-0.3.4-ubuntu-24.04-x86_64.tar.gz",
            "ba5417d7442d3cc8c800f89f0e2e765c4f30c30bf8257a8030022bc2416ca175",
        ),
        (
            "windows-11-arm64",
            "tinyvoice-module-0.3.4-windows-11-arm64.zip",
            "c9bb31d5bbaa0d614262a92ceaa793755bb651992f6b18c768062154b42dbebd",
        ),
        (
            "windows-2022-x86_64",
            "tinyvoice-module-0.3.4-windows-2022-x86_64.zip",
            "b0aff0e3c11fc825078fa62a4ead24cec41eb938ac6d45f6231e468e6bb1283a",
        ),
        (
            "windows-2025-x86_64",
            "tinyvoice-module-0.3.4-windows-2025-x86_64.zip",
            "a4acbbd05d8965de326e9f37241c28fa14cef696062b013720c96adbd56eb4be",
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
