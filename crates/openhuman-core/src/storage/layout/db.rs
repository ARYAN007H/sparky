//! Typed layout facade for the physical legacy SQLite inventory and copy.
//!
//! All SQLite access to legacy databases lives under `storage::import`, behind
//! the `legacy-import` feature. Layout migration code depends on this narrow
//! wrapper and does not open or query SQLite itself.

pub(crate) use crate::storage::import::layout_db::mismatch;
pub use crate::storage::import::layout_db::{Counts, DbCopy, checkpoint, copy, table_counts};
