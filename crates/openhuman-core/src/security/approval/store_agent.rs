//! Approval rows by the embedded agent that parked them, and whether the
//! store exists to hold any.

use anyhow::Result;

use super::{Config, PendingApproval};

/// Whether the approval store exists for `config`'s workspace. The document
/// store is created on first use, so it always does.
pub fn exists(_config: &Config) -> bool {
    true
}

/// [`list_pending`](super::list_pending) narrowed to the rows of `agent` (see
/// [`PendingApproval::belongs_to`]).
pub fn list_pending_for_agent(
    config: &Config,
    agent: Option<&str>,
) -> Result<Vec<PendingApproval>> {
    Ok(super::list_pending(config)?
        .into_iter()
        .filter(|row| row.belongs_to(agent))
        .collect())
}

/// The agent that parked the still-undecided `request_id`: `Ok(None)` when no
/// such row exists, `Ok(Some(None))` for a row the process parked itself.
pub fn pending_agent(config: &Config, request_id: &str) -> Result<Option<Option<String>>> {
    super::super::store_documents::current(config)?.pending_agent(request_id)
}
