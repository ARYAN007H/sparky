use super::*;
use crate::modules::storage_grants::{Denied, Object, Plan, authorize, declared};
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
    for row in ROWS {
        let grant =
            declared(row.module).unwrap_or_else(|| panic!("{} declares storage", row.module));
        grant
            .validate()
            .unwrap_or_else(|why| panic!("{}: {why}", row.module));
        assert!(grant.writable);
        let items = module_items(row.owner);
        assert_eq!(
            grant.collections.len(),
            items.collections.len(),
            "{}",
            row.module
        );
    }
    assert!(declared("not-a-module").is_none());
}

#[test]
fn the_registry_ids_exist() {
    for row in ROWS {
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
    assert_eq!(
        grant
            .collections
            .iter()
            .map(|collection| collection.name.as_str())
            .collect::<Vec<_>>(),
        ["juice.savings"]
    );
    assert!(allowed(
        &grant,
        Object::Documents,
        "Get",
        json!({"collection": "juice.savings", "id": "x"})
    ));
    assert!(!allowed(
        &grant,
        Object::Documents,
        "Get",
        json!({"collection": "unrelated.collection", "id": "x"})
    ));
    assert!(allowed(
        &grant,
        Object::Blobs,
        "Get",
        json!({"key": "tinyjuice_ccr/abc"})
    ));
    assert!(!allowed(
        &grant,
        Object::Blobs,
        "Get",
        json!({"key": "juice/ccr/abc"})
    ));
}

#[test]
fn non_document_grants_match_the_owner_contracts_without_broad_prefixes() {
    let connectors = declared("tinyconnectors").unwrap();
    assert!(allowed(
        &connectors,
        Object::Streams,
        "Append",
        json!({"stream": "connectors.triggers/2026-10-21"})
    ));
    assert!(allowed(
        &connectors,
        Object::Streams,
        "Append",
        json!({"stream": "triggers/2026-10-21"})
    ));
    assert_eq!(
        authorize(
            &connectors,
            Object::Streams,
            "Append",
            &json!({"stream": "other/2026-10-21"})
        ),
        Err(Denied::Target)
    );

    let wallet = declared("tinywallet").unwrap();
    assert!(allowed(
        &wallet,
        Object::Streams,
        "Append",
        json!({"stream": "wallet.payments"})
    ));
    assert_eq!(
        authorize(
            &wallet,
            Object::Streams,
            "Append",
            &json!({"stream": "other.payments"})
        ),
        Err(Denied::Target)
    );
}
