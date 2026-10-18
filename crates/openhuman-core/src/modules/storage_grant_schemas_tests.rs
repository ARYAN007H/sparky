use super::*;
use crate::modules::storage_grants::{authorize, declared, Denied, Object, Plan};
use serde_json::json;

fn allowed(
    grant: &StorageGrant,
    object: Object,
    selector: &str,
    request: serde_json::Value,
) -> bool {
    matches!(
        authorize(grant, object, selector, &request),
        Ok(Plan::Forward { .. })
    )
}

#[test]
fn every_row_has_a_valid_generated_grant() {
    for row in &ROWS {
        let grant =
            declared(row.module).unwrap_or_else(|| panic!("{} declares storage", row.module));
        grant
            .validate()
            .unwrap_or_else(|why| panic!("{}: {why}", row.module));
        assert!(grant.writable);
        let items = module_items(row.owner);
        // tinyconnectors also carries its one v1 collection (see `grant_for`).
        let v1_extra = usize::from(row.module == tinyconnectors::MODULE_ID);
        assert_eq!(
            grant.collections.len(),
            items.collections.len() + v1_extra,
            "{}",
            row.module
        );
    }
    assert!(declared("not-a-module").is_none());
}

#[test]
fn the_registry_ids_exist() {
    for row in &ROWS {
        assert!(
            crate::modules::registry::ALL
                .iter()
                .any(|record| record.id == row.module)
                || row.module == "tinywallet",
            "{}",
            row.module
        );
    }
}

#[test]
fn tinymcp_reaches_its_v2_collections_stream_and_secrets() {
    let grant = declared("tinymcp").unwrap();
    assert!(allowed(
        &grant,
        Object::Documents,
        "Get",
        json!({"collection": "mcp.servers", "id": "x"})
    ));
    assert!(allowed(
        &grant,
        Object::Streams,
        "Append",
        json!({"stream": "mcp.audit"})
    ));
    assert!(allowed(
        &grant,
        Object::Secrets,
        "Get",
        json!({"name": "mcp_client_env/abc"})
    ));
    assert_eq!(
        authorize(
            &grant,
            Object::Documents,
            "Get",
            &json!({"collection": "mcp_servers", "id": "x"})
        )
        .unwrap_err(),
        Denied::Target,
        "the v1 name is no longer granted"
    );
}

#[test]
fn tinyjuice_reaches_only_its_blob_prefix() {
    let grant = declared("tinyjuice").unwrap();
    assert!(grant.collections.is_empty());
    assert!(allowed(
        &grant,
        Object::Blobs,
        "Get",
        json!({"key": "juice/ccr/abc"})
    ));
    assert!(!allowed(
        &grant,
        Object::Blobs,
        "Get",
        json!({"key": "other/abc"})
    ));
}

#[test]
fn tinyconnectors_grant_covers_the_names_the_pinned_release_writes() {
    let grant = declared("tinyconnectors").unwrap();
    // v1 names, written by the released module the registry pins.
    assert!(allowed(
        &grant,
        Object::Documents,
        "Get",
        json!({"collection": "composio-user-scopes", "id": "gmail"})
    ));
    assert!(allowed(
        &grant,
        Object::Streams,
        "Append",
        json!({"stream": "triggers/2026-10-12", "entries": []})
    ));
    // v2 names, from the gitlink schema.
    assert!(allowed(
        &grant,
        Object::Documents,
        "Get",
        json!({"collection": "connectors.user_scopes", "id": "gmail"})
    ));
}
