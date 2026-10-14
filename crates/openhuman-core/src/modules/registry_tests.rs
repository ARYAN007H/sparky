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
fn tinywallet_registry_matches_the_verified_v023_release_manifest() {
    let wallet = find("tinywallet").expect("compiled TinyWallet module");
    assert_eq!(wallet.version, "0.23.0");
    #[cfg(feature = "web3")]
    assert_eq!(tinywallet_bus::CONTRACT_VERSION, (1, 14));
    assert_eq!(
        wallet.release_url,
        "https://github.com/tinyhumansai/tinywallet/releases/tag/v0.23.0"
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
            "tinywallet-module-0.23.1-archlinux-arm64.tar.gz",
            "63f561a54ac72dd5d05b5bb4ed88d0dd75d57957d4ba77529f72611fd73eb958",
        ),
        (
            "archlinux-x86_64",
            "tinywallet-module-0.23.1-archlinux-x86_64.tar.gz",
            "bf47d50a2d9c9d33a65048af7fddfeb5863b130e4581ecb2b062454d58e4c873",
        ),
        (
            "fedora-43-arm64",
            "tinywallet-module-0.23.1-fedora-43-arm64.tar.gz",
            "c0f656035bf491da0d2f628409cba85a35d37922056b62d0dc699fd32549f3b2",
        ),
        (
            "fedora-43-x86_64",
            "tinywallet-module-0.23.1-fedora-43-x86_64.tar.gz",
            "b6c89126b043a0dc6dca26e1a901d0359933a84a6076be6e4624a76fd761f832",
        ),
        (
            "fedora-44-arm64",
            "tinywallet-module-0.23.1-fedora-44-arm64.tar.gz",
            "6b874de4e364c0899f8687a7188286b2fc4982a843f7bd87abf22feb9746cd9f",
        ),
        (
            "fedora-44-x86_64",
            "tinywallet-module-0.23.1-fedora-44-x86_64.tar.gz",
            "6b1203f11f605fe3f4ef06b6247b8a6b93e6f2ea66ec931a74f4897b55318505",
        ),
        (
            "macos-15-arm64",
            "tinywallet-module-0.23.1-macos-15-arm64.tar.gz",
            "fb0b18a0050be7915708b5d19b128d0068194e59b215ce3dffafef0d7224de8b",
        ),
        (
            "macos-15-x86_64",
            "tinywallet-module-0.23.1-macos-15-x86_64.tar.gz",
            "f0939b246dd958b0a679c3915cfae8fa7069e09e1a83ceb06b74af6ccea4f1b8",
        ),
        (
            "macos-26-arm64",
            "tinywallet-module-0.23.1-macos-26-arm64.tar.gz",
            "7d3038ef40cdb160fbfd7bdbd0825594ada1f7eb098546c3d48d3ec2594f37d7",
        ),
        (
            "macos-26-x86_64",
            "tinywallet-module-0.23.1-macos-26-x86_64.tar.gz",
            "f780cb7169e8116c595d5bdaae52d6a650d7ae9dc11be2df0a3f48c74640d30f",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinywallet-module-0.23.1-ubuntu-22.04-arm64.tar.gz",
            "2aebd102b08c16d4a0b2b50f26afc1024522d7a3d3baf5944ad52d388e2c298c",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinywallet-module-0.23.1-ubuntu-22.04-x86_64.tar.gz",
            "ff16c343dac63ad54b71a82c378533059799257e2448ee1938fd4610f7949ffd",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinywallet-module-0.23.1-ubuntu-24.04-arm64.tar.gz",
            "56b715ff30e132a55ab15b0215780cecaeb4190aca7e41a2db270e0ec6bfc2b4",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinywallet-module-0.23.1-ubuntu-24.04-x86_64.tar.gz",
            "3ebe2672fe5c275c58ee00425719da879940a51f4ad9baa7dde3d94276bdad7b",
        ),
        (
            "windows-11-arm64",
            "tinywallet-module-0.23.1-windows-11-arm64.zip",
            "cbd1e60e0e64844f45b2d3d13fbc7b55e4634b309b06db6f992671af6555a105",
        ),
        (
            "windows-2022-x86_64",
            "tinywallet-module-0.23.1-windows-2022-x86_64.zip",
            "8b2c6128f25a630793b7ac8c57c0821aa06f434f3bf3350137f9fa61d7d03508",
        ),
        (
            "windows-2025-x86_64",
            "tinywallet-module-0.23.1-windows-2025-x86_64.zip",
            "7e83f7d0fcbca31bb5efd529adaa7cae3031f94aba192918afe4f690abc293f1",
        ),
    ];
    actual.sort_unstable();
    published.sort_unstable();
    assert_eq!(actual, published);
}

