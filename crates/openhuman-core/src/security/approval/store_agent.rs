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
///
/// An embedded agent's requests are kept in its own storage scope, so the rows
/// of `Some(agent)` are read there whichever scope the caller runs in (under a
/// SaaS profile every agent shares the profile's scope, which is the caller's).
pub fn list_pending_for_agent(
    config: &Config,
    agent: Option<&str>,
) -> Result<Vec<PendingApproval>> {
    let rows = match agent {
        Some(agent) if !acts_for_a_profile() => {
            let docs = super::super::store_documents::for_agent(config, agent)?;
            super::publish_expired(&docs.expire_stale(chrono::Utc::now())?);
            docs.list_pending()?
        }
        _ => super::list_pending(config)?,
    };
    Ok(rows
        .into_iter()
        .filter(|row| row.belongs_to(agent))
        .collect())
}

/// Whether the calling task serves a SaaS profile, whose agents share one scope.
fn acts_for_a_profile() -> bool {
    crate::core::runtime::current_tenant().is_ok_and(|tenant| tenant.profile.is_some())
}

/// The agent that parked the still-undecided `request_id`: `Ok(None)` when no
/// such row exists, `Ok(Some(None))` for a row the process parked itself.
pub fn pending_agent(config: &Config, request_id: &str) -> Result<Option<Option<String>>> {
    super::super::store_documents::current(config)?.pending_agent(request_id)
}
