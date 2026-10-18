//! [`DocumentLeases`]: the host's cluster leases on `tsd.leases`.
//!
//! The record, its compare-and-swap loop, the epoch rules and the write fence
//! live in tinystoragedrivers ([`tinystoragedrivers::Leases`]); this adapter
//! only fits them to [`LeaseStore`]: it supplies the caller's clock, the node
//! id (the lease holder) and endpoint, keeps the epoch this instance holds per
//! key so a repeated acquire is re-entrant while a restarted node (same id,
//! fresh instance) sees its own leftover record as an unclean takeover, and
//! maps the outcomes onto [`LeaseGrant`] and [`LeaseError`].
//!
//! One record per key under owner [`LEASE_OWNER`] in collection
//! [`LEASE_COLLECTION`] (`tsd.leases`) of the scope the caller binds (normally
//! [`CLUSTER_SCOPE`], see [`DocumentLeases::cluster`]). The record is
//! `{holder, epoch, expires_ms, endpoint?, released?}`. Exclusion across
//! processes needs a driver with cross-process CAS
//! ([`super::driver_has_cross_process_cas`]).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use tinystoragedrivers::primitives::{self, Acquire, Acquisition};
use tinystoragedrivers::{
    CollectionSpec, DocumentStore, Fence, Filter, Key, Leases, Scope, StorageBackend,
};

use super::lease::{
    validate_key, LeaseError, LeaseGrant, LeaseRecord, LeaseStore, CLUSTER_SCOPE, LEASE_COLLECTION,
};
use super::StorageError;

/// The `tsd.leases` owner of the host's process-placement leases.
pub const LEASE_OWNER: &str = "profiles";

/// Leases kept in `tsd.leases`. See the module docs.
pub struct DocumentLeases {
    docs: Arc<dyn DocumentStore>,
    /// The scope `docs` is bound to, when known ([`Self::cluster`]); a
    /// fencing driver reads the lease record from there.
    scope: Option<Scope>,
    node: String,
    endpoint: Option<String>,
    ttl_ms: i64,
    /// The epoch this instance holds per key (see the module docs).
    held: Mutex<HashMap<String, u64>>,
    /// Serializes `acquire` on this instance so an older completion cannot
    /// overwrite newer held state.
    acquire_gate: tokio::sync::Mutex<()>,
    declared: tokio::sync::OnceCell<()>,
}

impl std::fmt::Debug for DocumentLeases {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocumentLeases")
            .field("node", &self.node)
            .field("endpoint", &self.endpoint)
            .field("ttl_ms", &self.ttl_ms)
            .finish_non_exhaustive()
    }
}

impl DocumentLeases {
    /// Leases in `docs` (already bound to the shared scope) for `node`,
    /// reachable at `endpoint`, each grant lasting `ttl` past its last
    /// acquire or renew.
    pub fn new(
        docs: Arc<dyn DocumentStore>,
        node: impl Into<String>,
        endpoint: Option<String>,
        ttl: Duration,
    ) -> Self {
        Self {
            docs,
            scope: None,
            node: node.into(),
            endpoint,
            // A sub-millisecond TTL rounds up so a grant never expires at
            // the instant it is issued.
            ttl_ms: i64::try_from(ttl.as_millis()).unwrap_or(i64::MAX).max(1),
            held: Mutex::new(HashMap::new()),
            acquire_gate: tokio::sync::Mutex::new(()),
            declared: tokio::sync::OnceCell::new(),
        }
    }

    /// [`Self::new`] over `backend`'s [`CLUSTER_SCOPE`].
    ///
    /// # Errors
    ///
    /// When the backend refuses the scope.
    pub fn cluster(
        backend: &dyn StorageBackend,
        node: impl Into<String>,
        endpoint: Option<String>,
        ttl: Duration,
    ) -> Result<Self, StorageError> {
        let scope = Scope::new(CLUSTER_SCOPE)?;
        let scoped = backend.for_scope(&scope)?;
        Ok(Self::new(Arc::clone(scoped.documents()), node, endpoint, ttl).in_scope(scope))
    }

    /// Records that the document store this was built over is bound to
    /// `scope`, so storage fences can name the lease record
    /// ([`LeaseStore::driver_fence`]).
    #[must_use]
    pub fn in_scope(mut self, scope: Scope) -> Self {
        self.scope = Some(scope);
        self
    }

    /// This node's id.
    pub fn node(&self) -> &str {
        &self.node
    }

    /// The tsd leases handle whose clock reads `now_ms`, the caller's clock.
    fn leases_at(&self, now_ms: u64) -> Leases {
        Leases::new(&self.docs).with_clock(Arc::new(move || now_ms))
    }

    async fn declare(&self) -> Result<(), StorageError> {
        self.declared
            .get_or_try_init(|| async {
                self.docs
                    .ensure_collection(&CollectionSpec::new(LEASE_COLLECTION))
                    .await
            })
            .await
            .map(|_| ())
    }

    fn held_epoch(&self, key: &str) -> Option<u64> {
        self.held
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(key)
            .copied()
    }

    fn set_held(&self, key: &str, epoch: u64) {
        self.held
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(key.to_string(), epoch);
    }

    /// Drops the held entry only while it still names `epoch`, so a release
    /// finishing late cannot erase a newer acquisition's state.
    fn clear_held_if(&self, key: &str, epoch: u64) {
        let mut held = self
            .held
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if held.get(key) == Some(&epoch) {
            held.remove(key);
        }
    }

