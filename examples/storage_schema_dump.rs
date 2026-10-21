//! Prints the composed v2 collection declarations as JSON for the storage
//! schema documentation check.
//!
//! The dump reflects the `CollectionSpec` values the host prepares: names,
//! indexes, expiry fields, and search fields. Record shapes and migrations
//! remain owned by their domains.
//!
//! Run: `cargo run -p openhuman-cli --example storage_schema_dump`
//! Feature: `web3` adds the wallet owner's collections.

use openhuman_core::storage::schema::owner_collections;
use serde_json::json;

fn main() {
    let collections = owner_collections().expect("owner collection declarations compose");
    println!("{}", serde_json::to_string_pretty(&json!({ "collections": collections })).unwrap());
}
