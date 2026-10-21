//! Read-only inventory of records in the v1 shared SQLite storage database.
//!
//! Scope enumeration was a v1 driver capability. The v2 backend intentionally
//! has no equivalent, so a host migration must identify its legacy database
//! explicitly and decode its rows here before writing them through typed v2
//! storage ports. This reader never changes the source file.

use std::path::Path;

use crate::storage::rusqlite::{self, Connection, OpenFlags};
use anyhow::{Context, Result, bail};
use serde_json::Value;
use tinystoragedrivers::Meta;

/// One live document row from the v1 storage database.
#[derive(Debug, Clone, PartialEq)]
pub struct LegacyDocument {
    /// The old scope string, which may not parse as a v2 [`Scope`].
    pub scope: String,
    /// Collection name.
    pub collection: String,
    /// Document id.
    pub id: String,
    /// Decoded JSON body.
    pub body: Value,
    /// Persisted driver metadata.
    pub meta: Meta,
}

/// One stream and its retained entries from the v1 database.
#[derive(Debug, Clone, PartialEq)]
pub struct LegacyStream {
    /// The old scope string.
    pub scope: String,
    /// Stream name.
    pub name: String,
    /// First retained sequence number.
    pub base: u64,
    /// Whether the stream was sealed.
    pub sealed: bool,
    /// Seal time when available.
    pub sealed_ms: Option<i64>,
    /// Ordered entries, each paired with its original sequence number.
    pub entries: Vec<(u64, Value)>,
}

/// One blob row from the v1 database.
#[derive(Debug, Clone, PartialEq)]
pub struct LegacyBlob {
    /// The old scope string.
    pub scope: String,
    /// Blob key.
    pub key: String,
    /// Content type when recorded.
    pub content_type: Option<String>,
    /// Absolute expiry when recorded.
    pub expires_ms: Option<i64>,
    /// Original bytes.
    pub bytes: Vec<u8>,
}

/// A complete, immutable snapshot of the v1 rows needed for a scope move.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LegacyScopeInventory {
    /// Live document rows.
    pub documents: Vec<LegacyDocument>,
    /// Streams and their entries.
    pub streams: Vec<LegacyStream>,
    /// Blobs.
    pub blobs: Vec<LegacyBlob>,
}

impl LegacyScopeInventory {
    /// Reads the legacy database without creating, migrating, or mutating it.
    ///
    /// # Errors
    ///
    /// Returns an error for an unreadable database, malformed JSON, duplicate
    /// or inconsistent stream sequences, or a missing required v1 table.
    pub fn read(path: &Path) -> Result<Self> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .with_context(|| format!("open legacy storage database {} read-only", path.display()))?;
        let mut inventory = Self::default();

        if table_exists(&conn, "docs")? {
            let mut statement = conn.prepare(
                "SELECT scope, coll, id, schema, created_ms, updated_ms, expires_ms, deleted, body
                 FROM docs ORDER BY scope, coll, id",
            )?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, u32>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                    row.get::<_, bool>(7)?,
                    row.get::<_, String>(8)?,
                ))
            })?;
            for row in rows {
                let (
                    scope,
                    collection,
                    id,
                    schema,
                    created_ms,
                    updated_ms,
                    expires_ms,
                    deleted,
                    body,
                ) = row?;
                if deleted {
                    continue;
                }
                inventory.documents.push(LegacyDocument {
                    scope,
                    collection,
                    id,
                    body: serde_json::from_str(&body).context("decode legacy document body")?,
                    meta: Meta {
                        schema,
                        created_ms,
                        updated_ms,
                        expires_ms,
                    },
                });
            }
        }

        let mut heads = std::collections::BTreeMap::new();
        if table_exists(&conn, "stream_heads")? {
            let mut statement = conn.prepare(
                "SELECT scope, name, base, len, sealed, sealed_ms
                 FROM stream_heads ORDER BY scope, name",
            )?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, u64>(2)?,
                    row.get::<_, u64>(3)?,
                    row.get::<_, bool>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                ))
            })?;
            for row in rows {
                let (scope, name, base, len, sealed, sealed_ms) = row?;
                heads.insert((scope, name), (base, len, sealed, sealed_ms));
            }
        }
        let mut stream_entries: std::collections::BTreeMap<(String, String), Vec<(u64, Value)>> =
            std::collections::BTreeMap::new();
        if table_exists(&conn, "streams")? {
            let mut statement = conn
                .prepare("SELECT scope, name, seq, body FROM streams ORDER BY scope, name, seq")?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, u64>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?;
            for row in rows {
                let (scope, name, seq, body) = row?;
                stream_entries.entry((scope, name)).or_default().push((
                    seq,
                    serde_json::from_str(&body).context("decode legacy stream entry")?,
                ));
            }
        }
        for (key, entries) in stream_entries {
            let (base, len, sealed, sealed_ms) = heads.remove(&key).unwrap_or((
                entries.first().map_or(0, |(seq, _)| *seq),
                entries.len() as u64,
                false,
                None,
            ));
            validate_stream(&key.0, &key.1, base, len, &entries)?;
            inventory.streams.push(LegacyStream {
                scope: key.0,
                name: key.1,
                base,
                sealed,
                sealed_ms,
                entries,
            });
        }
        for ((scope, name), (base, len, sealed, sealed_ms)) in heads {
            validate_stream(&scope, &name, base, len, &[])?;
            inventory.streams.push(LegacyStream {
                scope,
                name,
                base,
                sealed,
                sealed_ms,
                entries: Vec::new(),
            });
        }

        if table_exists(&conn, "blobs")? {
            let mut statement = conn.prepare(
                "SELECT scope, key, content_type, expires_ms, bytes FROM blobs ORDER BY scope, key",
            )?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                ))
            })?;
            for row in rows {
                let (scope, key, content_type, expires_ms, bytes) = row?;
                inventory.blobs.push(LegacyBlob {
                    scope,
                    key,
                    content_type,
                    expires_ms,
                    bytes,
                });
            }
        }
        Ok(inventory)
    }
}

fn table_exists(conn: &Connection, name: &str) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        [name],
        |row| row.get(0),
    )
}

fn validate_stream(
    scope: &str,
    name: &str,
    base: u64,
    len: u64,
    entries: &[(u64, Value)],
) -> Result<()> {
    let expected_len =
        u64::try_from(entries.len()).context("legacy stream has too many entries")?;
    if expected_len != len {
        bail!("legacy stream {scope}/{name} has head length {len} but {expected_len} rows");
    }
    for (offset, (actual, _)) in entries.iter().enumerate() {
        let expected = base + offset as u64;
        if *actual != expected {
            bail!("legacy stream {scope}/{name} is not contiguous at sequence {expected}");
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "scope_move_tests.rs"]
mod tests;
