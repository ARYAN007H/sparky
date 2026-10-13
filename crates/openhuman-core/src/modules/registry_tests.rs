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
#[cfg(feature = "web3")]
fn tinywallet_registry_matches_the_verified_v020_release_manifest() {
    let wallet = find("tinywallet").expect("compiled TinyWallet module");
    assert_eq!(wallet.version, "0.20.0");
    #[cfg(feature = "web3")]
    assert_eq!(tinywallet_bus::CONTRACT_VERSION, (1, 14));
    assert_eq!(
        wallet.release_url,
        "https://github.com/tinyhumansai/tinywallet/releases/tag/v0.20.0"
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
            "tinywallet-module-0.20.0-archlinux-arm64.tar.gz",
            "04641e9a3e69a28d7b29b7055cd6b6f1e710f117e37392426a22eb1867e1a0c8",
        ),
        (
            "archlinux-x86_64",
            "tinywallet-module-0.20.0-archlinux-x86_64.tar.gz",
            "50dbe4f6b296941971712eacc831a9ff5947dfaca3be664687e6a1ae120f9b9d",
        ),
        (
            "fedora-43-arm64",
            "tinywallet-module-0.20.0-fedora-43-arm64.tar.gz",
            "3802fd3681bfeac1ce90e34c98600258993cc832fa89285895aabea22f0d2b81",
        ),
        (
            "fedora-43-x86_64",
            "tinywallet-module-0.20.0-fedora-43-x86_64.tar.gz",
            "9767aa07631a03adaed05430c03726f4e19e16b3561f1fc149264eb52332dbc5",
        ),
        (
            "fedora-44-arm64",
            "tinywallet-module-0.20.0-fedora-44-arm64.tar.gz",
            "3974790d265b775f968e5dcaa7e4206d08da52187cad09e438cc278aba9eb540",
        ),
        (
            "fedora-44-x86_64",
            "tinywallet-module-0.20.0-fedora-44-x86_64.tar.gz",
            "83ff87f0d71985e63ac73c95944e534286fde1a04b0a7daa499753e1fe943269",
        ),
        (
            "macos-15-arm64",
            "tinywallet-module-0.20.0-macos-15-arm64.tar.gz",
            "496e61f6ce4a431ee77bf730a713c3d1c8d76b6666f99d15972f7f62623e632f",
        ),
        (
            "macos-15-x86_64",
            "tinywallet-module-0.20.0-macos-15-x86_64.tar.gz",
            "427b9882eb1ef52be3ba28ed9e292790c60a1f769148b3bbafe4ef5389e1144a",
        ),
        (
            "macos-26-arm64",
            "tinywallet-module-0.20.0-macos-26-arm64.tar.gz",
            "65955f1a0c921da47e1a2cecc7ba4f19cd775c536f7039878dd6c60c3804b017",
        ),
        (
            "macos-26-x86_64",
            "tinywallet-module-0.20.0-macos-26-x86_64.tar.gz",
            "5fa92699fed3cb00c57f29c04fa101d94b24f85aedae91f1b6776d57f7162604",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinywallet-module-0.20.0-ubuntu-22.04-arm64.tar.gz",
            "cce338941314005f54584e567dc5d6c2f9a8a4189c4b47908ead8ceed19ae328",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinywallet-module-0.20.0-ubuntu-22.04-x86_64.tar.gz",
            "8712d00897266d0e2b0889ac76fe4da12d1917ffb8e2b3fcc8eb95c1c6f8570e",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinywallet-module-0.20.0-ubuntu-24.04-arm64.tar.gz",
            "1ca76d32b4d66c68b31963f4aa9cfdc6585276fff660d4ab29726dbb29e04499",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinywallet-module-0.20.0-ubuntu-24.04-x86_64.tar.gz",
            "6c092e41f8053851c5d4919e3c8ff6445cac4a9f8526889aa40a500a77ae40ba",
        ),
        (
            "windows-11-arm64",
            "tinywallet-module-0.20.0-windows-11-arm64.zip",
            "171172d90020470dfbe83d3be480b0d6127b123694fdbf600218acc7cba6f929",
        ),
        (
            "windows-2022-x86_64",
            "tinywallet-module-0.20.0-windows-2022-x86_64.zip",
            "80d5ae3272ae0d32fcc1fcb727573554dffcc34f793d35c9b2603d0a4f8cdb28",
        ),
        (
            "windows-2025-x86_64",
            "tinywallet-module-0.20.0-windows-2025-x86_64.zip",
            "c56914add49d2529dc4620e1c1a4846961d16da9b4d8fb9005c3d6d23989d51f",
        ),
    ];
    actual.sort_unstable();
    published.sort_unstable();
    assert_eq!(actual, published);
}

