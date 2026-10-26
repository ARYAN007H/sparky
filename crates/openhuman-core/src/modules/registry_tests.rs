use super::{find, ALL};
use tinybus::module::platform::candidates_for;

#[path = "registry_computer_release_tests.rs"]
mod computer_release_tests;

#[path = "registry_release_manifest_tests.rs"]
mod release_manifest_tests;

#[test]
#[cfg(feature = "web3")]
fn tinywallet_registry_matches_the_verified_v0260_release_manifest() {
    let wallet = find("tinywallet").expect("compiled TinyWallet module");
    assert_eq!(wallet.version, "0.26.0");
    #[cfg(feature = "web3")]
    assert_eq!(tinywallet_bus::CONTRACT_VERSION, (1, 14));
    assert_eq!(
        wallet.release_url,
        "https://github.com/tinyhumansai/tinywallet/releases/tag/v0.26.0"
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
            "tinywallet-module-0.26.0-archlinux-arm64.tar.gz",
            "b59ec08e44abf5079a19978b8adc3f6f2514a2bd8925c20429d554fe610a4c7e",
        ),
        (
            "archlinux-x86_64",
            "tinywallet-module-0.26.0-archlinux-x86_64.tar.gz",
            "b15f03b5c6ed054c79cdf6518c1c6674cfced37556f52d189d30ad86814090e5",
        ),
        (
            "fedora-43-arm64",
            "tinywallet-module-0.26.0-fedora-43-arm64.tar.gz",
            "f1be27c80bcc0cf93846ab12b9ba1151d17752875cb6603c23da1ee63d9b8a95",
        ),
        (
            "fedora-43-x86_64",
            "tinywallet-module-0.26.0-fedora-43-x86_64.tar.gz",
            "f22abcef72703f4ad3594f33eff9c16d3aa9850aefe2457635fab9f42b7add9f",
        ),
        (
            "fedora-44-arm64",
            "tinywallet-module-0.26.0-fedora-44-arm64.tar.gz",
            "39a5517ee0be1b3e8614f8691c5deb0b51817752007bd74dd781d877dae8313b",
        ),
        (
            "fedora-44-x86_64",
            "tinywallet-module-0.26.0-fedora-44-x86_64.tar.gz",
            "a48390bf558d1b14147790d880b792cf5f0e1aea7c79d38217f7d7ea53a2bc9d",
        ),
        (
            "macos-15-arm64",
            "tinywallet-module-0.26.0-macos-15-arm64.tar.gz",
            "04bc74a5187ac62b7416d88f847f9aa4b49914b8da864f82a19305fb43ace6f7",
        ),
        (
            "macos-15-x86_64",
            "tinywallet-module-0.26.0-macos-15-x86_64.tar.gz",
            "526119531f5132973c620ebceaf21e237cc39306a4638a31e88239b0816a6122",
        ),
        (
            "macos-26-arm64",
            "tinywallet-module-0.26.0-macos-26-arm64.tar.gz",
            "ad070c81c067578516e76c56edf68256ebb063826c98bfbcce2db16f2ab3ad0c",
        ),
        (
            "macos-26-x86_64",
            "tinywallet-module-0.26.0-macos-26-x86_64.tar.gz",
            "6c784834d21b3b0a94623f84dd3a132efd499788db8ae46844d7e3b42c0b4512",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinywallet-module-0.26.0-ubuntu-22.04-arm64.tar.gz",
            "bd404a0beecf749f6293685d3799abdf1ab701ec8b4fe6625bc7a14d9c890b79",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinywallet-module-0.26.0-ubuntu-22.04-x86_64.tar.gz",
            "87b7c1f9a76db09ab0dcbda1b02caef22e355c5c731bba7b53f8571aaa6a19fb",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinywallet-module-0.26.0-ubuntu-24.04-arm64.tar.gz",
            "9752e813ef204ae59759b8fee54cafe2b38b8b120dfe0ec451e28a7d20b9fdef",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinywallet-module-0.26.0-ubuntu-24.04-x86_64.tar.gz",
            "90bf726fb55fc0a6cd47e2e8a838d2e2ee7bf5217de19450a88479a089b85a17",
        ),
        (
            "windows-11-arm64",
            "tinywallet-module-0.26.0-windows-11-arm64.zip",
            "6eb5c829b6a38ae6732475b19d0bb7c106dc490ba55a0365208976c96eb275a2",
        ),
        (
            "windows-2022-x86_64",
            "tinywallet-module-0.26.0-windows-2022-x86_64.zip",
            "ca46336d7cad9d9728f0a7de58d69d2755bda23eeed617481c543102a1fb99f4",
        ),
        (
            "windows-2025-x86_64",
            "tinywallet-module-0.26.0-windows-2025-x86_64.zip",
            "c66b49e30adafe29033823b823785a68121ca6177b12a9ec1b989f1c68f2bde4",
        ),
    ];
    actual.sort_unstable();
    published.sort_unstable();
    assert_eq!(actual, published);
}

