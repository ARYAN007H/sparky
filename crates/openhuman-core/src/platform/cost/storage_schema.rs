//! The `cost` owner's storage schema: the usage ledger and the step that
//! brings the v1 collection forward.
//!
//! | v1 | v2 | change |
//! |---|---|---|
//! | `cost_records` (`usage.timestamp` RFC 3339 beside `ts_ms`) | `cost.records` | `usage.timestamp` dropped; `ts_ms` is the instant |

use serde_json::Value;
use tinystoragedrivers::{CollectionSpec, IndexSpec};

use super::types::CostRecord;

pub(super) const RECORDS: &str = "cost.records";
/// Collections owned by the cost ledger.
pub(crate) fn collection_specs() -> Vec<CollectionSpec> {
    vec![spec()]
}

pub(super) fn spec() -> CollectionSpec {
    CollectionSpec::new(RECORDS).index(IndexSpec::new("by_time", ["ts_ms"]))
}

/// A record as its stored body: its fields with the usage timestamp carried
/// once, as `ts_ms`.
///
/// # Errors
///
/// When the record does not serialize.
pub(super) fn to_body(record: &CostRecord) -> anyhow::Result<Value> {
    let mut doc = serde_json::to_value(record)?;
    if let Some(usage) = doc.get_mut("usage").and_then(Value::as_object_mut) {
        usage.remove("timestamp");
    }
    doc["ts_ms"] = record.usage.timestamp.timestamp_millis().into();
    Ok(doc)
}

/// The record a stored body holds.
///
/// # Errors
///
/// When the body is not a record.
pub(super) fn from_body(mut doc: Value) -> anyhow::Result<CostRecord> {
    let ts_ms = doc.get("ts_ms").and_then(Value::as_i64).unwrap_or_default();
    let at = chrono::DateTime::from_timestamp_millis(ts_ms).unwrap_or_default();
    if let Some(usage) = doc.get_mut("usage").and_then(Value::as_object_mut) {
        usage.insert("timestamp".into(), serde_json::json!(at));
    }
    Ok(serde_json::from_value(doc)?)
}

#[cfg(test)]
#[path = "storage_schema_tests.rs"]
mod tests;