#[test]
fn tinybox_registry_matches_the_published_v0124_release_manifest() {
    let record = find("tinybox").expect("compiled TinyBox module");
    assert_eq!(record.version, "0.1.24");
    assert_eq!(tinybox_bus::CONTRACT_VERSION, (1, 14));
    assert_eq!(
        record.release_url,
        "https://github.com/tinyhumansai/tinybox/releases/tag/v0.1.24"
    );

    let actual = record
        .assets
        .iter()
        .map(|asset| (asset.host_key, asset.archive, asset.sha256))
        .collect::<Vec<_>>();
    let published = [
        (
            "archlinux-arm64",
            "tinybox-0.1.24-archlinux-arm64.tar.gz",
            "11b4c959868e9e13f60af116946145530ca76ad68ab2ceef4fea11a0c2f76c5f",
        ),
        (
            "archlinux-x86_64",
            "tinybox-0.1.24-archlinux-x86_64.tar.gz",
            "84764f24331dfb5ff1798419e69ada288233506f6d28f4f582f5e6a87f65d9e9",
        ),
        (
            "fedora-43-arm64",
            "tinybox-0.1.24-fedora-43-arm64.tar.gz",
            "26763cb6a77e638adfb8641b58aaa0f57493403c7ab4dbca639f35e434fbd20c",
        ),
        (
            "fedora-43-x86_64",
            "tinybox-0.1.24-fedora-43-x86_64.tar.gz",
            "442d67e92c011125350e672a505925008b0bb26c368d3408c24029e60bc12be9",
        ),
        (
            "fedora-44-arm64",
            "tinybox-0.1.24-fedora-44-arm64.tar.gz",
            "67c0bf40748a1bf9e749bc918cdaddc5ba53f29f9cc3327a7c56fc82ef8dfdee",
        ),
        (
            "fedora-44-x86_64",
            "tinybox-0.1.24-fedora-44-x86_64.tar.gz",
            "5e320dcce68a3b36c61a33647575c9e3f1449388f489e6aa094b7684863ab377",
        ),
        (
            "macos-15-arm64",
            "tinybox-0.1.24-macos-15-arm64.tar.gz",
            "59a76b8ea5ce093d26616b115cd11493459dcefebec73c2b63fa115e4c70d6f7",
        ),
        (
            "macos-15-x86_64",
            "tinybox-0.1.24-macos-15-x86_64.tar.gz",
            "dd73c660e3c54004dad55257eeefc812fa8c2b3bf89d82e346b5bb4e95439b7f",
        ),
        (
            "macos-26-arm64",
            "tinybox-0.1.24-macos-26-arm64.tar.gz",
            "3c2091b15246cb6fb566c73cbfc2f1a334b9f1d1a4ea58839f3576536321b285",
        ),
        (
            "macos-26-x86_64",
            "tinybox-0.1.24-macos-26-x86_64.tar.gz",
            "a077fae96405163fe4cc2ae0dcdfb2dee9e2b8da8ea7f7122d180d0c2f3789eb",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinybox-0.1.24-ubuntu-22.04-arm64.tar.gz",
            "beb26115d378dce167ffe6970f57dfd268c9851f10e254306812be3f478f4567",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinybox-0.1.24-ubuntu-22.04-x86_64.tar.gz",
            "be69717def1d836a8e64e3ed544fdb4fb772ef641df68b473e9677f030f94809",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinybox-0.1.24-ubuntu-24.04-arm64.tar.gz",
            "4935e8099f1a373b7d3fffc6f5a4019d3e40a4c5c3d360e30c1d311ab6eba777",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinybox-0.1.24-ubuntu-24.04-x86_64.tar.gz",
            "33391c0e42a5dc48328bb3cb2f785c44f4fc056ec5fc125a655c1416669b233e",
        ),
        (
            "windows-11-arm64",
            "tinybox-0.1.24-windows-11-arm64.zip",
            "9b00a82bacbba00f528aaf6655bc005fa96751f774cb41cf960cb5c1ac0face9",
        ),
        (
            "windows-2022-x86_64",
            "tinybox-0.1.24-windows-2022-x86_64.zip",
            "a5b676f18bc1252ee81d614c35fd2b1ffb2b2ecc3a0335face170d2185432255",
        ),
        (
            "windows-2025-x86_64",
            "tinybox-0.1.24-windows-2025-x86_64.zip",
            "c09e9d417442be2748cea8f6a1fe17b323675400f3e8b82dc647ec5a84eb3b99",
        ),
    ];

    assert_eq!(actual.len(), 17);
    assert_eq!(actual, published);
}
#[test]
fn tinymcp_registry_matches_the_published_v0150_release_manifest() {
    let record = find("tinymcp").expect("compiled TinyMCP module");
    assert_eq!(record.version, "0.15.0");
    assert_eq!(tinymcp_bus::CONTRACT_VERSION, (1, 22));
    assert_eq!(
        record.release_url,
        "https://github.com/tinyhumansai/tinymcp/releases/tag/v0.15.0"
    );
    let actual = record
        .assets
        .iter()
        .map(|asset| (asset.host_key, asset.archive, asset.sha256))
        .collect::<Vec<_>>();
    let published = [
        (
            "archlinux-rolling-arm64",
            "tinymcp-0.15.0-archlinux-arm64.tar.gz",
            "c77d02b8c03278ad24294dc63e0cd1ae490d4d17ee7b8543e71c2600176a7b60",
        ),
        (
            "archlinux-rolling-x86_64",
            "tinymcp-0.15.0-archlinux-x86_64.tar.gz",
            "836259b5cb2a06da6df598593fe2b60670d36faf3151e875946f16ccb5456fff",
        ),
        (
            "fedora-43-arm64",
            "tinymcp-0.15.0-fedora-43-arm64.tar.gz",
            "0ba117c96137e13f958de0a557094a06c58797c7c03eee1b8fedfc25c7f17b6a",
        ),
        (
            "fedora-43-x86_64",
            "tinymcp-0.15.0-fedora-43-x86_64.tar.gz",
            "da5154722f65019f7de2452662f1f46c8425bfba95d0aa6613dab1efff6fec7e",
        ),
        (
            "fedora-44-arm64",
            "tinymcp-0.15.0-fedora-44-arm64.tar.gz",
            "ada62a47ad9d88a4bd9f8e0defe67718a75db2f30e8e298e838b7d5d3f036f4f",
        ),
        (
            "fedora-44-x86_64",
            "tinymcp-0.15.0-fedora-44-x86_64.tar.gz",
            "2036d7a13f06bfb243fd7f54626cdcf7ed4653a534ac780503ee9980be1488ca",
        ),
        (
            "macos-15-arm64",
            "tinymcp-0.15.0-macos-15-arm64.tar.gz",
            "11aaa66c1dc42ec2b6f6a70c01d1c98e95bef22c7f626d32c6da9455b2cdf38f",
        ),
        (
            "macos-15-x86_64",
            "tinymcp-0.15.0-macos-15-x86_64.tar.gz",
            "11ee1ecede7599f575d75fcc3e030ea0f18df9ffe91a8264dd20da4453825569",
        ),
        (
            "macos-26-arm64",
            "tinymcp-0.15.0-macos-26-arm64.tar.gz",
            "4062a4a7de2bda4d707960c715412ce61d66e8b5983f05b75ef5bca5931eb866",
        ),
        (
            "macos-26-x86_64",
            "tinymcp-0.15.0-macos-26-x86_64.tar.gz",
            "8a144d5f19bb743186373ec18c118da9f1b772d592a14caed892f808a690c41f",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinymcp-0.15.0-ubuntu-22.04-arm64.tar.gz",
            "1d3de88aeaf2255d44f5895ae06d7c7b4e2869314fff0efe8661a1eea50d0863",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinymcp-0.15.0-ubuntu-22.04-x86_64.tar.gz",
            "aacd2ea47433e8c61941be71b5ff95036afe4435da4c38f2ab1dfa0d31ddade1",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinymcp-0.15.0-ubuntu-24.04-arm64.tar.gz",
            "c5dcc8c5f74d00f08d8a4dace062ebaaf347edcee7342d11ff0ba9da840e5807",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinymcp-0.15.0-ubuntu-24.04-x86_64.tar.gz",
            "fefa976a7e8ea19c3c63ce9dedb6652411258c478b0c16abceaa460c3afb95e7",
        ),
        (
            "windows-11-arm64",
            "tinymcp-0.15.0-windows-11-arm64.zip",
            "055fc3a47d050ab5fa76c993ff85d8690dbcaaab31b4673ba40e88575951d9a8",
        ),
        (
            "windows-2022-x86_64",
            "tinymcp-0.15.0-windows-2022-x86_64.zip",
            "e9f06cc33a3e29f1de05935c26238c2f23c46936544ba55a74ac89e3c8c8f690",
        ),
        (
            "windows-2025-x86_64",
            "tinymcp-0.15.0-windows-2025-x86_64.zip",
            "943dea2ae0695c10a9f4376b5025c560f05668f5bd121e06e355bebfc819f970",
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
    assert_eq!(record.version, "0.1.29");
    assert_eq!(tinychannels_bus::CONTRACT_VERSION, 20);
    assert_eq!(
        tinychannels_bus::module_relay::MODULE_OPERATION_VERSION,
        (1, 20)
    );
    let expected = [
        (
            "macos-15-arm64",
            "tinychannels-module-0.1.29-macos-15-arm64.tar.gz",
            "a1dbca6cc5bb04baec1f5b82d43593d87eb135619f95be3d64441a96cc95127e",
        ),
        (
            "macos-15-x86_64",
            "tinychannels-module-0.1.29-macos-15-x86_64.tar.gz",
            "464ceaca6acb3f6653c5e77c6e7d6d357a0cfbcc5e9691f55dc7eb4e1368a7a6",
        ),
        (
            "macos-26-arm64",
            "tinychannels-module-0.1.29-macos-26-arm64.tar.gz",
            "2943b3e8d6b2006a93ad1f880cfc7cd4fbcfb48540c0913bb6318470249edf9c",
        ),
        (
            "macos-26-x86_64",
            "tinychannels-module-0.1.29-macos-26-x86_64.tar.gz",
            "50e92f04b2ef30db6f5053742819a2fbfdfb6d6361f63124fac9f32079775970",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinychannels-module-0.1.29-ubuntu-22.04-arm64.tar.gz",
            "ae621d0ff6cc908135073a2d990ac7d3bc9fd5cc9da99ddcd52e2c610359f7b0",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinychannels-module-0.1.29-ubuntu-22.04-x86_64.tar.gz",
            "e9b5d5d77f551fc17c715bcc23ce9d18d337570ccc5500438bdeff3d5d507dd6",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinychannels-module-0.1.29-ubuntu-24.04-arm64.tar.gz",
            "699dfb6ae0ab31deb1334fc287b58fc75e31606ce8f28c07a9dcec1c089c6d9c",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinychannels-module-0.1.29-ubuntu-24.04-x86_64.tar.gz",
            "3b90fb51324a9068ad7ab5deb8162584d1bebac75ef4abfa05984b6d73fe2db0",
        ),
        (
            "windows-11-arm64",
            "tinychannels-module-0.1.29-windows-11-arm64.zip",
            "efd1b177c73832510434e97f3c3e7e4ee40f36e0d24f6938a5b3e0dfc9297f0c",
        ),
        (
            "windows-2022-x86_64",
            "tinychannels-module-0.1.29-windows-2022-x86_64.zip",
            "c8d8dac1e6529d552920ae2d77520971a9cd1241e164b0b4c588384ebaf297da",
        ),
        (
            "windows-2025-x86_64",
            "tinychannels-module-0.1.29-windows-2025-x86_64.zip",
            "c347c824605aae4411b3b5e1385c689043da288445ce75fc58839f7d354aafb6",
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
    assert_eq!(record.version, "0.2.0");
    assert_eq!(tinyvoice_bus::CONTRACT_VERSION, (1, 9));
    let expected = [
        (
            "macos-15-arm64",
            "tinyvoice-module-0.2.0-macos-15-arm64.tar.gz",
            "340e799db919f145382b372c45d237ef1edbf69afa2fa0ab976eccd850b62f03",
        ),
        (
            "macos-15-x86_64",
            "tinyvoice-module-0.2.0-macos-15-x86_64.tar.gz",
            "f5bd0ab69258b5fda58074768c959df41f49ed1fda34a8c2d32027b7feb0348e",
        ),
        (
            "macos-26-arm64",
            "tinyvoice-module-0.2.0-macos-26-arm64.tar.gz",
            "7a7babe46af562e9833d59a594f66babe76bdc0a7b925754d04bde8579bb1299",
        ),
        (
            "macos-26-x86_64",
            "tinyvoice-module-0.2.0-macos-26-x86_64.tar.gz",
            "a29df2511d63eec3c1cb97bdee2a13a7da94f3c53f5f36cb217c271363e6c2fa",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinyvoice-module-0.2.0-ubuntu-22.04-arm64.tar.gz",
            "e59e98a6844e6e202da9df539d63dedc0cb2c7c733b8cd91ff6fc6b9a2baf9a0",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinyvoice-module-0.2.0-ubuntu-22.04-x86_64.tar.gz",
            "57330db85525148a78352882181ab7008fd79c18528289ed43f11cdae4f01c31",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinyvoice-module-0.2.0-ubuntu-24.04-arm64.tar.gz",
            "d9cfea6d2a910e98f6609dbca65d3bbdcc3950f40615545a0ce14d9117ce34cd",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinyvoice-module-0.2.0-ubuntu-24.04-x86_64.tar.gz",
            "7993badf940bfbc988c078f9c1ba40af29a6e4177ead8680b78e7a98545cc5e4",
        ),
        (
            "windows-11-arm64",
            "tinyvoice-module-0.2.0-windows-11-arm64.zip",
            "d7be3ffd3e6bd2d3deb7f334d447250cb31228f49dfec86e3a24c4d3e4d93754",
        ),
        (
            "windows-2022-x86_64",
            "tinyvoice-module-0.2.0-windows-2022-x86_64.zip",
            "15190a7cd8a3d49d4b4c9b4e3e39247fa936ecea9a7d5f7becd2911e97a8636b",
        ),
        (
            "windows-2025-x86_64",
            "tinyvoice-module-0.2.0-windows-2025-x86_64.zip",
            "57c802f2e1c07c66aae10de26013f3ddb6f02d5309fb6ea5f0580d048501bd86",
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
        "e038b2529e40d834f4754f3566aaafc5cb5c4a2f0b643a389b4b66ba7f91db74"
    );
}