#[test]
fn tinybox_registry_matches_the_published_v022_release_manifest() {
    let record = find("tinybox").expect("compiled TinyBox module");
    assert_eq!(record.version, "0.2.2");
    assert_eq!(tinybox_bus::CONTRACT_VERSION, (2, 1));
    assert_eq!(
        record.release_url,
        "https://github.com/tinyhumansai/tinybox/releases/tag/v0.2.2"
    );

    let actual = record
        .assets
        .iter()
        .map(|asset| (asset.host_key, asset.archive, asset.sha256))
        .collect::<Vec<_>>();
    let published = [
        (
            "archlinux-arm64",
            "tinybox-0.2.2-archlinux-arm64.tar.gz",
            "1ffb8939dd2d4863743af1c141fd57c5b7a40fef23ef76986cc9e78bf10f59db",
        ),
        (
            "archlinux-x86_64",
            "tinybox-0.2.2-archlinux-x86_64.tar.gz",
            "f905754bc510090cf8cf62f407e4a11d39b3dd4c854033d51ceda988b40fb0d0",
        ),
        (
            "fedora-43-arm64",
            "tinybox-0.2.2-fedora-43-arm64.tar.gz",
            "9d4af8477c4c4fbdd9a5f07ce7c0af7dcc680da742807f2251c1ec9e1e52422e",
        ),
        (
            "fedora-43-x86_64",
            "tinybox-0.2.2-fedora-43-x86_64.tar.gz",
            "4d159a01d9fe4d48bdd06443a4c179cc213410a2f659a55432661c1552e381b8",
        ),
        (
            "fedora-44-arm64",
            "tinybox-0.2.2-fedora-44-arm64.tar.gz",
            "a152c2168b279dc5e34b2538134715e14e820f205ac2ad17d9bc6d1fce1cd6ba",
        ),
        (
            "fedora-44-x86_64",
            "tinybox-0.2.2-fedora-44-x86_64.tar.gz",
            "89e074b41946396cbf08587290ae3936c9ba931a7f598ff64f531428ae0f65a9",
        ),
        (
            "macos-15-arm64",
            "tinybox-0.2.2-macos-15-arm64.tar.gz",
            "0db00ded02f00bc90b1b247560aee7bc3899d33f34ee3d01447e7d48e7c6d170",
        ),
        (
            "macos-15-x86_64",
            "tinybox-0.2.2-macos-15-x86_64.tar.gz",
            "af8d4c3ec05f2d87bf1cfb19e69f98e15cb47c50b62391d502dfb236a9074291",
        ),
        (
            "macos-26-arm64",
            "tinybox-0.2.2-macos-26-arm64.tar.gz",
            "281aed7470018142a8f5605a7d495599426178594fcb2fcc06907ff478ef4606",
        ),
        (
            "macos-26-x86_64",
            "tinybox-0.2.2-macos-26-x86_64.tar.gz",
            "8048fa03a2e0b1858ab783dc10271773180f14d4ba915e68f29463abed3a5a75",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinybox-0.2.2-ubuntu-22.04-arm64.tar.gz",
            "7da76007f7c61572035401e6ca96aa57884bcf69c78a5dc0fc1d44cffbea7a79",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinybox-0.2.2-ubuntu-22.04-x86_64.tar.gz",
            "4ea0af27c33b0fa21cafd4b304fb9f5137ac023ed96c9204eb008849e42f6afa",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinybox-0.2.2-ubuntu-24.04-arm64.tar.gz",
            "6ff35191084f21a6d4aca29ad41a21c4f9325ea4a50244def13afbb5fc46ae1d",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinybox-0.2.2-ubuntu-24.04-x86_64.tar.gz",
            "e52934c1bdd541bd0b23b565b930dba17fbd1cc112513f7dfa6ded83921031b6",
        ),
        (
            "windows-11-arm64",
            "tinybox-0.2.2-windows-11-arm64.zip",
            "c3118be2e00d3823b9e79337ea8d3628db51564781c1839547490d0efb36f899",
        ),
        (
            "windows-2022-x86_64",
            "tinybox-0.2.2-windows-2022-x86_64.zip",
            "cf3d75b5386b9e1924dca7bc46aaec33a59ec1baefe60d7eeacc94d4d8b61f3e",
        ),
        (
            "windows-2025-x86_64",
            "tinybox-0.2.2-windows-2025-x86_64.zip",
            "7dc94f4f4f33582fc4b7efce4fadfb1870ea8902b3f09b5525c20233145dddf2",
        ),
    ];

    assert_eq!(actual.len(), 17);
    assert_eq!(actual, published);
}
#[test]
fn tinymcp_registry_matches_the_published_v0203_release_manifest() {
    let record = find("tinymcp").expect("compiled TinyMCP module");
    assert_eq!(record.version, "0.20.3");
    assert_eq!(tinymcp_bus::CONTRACT_VERSION, (1, 28));
    assert_eq!(
        record.release_url,
        "https://github.com/tinyhumansai/tinymcp/releases/tag/v0.20.3"
    );
    let actual = record
        .assets
        .iter()
        .map(|asset| (asset.host_key, asset.archive, asset.sha256))
        .collect::<Vec<_>>();
    let published = [
        (
            "archlinux-rolling-arm64",
            "tinymcp-0.20.3-archlinux-arm64.tar.gz",
            "aee29238dae7f08173daba386dcf7630890051f9793d20444cc01ccb07e995ae",
        ),
        (
            "archlinux-rolling-x86_64",
            "tinymcp-0.20.3-archlinux-x86_64.tar.gz",
            "73cef881d21b0506fb1dc0b4ee9fcc3148dd536a8719e6c7106a02a93d41c8fc",
        ),
        (
            "fedora-43-arm64",
            "tinymcp-0.20.3-fedora-43-arm64.tar.gz",
            "cd57e0c664e4dd5085dfea368a2b3528431e5d5999b04b0b0875c9fa2d3102df",
        ),
        (
            "fedora-43-x86_64",
            "tinymcp-0.20.3-fedora-43-x86_64.tar.gz",
            "78e93ea6d918a8296b9af5489fd6135d5dc2a4600adcff845e6cc7e0442b3022",
        ),
        (
            "fedora-44-arm64",
            "tinymcp-0.20.3-fedora-44-arm64.tar.gz",
            "42ea550016832503bf2e342a8207d3e0d582d7c69b161b1ceaca33a10f2c495c",
        ),
        (
            "fedora-44-x86_64",
            "tinymcp-0.20.3-fedora-44-x86_64.tar.gz",
            "0f9bfb3602b9153bcbc7a1c70fb29aea3d6f7873ab024b6791f7d571b749d7a3",
        ),
        (
            "macos-15-arm64",
            "tinymcp-0.20.3-macos-15-arm64.tar.gz",
            "3efc9a2fbaca40415733d3e22c2521e037d6d3e0d84109c12e9e82317fd2545b",
        ),
        (
            "macos-15-x86_64",
            "tinymcp-0.20.3-macos-15-x86_64.tar.gz",
            "34481bf3379612686fc5c1dc296b441f52f437b3ce7bc74ab74ef31389623230",
        ),
        (
            "macos-26-arm64",
            "tinymcp-0.20.3-macos-26-arm64.tar.gz",
            "cc073af4aa19fbaf7ed7b3c2749affd4bdff3f0c2856970e07da1f30a710b964",
        ),
        (
            "macos-26-x86_64",
            "tinymcp-0.20.3-macos-26-x86_64.tar.gz",
            "2f7cccc732d091b87d884263ee18149b490dd522969e31eb3ec6295c874a31e7",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinymcp-0.20.3-ubuntu-22.04-arm64.tar.gz",
            "960b06810b7164fcc7d32139cfe0a0d9f5e144268bbe59c0d8e7d9f2841810fa",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinymcp-0.20.3-ubuntu-22.04-x86_64.tar.gz",
            "0911cfcfb2647d1a41d8a87e3959e84e4336f54b7939f473e0bf8e6eb2ec5432",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinymcp-0.20.3-ubuntu-24.04-arm64.tar.gz",
            "6e93b86b9b9a550625c46a78a93e10369749e576ccba903a63731aec2b0f94ae",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinymcp-0.20.3-ubuntu-24.04-x86_64.tar.gz",
            "a955333c3c4255db62bd77b8cc54f2a0a510b62a41f1e3f37265aa8da82af390",
        ),
        (
            "windows-11-arm64",
            "tinymcp-0.20.3-windows-11-arm64.zip",
            "33e96f7cb8feb23025e80f8af19d55dc53f9443ffa119370266fc44389e6f660",
        ),
        (
            "windows-2022-x86_64",
            "tinymcp-0.20.3-windows-2022-x86_64.zip",
            "df48dd51c1f9e4648df2b2e199bdb6cc5defcf9733b9767505f43f2aab23753d",
        ),
        (
            "windows-2025-x86_64",
            "tinymcp-0.20.3-windows-2025-x86_64.zip",
            "53004de45083fd1b4b8b578ce1b24388c8ac05b2b4f4a761c7885b4d5edbcb88",
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
    assert_eq!(search.version, "0.5.3");
    assert_eq!(tinysearch_bus::CONTRACT_VERSION, (2, 2));
    assert_eq!(
        search.asset_for("ubuntu-24.04-x86_64").unwrap().sha256,
        "78b5632d9c207f9b78ee83627bf35ae73172d0fb1fb7ea47273058f4c1f4c0b1"
    );
}