#[test]
fn tinybox_registry_matches_the_published_v0130_release_manifest() {
    let record = find("tinybox").expect("compiled TinyBox module");
    assert_eq!(record.version, "0.1.30");
    assert_eq!(tinybox_bus::CONTRACT_VERSION, (1, 18));
    assert_eq!(
        record.release_url,
        "https://github.com/tinyhumansai/tinybox/releases/tag/v0.1.30"
    );

    let actual = record
        .assets
        .iter()
        .map(|asset| (asset.host_key, asset.archive, asset.sha256))
        .collect::<Vec<_>>();
    let published = [
        (
            "archlinux-arm64",
            "tinybox-0.1.31-archlinux-arm64.tar.gz",
            "f261dc7992df85dd2fd42e671c40ca90e87da8a7c9c7a246cb65ebec92070ae0",
        ),
        (
            "archlinux-x86_64",
            "tinybox-0.1.31-archlinux-x86_64.tar.gz",
            "aa0e7a91b7772c80c2a0be3cd99269d93b80a922d46be184eee011754b8b2af7",
        ),
        (
            "fedora-43-arm64",
            "tinybox-0.1.31-fedora-43-arm64.tar.gz",
            "62af32f2ab2b90133df13ec7037cd0f5b5c160ee94a9828ccd1f20c20455bbfc",
        ),
        (
            "fedora-43-x86_64",
            "tinybox-0.1.31-fedora-43-x86_64.tar.gz",
            "f5144f0926ca50a27510029ca24afaeb22c95ce3732a45113cef5c6f86d5e14b",
        ),
        (
            "fedora-44-arm64",
            "tinybox-0.1.31-fedora-44-arm64.tar.gz",
            "893368d05de8847c96d087a7b186a247485483a60a377b8dca39a7eaec48c8f4",
        ),
        (
            "fedora-44-x86_64",
            "tinybox-0.1.31-fedora-44-x86_64.tar.gz",
            "ffbbd8deb020ff210478bb267b64669429da21667c83baecc83025094d71165a",
        ),
        (
            "macos-15-arm64",
            "tinybox-0.1.31-macos-15-arm64.tar.gz",
            "9c7c04d8be65a88fee4cfe1ed763c2076dea3aa60794539d55167fa72968cce7",
        ),
        (
            "macos-15-x86_64",
            "tinybox-0.1.31-macos-15-x86_64.tar.gz",
            "f976d44ea0e08ce34e845f593897c558b69e787fae1a36fa4fbf20e250bf5d35",
        ),
        (
            "macos-26-arm64",
            "tinybox-0.1.31-macos-26-arm64.tar.gz",
            "abd59c035f8f452ee5466e46a731e6d39adaf478c08d562e8fc3ffdf5d88b8a6",
        ),
        (
            "macos-26-x86_64",
            "tinybox-0.1.31-macos-26-x86_64.tar.gz",
            "73dd91aee335ff0437adb831f2e22d1d891587518a1e9fd5f6ff094b562669cc",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinybox-0.1.31-ubuntu-22.04-arm64.tar.gz",
            "a892040113bde90a1aa8d2c3cf0721be27b58ff3ae3c90341024af2b91137839",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinybox-0.1.31-ubuntu-22.04-x86_64.tar.gz",
            "f73d8d5a31b3db2aff802594b6219af69ac9b04638cd23154e32fd32247c578c",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinybox-0.1.31-ubuntu-24.04-arm64.tar.gz",
            "dd140c0da699ee91f65593424a98a071f5af9f7f4bcc6e29bd6a825493e5071a",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinybox-0.1.31-ubuntu-24.04-x86_64.tar.gz",
            "addaf5c10364d05d4afa5e7b7780de329638c4e08f07027d8cd7bce313ea89f2",
        ),
        (
            "windows-11-arm64",
            "tinybox-0.1.31-windows-11-arm64.zip",
            "994c21e2f77c14323bdf186a4100bbd7077e1fa6efb5c20501413de54fae9ac3",
        ),
        (
            "windows-2022-x86_64",
            "tinybox-0.1.31-windows-2022-x86_64.zip",
            "e5c0599d7ac90b254c946a8d544a8c18d1ea3b7e2b8eadc5cce4fbc2f8e3b41a",
        ),
        (
            "windows-2025-x86_64",
            "tinybox-0.1.31-windows-2025-x86_64.zip",
            "136291f2ae3df0f509cea6573e926f660cbb55bf3a5e0305cbda22677b9fa579",
        ),
    ];

    assert_eq!(actual.len(), 17);
    assert_eq!(actual, published);
}
#[test]
fn tinymcp_registry_matches_the_published_v0181_release_manifest() {
    let record = find("tinymcp").expect("compiled TinyMCP module");
    assert_eq!(record.version, "0.18.1");
    assert_eq!(tinymcp_bus::CONTRACT_VERSION, (1, 24));
    assert_eq!(
        record.release_url,
        "https://github.com/tinyhumansai/tinymcp/releases/tag/v0.18.1"
    );
    let actual = record
        .assets
        .iter()
        .map(|asset| (asset.host_key, asset.archive, asset.sha256))
        .collect::<Vec<_>>();
    let published = [
        (
            "archlinux-rolling-arm64",
            "tinymcp-0.18.2-archlinux-arm64.tar.gz",
            "81d3e59954938768f8384dcf30b0b8aa79f008f0943cce60cd4ba77f0e1ec2dc",
        ),
        (
            "archlinux-rolling-x86_64",
            "tinymcp-0.18.2-archlinux-x86_64.tar.gz",
            "54c44adc674ece88f747c6d977df661ad7e068a8f0e5769533995c1f2423d87c",
        ),
        (
            "fedora-43-arm64",
            "tinymcp-0.18.2-fedora-43-arm64.tar.gz",
            "4b4e66d9cd14df0d264c35cf85c445f4df1f6ca9a33651815c1d90711b21d032",
        ),
        (
            "fedora-43-x86_64",
            "tinymcp-0.18.2-fedora-43-x86_64.tar.gz",
            "dd1f8fb674e8a07d795bbcabf7c69a479e9bd1fdcdba6ac2af930d02af3570e6",
        ),
        (
            "fedora-44-arm64",
            "tinymcp-0.18.2-fedora-44-arm64.tar.gz",
            "1b3733741afed877f9c7757ed6c48f75235ac7017e99f717c3334954221d05ae",
        ),
        (
            "fedora-44-x86_64",
            "tinymcp-0.18.2-fedora-44-x86_64.tar.gz",
            "69328ff8e8b553cf095a482c41a94418caf0bcb44f36921acf583b30cf8eb713",
        ),
        (
            "macos-15-arm64",
            "tinymcp-0.18.2-macos-15-arm64.tar.gz",
            "da8d98429d356914e2e953c73c99613bb0f333af99321c7f2574e2950423acf7",
        ),
        (
            "macos-15-x86_64",
            "tinymcp-0.18.2-macos-15-x86_64.tar.gz",
            "73adee09c2fd747a180a43f9c6700ea03e3f97544994e2564fc9775e2dfe3ce9",
        ),
        (
            "macos-26-arm64",
            "tinymcp-0.18.2-macos-26-arm64.tar.gz",
            "f334675b6b3565faeb6e17f0d2e8caafbf87a72e72aef1fad0d8c8ba40c49f0b",
        ),
        (
            "macos-26-x86_64",
            "tinymcp-0.18.2-macos-26-x86_64.tar.gz",
            "00d094d768553e9dfdb73ae298bab1c5103f3ae806ad5fdd36eda4bfdafc4751",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinymcp-0.18.2-ubuntu-22.04-arm64.tar.gz",
            "bdbcd45b415fe1c30d1c7fe583d7a71a2296ef51290b9e545dc02812a3f0229b",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinymcp-0.18.2-ubuntu-22.04-x86_64.tar.gz",
            "4313c456e1e947ac090fec304b13229baba680175ab7312e38b230e5c42393d3",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinymcp-0.18.2-ubuntu-24.04-arm64.tar.gz",
            "41cc557273334b6da977b0b0a29f37364a2455a5444cb0fff8c63ab976a846f4",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinymcp-0.18.2-ubuntu-24.04-x86_64.tar.gz",
            "e74f33f32a5dfb3209a414236ea368bb746e648d7b24fa0dbd15a3a3df908147",
        ),
        (
            "windows-11-arm64",
            "tinymcp-0.18.2-windows-11-arm64.zip",
            "e1c53505f1137f1c0a536f9d16e280d834ec96706ef34d64095931419b6260f6",
        ),
        (
            "windows-2022-x86_64",
            "tinymcp-0.18.2-windows-2022-x86_64.zip",
            "c9e43713feab3fa66a29328170b3b25cbabe2ae2f1ef0431291c43805ba950c8",
        ),
        (
            "windows-2025-x86_64",
            "tinymcp-0.18.2-windows-2025-x86_64.zip",
            "790be0a331f000d34b93515efe67954ec33fe5d869f529c970649aea5db1d8b7",
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
    assert_eq!(record.version, "0.1.31");
    assert_eq!(tinychannels_bus::CONTRACT_VERSION, 21);
    assert_eq!(
        tinychannels_bus::module_relay::MODULE_OPERATION_VERSION,
        (1, 21)
    );
    let expected = [
        (
            "macos-15-arm64",
            "tinychannels-module-0.1.32-macos-15-arm64.tar.gz",
            "0322f515b80548cddcf0b731f722fa26ff494d2afd30bfbcb65f6780f06f20c3",
        ),
        (
            "macos-15-x86_64",
            "tinychannels-module-0.1.32-macos-15-x86_64.tar.gz",
            "90525be14322388c88b72d1376cab6fa128e572bddcbdfb4e0dcef21aee19127",
        ),
        (
            "macos-26-arm64",
            "tinychannels-module-0.1.32-macos-26-arm64.tar.gz",
            "b4649d9d8bb28310c91b35d51149848b3990afc674bf8f45625a33befcb1ecd0",
        ),
        (
            "macos-26-x86_64",
            "tinychannels-module-0.1.32-macos-26-x86_64.tar.gz",
            "e5461a8b34bc110e023d5f72c102923dc31ef9726ba90e2dc542fb86c35ea505",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinychannels-module-0.1.32-ubuntu-22.04-arm64.tar.gz",
            "4f6b4d343775f850ffa2d53c7ea318ee71cc399058355eddd57f711158a2c3fc",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinychannels-module-0.1.32-ubuntu-22.04-x86_64.tar.gz",
            "f39ac40d83bd2e6df22d613bd45ceb3e4d53a41e11a79fd405fdf3deae60695a",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinychannels-module-0.1.32-ubuntu-24.04-arm64.tar.gz",
            "3504e2f88904f1834d5bce9a09924b32fc1a06cb863c25a780dfdab87cc86c95",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinychannels-module-0.1.32-ubuntu-24.04-x86_64.tar.gz",
            "a51bfe7a27260df3683f3e99a0a04ca1088dede0324486da95c1a24f4deb07a6",
        ),
        (
            "windows-11-arm64",
            "tinychannels-module-0.1.32-windows-11-arm64.zip",
            "b2e28cbec56b18f510062370a17784779ee285dda057084d9f9ca9b9456e1b38",
        ),
        (
            "windows-2022-x86_64",
            "tinychannels-module-0.1.32-windows-2022-x86_64.zip",
            "313de6b8e1c234e1fa44d08d761503a2ef7c8d6790fbda8a96c7cf2fe013c06d",
        ),
        (
            "windows-2025-x86_64",
            "tinychannels-module-0.1.32-windows-2025-x86_64.zip",
            "fa425b3c0de352c5048aa19c367dacecd05351e09fd5c76ddd5e42138510c103",
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
    assert_eq!(record.version, "0.3.1");
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
