//! Handing an inbound event to the queues.
//!
//! [`offer`] is what an inbound path calls instead of running triage itself.
//! When the queues are active and take the event, the path is done: the
//! module persists it before it answers, so the platform may be acknowledged,
//! and it calls the host back (`Deliver`) when the event is due. When they
//! are inactive or unavailable (the module cannot load on this host, the call
//! fails, the event is too large to queue), the caller runs its direct path,
//! exactly as it did before the queues existed. Nothing is dropped by the
//! hand-off itself.

use tinybus::module::ModuleClient;
use tinyeventqueues_bus::{Disposition, InboundEvent};

use crate::config::Config;
use crate::core::bus::BUS;
use crate::core::events::DomainEvent;
use crate::modules::event_queues;

/// What became of an offered event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Offered {
    /// The queues own the event now: accepted, or ended at ingress (a
    /// redelivery of an event already taken, an invalid envelope). The caller
    /// does nothing more.
    Queued(Disposition),
    /// The queues did not take it; the caller runs its direct path.
    Direct(String),
}

impl Offered {
    /// Whether the queues own the event.
    #[must_use]
    pub fn is_queued(&self) -> bool {
        matches!(self, Self::Queued(_))
    }
}

/// Offer `event` to the inbound event queues through `client`.
pub async fn offer(config: &Config, client: &ModuleClient, event: InboundEvent) -> Offered {
    if !super::active(config) {
        return Offered::Direct("inbound event queues inactive".to_owned());
    }
    let source = event.source.clone();
    let event_id = event.event_id.clone();
    let response = match event_queues::ingest(client, vec![event]).await {
        Ok(response) => response,
        Err(error) => {
            tracing::warn!(
                source = %source,
                event_id = %event_id,
                error = %error,
                "[inbound] ingest failed; taking the direct path"
            );
            return Offered::Direct(format!("ingest failed: {error}"));
        }
    };
    let Some(result) = response.results.into_iter().next() else {
        tracing::warn!(source = %source, "[inbound] ingest answered no result; taking the direct path");
        return Offered::Direct("ingest answered no result".to_owned());
    };
    let reason = result
        .reason
        .and_then(|reason| serde_json::to_value(reason).ok())
        .and_then(|value| value.as_str().map(str::to_owned));
    match result.disposition {
        Disposition::Accepted => {
            tracing::debug!(source = %source, event_id = %event_id, "[inbound] queued");
        }
        disposition => {
            let reason = reason.unwrap_or_else(|| format!("{disposition:?}").to_lowercase());
            tracing::info!(
                source = %source,
                event_id = %event_id,
                reason = %reason,
                "[inbound] ended at ingress"
            );
            BUS.publish(DomainEvent::InboundEventDropped {
                source,
                event_id,
                reason,
            });
        }
    }
    Offered::Queued(result.disposition)
}

#[cfg(test)]
#[path = "route_tests.rs"]
mod tests;
