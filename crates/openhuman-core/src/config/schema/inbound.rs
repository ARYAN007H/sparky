//! `[inbound]` — inbound event queues (the `tinyeventqueues` module).
//!
//! Composio triggers and agent webhooks are ingested into the module, which
//! persists them on the host's storage, rejects redeliveries, filters and
//! paces them, and calls the host back to run triage (`Deliver`) — retrying a
//! deferred triage at the time it asked for instead of dropping it.
//!
//! The component switch is `[components] eventqueues`; `enabled` here turns
//! the routing off while leaving the component on. With either off — or with
//! the module unavailable on this host — events take the direct triage path.
//!
//! | setting | meaning |
//! | --- | --- |
//! | `enabled` | route inbound events through the queues |
//! | `queues_path` | a `queues.json` replacing the shipped one |
//! | `pipeline_path` | a `pipeline.json` replacing the shipped one |
//! | `queue_url` | where the queues live: `storage` (default) or a broker URL |
//! | `triage_timeout_ms` | how long one delivered event's triage may take |

use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Default bound on one delivered event's triage: below the module's shortest
/// lane lease (two minutes), so a slow triage retries rather than racing a
/// redelivery of the same turn.
pub const DEFAULT_TRIAGE_TIMEOUT_MS: u64 = 90_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct InboundConfig {
    /// Route Composio triggers and agent webhooks through the inbound event
    /// queues. Off, they take the direct triage path.
    pub enabled: bool,
    /// A `queues.json` (a storage `QueueManifest`) replacing the shipped one.
    /// The host declares exactly these queues on the module's scope.
    pub queues_path: Option<PathBuf>,
    /// A `pipeline.json` replacing the shipped one: gate, coalescing,
    /// salience, lanes, cadences and retry tuning.
    pub pipeline_path: Option<PathBuf>,
    /// Where the queues live: unset or `storage` keeps them in the module's
    /// storage scope; `amqp://` / `amqps://` selects a `RabbitMQ` broker
    /// (only in builds with the storage `rabbitmq` driver). Never logged.
    pub queue_url: Option<String>,
    /// How long one delivered event's triage may take, in milliseconds,
    /// before the delivery is retried.
    pub triage_timeout_ms: u64,
}

impl Default for InboundConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            queues_path: None,
            pipeline_path: None,
            queue_url: None,
            triage_timeout_ms: DEFAULT_TRIAGE_TIMEOUT_MS,
        }
    }
}

#[cfg(test)]
#[path = "inbound_tests.rs"]
mod tests;