    /// The grant for a lease tsd just wrote.
    fn grant(key: &str, lease: &primitives::Lease, previous_unclean: bool) -> LeaseGrant {
        LeaseGrant {
            key: key.to_string(),
            epoch: lease.epoch,
            expires_at_ms: u64::try_from(lease.expires_ms).unwrap_or_default(),
            previous_unclean,
        }
    }
}

fn record_of(record: primitives::LeaseRecord) -> LeaseRecord {
    LeaseRecord {
        owner: record.holder,
        endpoint: record.endpoint,
        epoch: record.epoch,
        expires_at_ms: u64::try_from(record.expires_ms).unwrap_or_default(),
        released: record.released,
    }
}

#[async_trait]
impl LeaseStore for DocumentLeases {
    async fn acquire(&self, key: &str, now_ms: u64) -> Result<LeaseGrant, LeaseError> {
        validate_key(key)?;
        self.declare().await?;
        let _gate = self.acquire_gate.lock().await;
        let outcome = self
            .leases_at(now_ms)
            .acquire_with(Acquire {
                owner: LEASE_OWNER,
                key,
                holder: &self.node,
                ttl_ms: self.ttl_ms,
                endpoint: self.endpoint.as_deref(),
                held_epoch: self.held_epoch(key),
            })
            .await?;
        match outcome {
            Acquisition::Granted {
                lease,
                previous_unclean,
            } => {
                self.set_held(key, lease.epoch);
                tracing::debug!(
                    target: "openhuman::storage::lease",
                    key,
                    node = %self.node,
                    epoch = lease.epoch,
                    previous_unclean,
                    "[lease] acquired"
                );
                Ok(Self::grant(key, &lease, previous_unclean))
            }
            Acquisition::Held(record) => {
                tracing::debug!(
                    target: "openhuman::storage::lease",
                    key,
                    node = %self.node,
                    owner = %record.holder,
                    epoch = record.epoch,
                    "[lease] acquire refused: held elsewhere"
                );
                Err(LeaseError::Held(record_of(record)))
            }
        }
    }

    async fn renew(&self, grant: &LeaseGrant, now_ms: u64) -> Result<LeaseGrant, LeaseError> {
        validate_key(&grant.key)?;
        if now_ms >= grant.expires_at_ms || self.held_epoch(&grant.key) != Some(grant.epoch) {
            tracing::info!(
                target: "openhuman::storage::lease",
                key = %grant.key,
                node = %self.node,
                epoch = grant.epoch,
                "[lease] renew refused: grant expired or not held by this instance"
            );
            return Err(LeaseError::Lost);
        }
        self.declare().await?;
        let renewed = self
            .leases_at(now_ms)
            .renew(LEASE_OWNER, &grant.key, &self.node, grant.epoch, self.ttl_ms)
            .await?;
        let Some(lease) = renewed else {
            // The held entry stays: it names an epoch the record has moved
            // past, so it can never make a later acquire re-entrant.
            tracing::info!(
                target: "openhuman::storage::lease",
                key = %grant.key,
                node = %self.node,
                epoch = grant.epoch,
                "[lease] grant lost: the record changed since it was written"
            );
            return Err(LeaseError::Lost);
        };
        tracing::trace!(
            target: "openhuman::storage::lease",
            key = %grant.key,
            node = %self.node,
            epoch = grant.epoch,
            "[lease] renewed"
        );
        Ok(Self::grant(&grant.key, &lease, grant.previous_unclean))
    }

    async fn release(&self, grant: LeaseGrant) -> Result<(), LeaseError> {
        validate_key(&grant.key)?;
        if self.held_epoch(&grant.key) != Some(grant.epoch) {
            return Err(LeaseError::Lost);
        }
        self.declare().await?;
        // `release` ends the lease at the moment it is called, by tsd's own
        // clock reading; the caller's clock is only for acquire and renew.
        let released = self
            .leases_at(grant.expires_at_ms.saturating_sub(self.ttl_ms.unsigned_abs()))
            .release(LEASE_OWNER, &grant.key, &self.node, grant.epoch)
            .await?;
        if !released {
            return Err(LeaseError::Lost);
        }
        self.clear_held_if(&grant.key, grant.epoch);
        tracing::debug!(
            target: "openhuman::storage::lease",
            key = %grant.key,
            node = %self.node,
            epoch = grant.epoch,
            "[lease] released"
        );
        Ok(())
    }

    async fn holder(&self, key: &str) -> Result<Option<LeaseRecord>, LeaseError> {
        validate_key(key)?;
        self.declare().await?;
        Ok(self
            .leases_at(0)
            .inspect(LEASE_OWNER, key)
            .await?
            .map(record_of))
    }

    fn driver_fence(&self, key: &str, node: &str, epoch: u64) -> Option<Fence> {
        let scope = self.scope.clone()?;
        let id = Key::of([LEASE_OWNER, key]).ok()?;
        Some(Fence::new(
            scope,
            LEASE_COLLECTION,
            id.as_str().to_string(),
            Filter::eq("epoch", epoch)
                .and(Filter::eq("holder", node.to_string()))
                .and(Filter::exists("released", false)),
        ))
    }
}

#[cfg(test)]
#[path = "lease_documents_tests.rs"]
mod tests;
