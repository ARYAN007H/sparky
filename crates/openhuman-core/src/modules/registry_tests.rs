use super::{find, ALL};
use tinybus::module::platform::candidates_for;

#[test]
fn tinycomputer_registry_matches_bus_contract_and_published_release() {
    let desktop = find("tinycomputer").expect("compiled computer module");
    assert_eq!(desktop.bus_name, tinycomputer_bus::names::INTERFACE);
    assert_eq!(desktop.object_path, tinycomputer_bus::names::OBJECT_PATH);
    assert_eq!(desktop.version, "0.10.1");
    assert_eq!(desktop.assets.len(), 7);
    assert_eq!(
        desktop.asset_for("macos-26-arm64").unwrap().sha256,
        "39d1ca3db737c26b4d39d9a22c16da7845735c6349dd012c59533499498af874"
    );
}

#[test]
fn tinybox_registry_matches_the_published_v0118_release_manifest() {
    let record = find("tinybox").expect("compiled TinyBox module");
    assert_eq!(record.version, "0.1.18");
    assert_eq!(
        record.release_url,
        "https://github.com/tinyhumansai/tinybox/releases/tag/v0.1.18"
    );

    let actual = record
        .assets
        .iter()
        .map(|asset| (asset.host_key, asset.archive, asset.sha256))
        .collect::<Vec<_>>();
    let published = [
        (
            "ubuntu-24.04-x86_64",
            "tinybox-0.1.18-ubuntu-24.04-x86_64.tar.gz",
            "232934349cb0ece8f049c00e5d15e38e30248a100cd9b66b203e69aee23145da",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinybox-0.1.18-ubuntu-24.04-arm64.tar.gz",
            "994a37f95e128cd1b54999a9d4eebf012eaeb17b6a3006c9985a83b10e4ee6e1",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinybox-0.1.18-ubuntu-22.04-x86_64.tar.gz",
            "f8210816dfacd7ee58dcfb4482845f40ee4dbb435f706c2e154e9542a15f8329",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinybox-0.1.18-ubuntu-22.04-arm64.tar.gz",
            "b192c1ce4333b8668ec9f38a0197aeb443c5a9afd63d5f9b617a6de97f080f4b",
        ),
        (
            "macos-26-arm64",
            "tinybox-0.1.18-macos-26-arm64.tar.gz",
            "6d7c9120f6f773861ed0901360142ad0c131fe9597d937a21788891a6cae9663",
        ),
        (
            "macos-26-x86_64",
            "tinybox-0.1.18-macos-26-x86_64.tar.gz",
            "7e61a78a8ea82aec31692d84979169ba72202ac7698ecac7cf515445f911473d",
        ),
        (
            "macos-15-arm64",
            "tinybox-0.1.18-macos-15-arm64.tar.gz",
            "b827b00712d8ea5eaf081be9bb8d9e5283dd29fbf34c1dc8d26d15a64224483b",
        ),
        (
            "macos-15-x86_64",
            "tinybox-0.1.18-macos-15-x86_64.tar.gz",
            "c4526cc2f05137d1d82d8a41681403c3c74d0c585a7ff51c4dd49ff3f2eb9694",
        ),
        (
            "windows-2025-x86_64",
            "tinybox-0.1.18-windows-2025-x86_64.zip",
            "c320392f979d3298c4bcecb4b54f18db40d49bb546634a441219eef9f5fbf592",
        ),
        (
            "windows-2022-x86_64",
            "tinybox-0.1.18-windows-2022-x86_64.zip",
            "d06c300eb7651147e9c7672e1377f25cc88e824be6afadfdafd4ec7eb6f5f8d1",
        ),
        (
            "windows-11-arm64",
            "tinybox-0.1.18-windows-11-arm64.zip",
            "b3a26410b6a7adda83ae9ce23e0716367e06c3561e8a2f9ffbe30b6820ccae89",
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

fn supported_host_keys(record: &super::ModuleRecord) -> Vec<String> {
    every_host_key()
        .into_iter()
        .filter(|key| record.id != "tinycomputer" || !key.starts_with("ubuntu-"))
        .collect()
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
