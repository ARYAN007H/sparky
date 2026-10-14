//! The host side of `ai.tinyhumans.tinystorage.Storage`: durable storage for
//! loadable modules, served from the process's installed `StorageBackend`.
//!
//! A loaded module shares the process but not the host's storage handles. It
//! calls this interface instead of keeping its own files, and every call is
//! decided here, never by the module:
//!
//! * **Who is calling.** The module is the broker-authenticated sender of the
//!   call (`CallContext::authenticated_sender`), mapped back to a registry
//!   module id by asking the broker who owns each module's bus name. A call
//!   with no authenticated sender, or from a peer that is no registry module,
//!   is refused.
//! * **Which scope.** Never read from the request, which has no such field
//!   (the wire types reject unknown fields). The scope is derived from the
//!   module id and the host's current scope ([`crate::storage::scope_from`]):
//!   `module:<id>@<base>`, so two modules never share a scope and a module can
//!   never name another tenant's. In SaaS mode a call with no profile is
//!   refused like any other storage use.
//! * **What it may touch.** The module's [`StorageGrant`], declared by the
//!   host build ([`super::storage_grants::declared`]): its collections, stream,
//!   blob and secret prefixes, and whether it may write. Anything else is a
//!   refusal before the backend is reached.
//! * **Schema.** Only the host runs `ensure_collection`, from the grant's
//!   declared specs, when it binds a module to a scope.
//! * **Fencing.** The scope handle comes from the installed backend, which the
//!   core opened wrapped in its lease fences ([`crate::storage::open`]), so a
//!   module write under a stale epoch fails `Fenced`; the module never
//!   supplies or sees a fence.
//! * **Audit.** One line per call: module id, scope, object, operation, target
//!   (a collection or a granted prefix) and outcome. Never a payload.
//!
//! The reference `StorageServer` of `tinystoragedrivers-client` answers the
//! messages once a call is allowed; this file adds only the host's decisions.
//!
//! Blobs over a bus stream (`PutStream`/`GetStream`) and watches (`WatchOpen`)
//! go through `tinystoragedrivers-busproxy`, which binds them to the peer: an
//! upload is read with `Connection::accept_stream_from` only from the
//! authenticated caller that opened it, a download is opened toward that
//! caller, and a watch's invalidations are unicast to the subscribing caller
//! with `Connection::emit_to`. Every decision above still applies first.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tinybus::BusName;
use tinystoragedrivers::secrets::{DocumentSecrets, KeyProvider};
use tinystoragedrivers::BlobReader;
use tinystoragedrivers_bus::{
    BlobKeyRequest, BlobStreamHead, PutBlobStreamRequest, StorageFailure, StorageKind,
    WatchChanged,
};
use tinystoragedrivers_busproxy::{HostFailure, StorageCall, StorageEndpoint};
use tokio::sync::broadcast;

use tinystoragedrivers_client::{Admission, StorageServer};

use super::storage_grants::{authorize, declared, filter_reply, Object, Plan, StorageGrant};
use crate::storage::{Scope, StorageBackend, StorageError};

/// The largest blob a module may send by `PutStream`. The body is bounded per
/// chunk by the bus and by its declared length by the server, so this is the
/// host's own ceiling on one module's single upload.
const MAX_STREAMED_BLOB_BYTES: u64 = 256 * 1024 * 1024;
/// Relays kept for callers that never became a bound module.
const MAX_IDLE_RELAYS: usize = 256;

/// Maps a broker-authenticated sender to a registry module id.
#[async_trait]
pub(crate) trait CallerDirectory: Send + Sync {
    /// The module that owns `sender`, if it is one.
    async fn module_for(&self, sender: &BusName) -> Option<String>;
}

/// What the host decides from, separated so tests can supply their own.
#[async_trait]
pub(crate) trait Environment: Send + Sync {
    /// The installed backend, if the host configured one.
    fn backend(&self) -> Option<Arc<dyn StorageBackend>>;
    /// The scope `module_id`'s data lives in: the host's current scope with the
    /// module added ([`module_scope`]).
    fn module_scope(&self, module_id: &str) -> Result<Scope, StorageError>;
    /// The module's declared grant.
    fn grant(&self, module_id: &str) -> Option<StorageGrant>;
    /// The key provider for module secrets.
    fn keys(&self) -> Result<Arc<dyn KeyProvider>, StorageError>;
    /// Consults the host's write fence for `scope` before a write is forwarded
    /// (the backend checks it again). A refusal means the lease behind the
    /// scope is gone.
    async fn admit_write(&self, scope: &Scope) -> Result<(), StorageError>;
}

/// One line of the audit trail. Carries no payload by construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AuditRecord {
    pub module: String,
    pub scope: String,
    pub object: &'static str,
    pub op: String,
    pub target: String,
    pub outcome: &'static str,
}

/// Receives every audit record.
pub(crate) trait AuditSink: Send + Sync {
    fn record(&self, record: &AuditRecord);
}

struct LogAudit;

impl AuditSink for LogAudit {
    fn record(&self, record: &AuditRecord) {
        tracing::info!(
            target: "openhuman::storage_host",
            module = %record.module,
            scope = %record.scope,
            object = record.object,
            op = %record.op,
            target = %record.target,
            outcome = record.outcome,
            "[storage_host] module storage call"
        );
    }
}

/// How a call failed, before it becomes a bus error.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum HostError {
    /// Not allowed; the label says why and holds no request content.
    Denied(&'static str),
    /// The backend refused or failed, as a content-free category.
    Storage(StorageFailure),
}

impl From<HostError> for HostFailure {
    fn from(error: HostError) -> Self {
        match error {
            HostError::Denied(label) => HostFailure::Denied(label),
            HostError::Storage(failure) => HostFailure::Storage(failure),
        }
    }
}

impl From<HostError> for tinybus::Error {
    fn from(error: HostError) -> Self {
        HostFailure::from(error).into()
    }
}

fn failure(kind: StorageKind) -> HostError {
    HostError::Storage(StorageFailure { kind })
}

/// A module's scope under the host's current tenant.
///
/// * For a profile (SaaS): `<profile>~module:<id>`, with the profile escaped
///   as a session key is. That sits under the profile's session-key prefix,
///   so the profile's lease fence ([`crate::profiles`]) guards a module's
///   writes exactly as it guards the profile's transcripts. Too long to be a
///   scope is an error, never a hash, because a hash would fall outside the
///   fence.
/// * Otherwise: `module:<id>@<base>`, or a hash of both when that is not a
///   valid scope or the id could make the form ambiguous.
///
/// Either way two modules never share a scope, and nothing the module sends
/// takes part.
pub(crate) fn module_scope(
    profile: Option<&str>,
    base: &Scope,
    module_id: &str,
) -> Result<Scope, StorageError> {
    let plain = !module_id.is_empty()
        && module_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
    if let Some(profile) = profile {
        if !plain {
            return Err(StorageError::invalid_input("module id is not a plain name"));
        }
        let prefix = crate::core::runtime::session_key_prefix(profile);
        return Scope::new(format!("{prefix}module:{module_id}"));
    }
    if plain {
        if let Ok(scope) = Scope::new(format!("module:{module_id}@{}", base.as_str())) {
            return Ok(scope);
        }
    }
    use sha2::Digest;
    let mut digest = sha2::Sha256::new();
    digest.update(module_id.as_bytes());
    digest.update([0]);
    digest.update(base.as_str().as_bytes());
    Scope::new(format!("module-sha256:{}", hex::encode(digest.finalize())))
}

/// A module bound to a scope: its grant and the server that answers it.
struct Binding {
    backend: Arc<dyn StorageBackend>,
    grant: StorageGrant,
    server: StorageServer,
}

pub(crate) struct StorageHost {
    directory: Arc<dyn CallerDirectory>,
    environment: Arc<dyn Environment>,
    audit: Arc<dyn AuditSink>,
    bindings: tokio::sync::Mutex<HashMap<(String, String), Arc<Binding>>>,
    relays: std::sync::Mutex<HashMap<BusName, Relay>>,
}

/// Where one caller's watch invalidations are collected for the bus endpoint.
///
/// The endpoint asks for a caller's signals synchronously and before the
/// caller's first `WatchOpen` reaches the host, when the host has not yet
/// resolved which module the caller is. So the relay exists per caller from
/// the first ask, and [`StorageHost::handle`] feeds it from the caller's bound
/// server as soon as the caller is a known module (before the call is
/// forwarded, so no signal is missed).
struct Relay {
    tx: broadcast::Sender<WatchChanged>,
    /// The binding currently feeding `tx`, so a rebind starts a new bridge.
    fed_by: Option<std::sync::Weak<Binding>>,
    bridge: Option<tokio::task::JoinHandle<()>>,
}

impl Drop for Relay {
    fn drop(&mut self) {
        if let Some(bridge) = self.bridge.take() {
            bridge.abort();
        }
    }
}

/// Signals buffered per caller before the slowest reader loses the oldest.
const RELAY_BUFFER: usize = 256;

impl StorageHost {
    pub(crate) fn new(
        directory: Arc<dyn CallerDirectory>,
        environment: Arc<dyn Environment>,
        audit: Arc<dyn AuditSink>,
    ) -> Self {
        Self {
            directory,
            environment,
            audit,
            bindings: tokio::sync::Mutex::new(HashMap::new()),
            relays: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// The watch invalidations of `sender`, for the bus endpoint to unicast.
    fn watch_signals(&self, sender: &BusName) -> Option<broadcast::Receiver<WatchChanged>> {
        let mut relays = self.relays.lock().ok()?;
        if !relays.contains_key(sender) {
            // Callers that never became a module leave an idle relay behind;
            // keep those bounded.
            if relays.len() >= MAX_IDLE_RELAYS {
                relays.retain(|_, relay| relay.bridge.is_some() || relay.tx.receiver_count() > 0);
            }
            if relays.len() >= MAX_IDLE_RELAYS {
                return None;
            }
            let (tx, _) = broadcast::channel(RELAY_BUFFER);
            relays.insert(
                sender.clone(),
                Relay {
                    tx,
                    fed_by: None,
                    bridge: None,
                },
            );
        }
        relays.get(sender).map(|relay| relay.tx.subscribe())
    }

    /// Starts feeding `sender`'s relay from `binding`'s server, replacing a
    /// bridge from an earlier binding. The bridge subscribes before it is
    /// spawned, so nothing the server signals afterwards is lost.
    fn feed_relay(&self, sender: &BusName, binding: &Arc<Binding>) {
        let Ok(mut relays) = self.relays.lock() else {
            return;
        };
        let Some(relay) = relays.get_mut(sender) else {
            // Nobody asked for this caller's signals (it never opened a watch
            // through the bus), so there is nothing to feed.
            return;
        };
        let current = relay
            .fed_by
            .as_ref()
            .is_some_and(|fed| fed.ptr_eq(&Arc::downgrade(binding)));
        if current && relay.bridge.as_ref().is_some_and(|task| !task.is_finished()) {
            return;
        }
        if let Some(stale) = relay.bridge.take() {
            stale.abort();
        }
        let mut signals = binding.server.signals();
        let tx = relay.tx.clone();
        relay.fed_by = Some(Arc::downgrade(binding));
        relay.bridge = Some(tokio::spawn(async move {
            loop {
                match signals.recv().await {
                    Ok(signal) => {
                        // No receiver just means the bus forwarder is gone.
                        let _ = tx.send(signal);
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => return,
                }
            }
        }));
    }

    /// Resolves who is calling and decides the call: caller, scope, binding and
    /// grant. Everything a forwarded call needs, or the refusal.
    async fn admit(
        &self,
        object: Object,
        selector: &str,
        request: &Value,
        sender: Option<&BusName>,
    ) -> Result<Admitted, HostError> {
        let Some(sender) = sender else {
            tracing::warn!(
                target: "openhuman::storage_host",
                object = object.as_str(),
                op = selector,
                "[storage_host] refused a call with no authenticated sender"
            );
            return Err(HostError::Denied("unauthenticated-caller"));
        };
        let Some(module) = self.directory.module_for(sender).await else {
            tracing::warn!(
                target: "openhuman::storage_host",
                object = object.as_str(),
                op = selector,
                "[storage_host] refused a call from a peer that is not a registry module"
            );
            return Err(HostError::Denied("unknown-caller"));
        };
        let scope = self.environment.module_scope(&module);
        let mut call = AuditedCall {
            audit: Arc::clone(&self.audit),
            module,
            scope: scope
                .as_ref()
                .map_or_else(|_| "-".to_string(), |scope| scope.as_str().to_string()),
            object,
            op: selector.to_string(),
        };
        let scope = match scope {
            Ok(scope) => scope,
            Err(_) => {
                call.record("", "no-scope");
                return Err(HostError::Denied("no-scope"));
            }
        };
        let binding = match self.bind(&call.module, scope.clone()).await {
            Ok(binding) => binding,
            Err(error) => {
                call.record("", error.label());
                return Err(error.into_host());
            }
        };
        // Before the call reaches the server: a watch it opens must find the
        // caller's relay already listening.
        self.feed_relay(sender, &binding);
        let plan = match authorize(&binding.grant, object, selector, request) {
            Ok(plan) => plan,
            Err(denied) => {
                call.record("", denied.label());
                return Err(HostError::Denied(denied.label()));
            }
        };
        Ok(Admitted {
            call,
            scope,
            binding,
            plan,
        })
    }

    /// Consults the write fence when the plan writes. `Err` is already audited.
    async fn fence(&self, admitted: &Admitted, target: &str, write: bool) -> Result<(), HostError> {
        if !write {
            return Ok(());
        }
        if let Err(refused) = self.environment.admit_write(&admitted.scope).await {
            let kind = if crate::storage::fence::fence_error(&refused).is_some() {
                StorageKind::Fenced
            } else {
                StorageKind::Backend
            };
            admitted.call.record(target, "fenced");
            return Err(failure(kind));
        }
        Ok(())
    }

    /// Decides and answers one call.
    ///
    /// `sender` is the authenticated sender the bus delivered, never anything
    /// from the request. `admission` is `Confidential` only for a call the bus
    /// delivered on a confidential route.
    pub(crate) async fn handle(
        &self,
        object: Object,
        selector: &str,
        request: Value,
        sender: Option<&BusName>,
        admission: Admission,
    ) -> Result<Value, HostError> {
        let admitted = self.admit(object, selector, &request, sender).await?;
        match &admitted.plan {
            Plan::Answer(reply) => {
                admitted.call.record("", "answered-by-host");
                Ok(reply.clone())
            }
            Plan::Forward {
                target,
                filter,
                write,
            } => {
                self.fence(&admitted, target, *write).await?;
                match admitted
                    .binding
                    .server
                    .handle(admission, object.as_str(), selector, request)
                    .await
                {
                    Ok(reply) => {
                        admitted.call.record(target, "ok");
                        Ok(filter_reply(&admitted.binding.grant, *filter, reply))
                    }
                    Err(failed) => {
                        admitted.call.record(target, "storage-error");
                        Err(HostError::Storage(failed))
                    }
                }
            }
        }
    }

    /// `Blobs.PutStream`: stores `body`, which the bus delivered from a stream
    /// the authenticated caller opened, as exactly `head.len` bytes.
    pub(crate) async fn put_blob_stream(
        &self,
        sender: Option<&BusName>,
        head: PutBlobStreamRequest,
        body: BlobReader,
    ) -> Result<Value, HostError> {
        let request = serde_json::to_value(&head).map_err(|_| HostError::Denied("malformed-request"))?;
        let admitted = self.admit(Object::Blobs, "PutStream", &request, sender).await?;
        let Plan::Forward { target, write, .. } = &admitted.plan else {
            return Err(HostError::Denied("malformed-request"));
        };
        if head.len > MAX_STREAMED_BLOB_BYTES {
            admitted.call.record(target, "blob-too-large");
            return Err(HostError::Denied("blob-too-large"));
        }
        self.fence(&admitted, target, *write).await?;
        match admitted.binding.server.put_blob_stream(head, body).await {
            Ok(info) => {
                admitted.call.record(target, "ok");
                serde_json::to_value(info).map_err(|_| HostError::Denied("encode-failed"))
            }
            Err(failed) => {
                admitted.call.record(target, "storage-error");
                Err(HostError::Storage(failed))
            }
        }
    }

    /// `Blobs.GetStream`: the blob's metadata and body, or `None`.
    pub(crate) async fn get_blob_stream(
        &self,
        sender: Option<&BusName>,
        request: BlobKeyRequest,
    ) -> Result<Option<(BlobStreamHead, BlobReader)>, HostError> {
        let wire = serde_json::to_value(&request).map_err(|_| HostError::Denied("malformed-request"))?;
        let admitted = self.admit(Object::Blobs, "GetStream", &wire, sender).await?;
        let Plan::Forward { target, .. } = &admitted.plan else {
            return Err(HostError::Denied("malformed-request"));
        };
        match admitted.binding.server.get_blob_stream(request).await {
            Ok(found) => {
                admitted.call.record(target, "ok");
                Ok(found)
            }
            Err(failed) => {
                admitted.call.record(target, "storage-error");
                Err(HostError::Storage(failed))
            }
        }
    }

    /// The module's binding for `scope`, built on first use and again when the
    /// installed backend changed. The host runs the declared schema here.
    async fn bind(&self, module: &str, scope: Scope) -> Result<Arc<Binding>, BindError> {
        let backend = self.environment.backend().ok_or(BindError::NoBackend)?;
        let key = (module.to_string(), scope.as_str().to_string());
        let mut bindings = self.bindings.lock().await;
        if let Some(existing) = bindings.get(&key) {
            if Arc::ptr_eq(&existing.backend, &backend) {
                return Ok(Arc::clone(existing));
            }
        }
        let grant = self.environment.grant(module).ok_or(BindError::NoGrant)?;
        grant.validate().map_err(|reason| {
            tracing::error!(
                target: "openhuman::storage_host",
                module,
                reason,
                "[storage_host] refused an invalid storage grant"
            );
            BindError::BadGrant
        })?;
        let storage = backend
            .for_scope(&scope)
            .map_err(|_| BindError::Backend(StorageKind::Backend))?;
        for spec in grant.specs() {
            storage
                .documents()
                .ensure_collection(spec)
                .await
                .map_err(|_| BindError::Backend(StorageKind::Backend))?;
        }
        let mut server = StorageServer::new(storage.clone());
        if !grant.secret_prefixes.is_empty() {
            // A key-provider failure is retryable, so refuse the bind rather
            // than cache a server that silently lacks secrets.
            let keys = self.environment.keys().map_err(|_| {
                tracing::warn!(
                    target: "openhuman::storage_host",
                    module,
                    "[storage_host] no master key; refusing to bind a grant with secrets"
                );
                BindError::Backend(StorageKind::Backend)
            })?;
            server = server.with_secrets(Arc::new(DocumentSecrets::new(&storage, keys)));
        }
        let binding = Arc::new(Binding {
            backend,
            grant,
            server,
        });
        bindings.insert(key, Arc::clone(&binding));
        Ok(binding)
    }
}

/// A call that passed identification, with what answering it needs.
struct Admitted {
    call: AuditedCall,
    scope: Scope,
    binding: Arc<Binding>,
    plan: Plan,
}

/// The audit line of one call, minus its target and outcome.
struct AuditedCall {
    audit: Arc<dyn AuditSink>,
    module: String,
    scope: String,
    object: Object,
    op: String,
}

impl AuditedCall {
    fn record(&self, target: &str, outcome: &'static str) {
        self.audit.record(&AuditRecord {
            module: self.module.clone(),
            scope: self.scope.clone(),
            object: self.object.as_str(),
            op: self.op.clone(),
            target: target.to_string(),
            outcome,
        });
    }
}

enum BindError {
    NoBackend,
    NoGrant,
    BadGrant,
    Backend(StorageKind),
}

impl BindError {
    fn label(&self) -> &'static str {
        match self {
            Self::NoBackend => "no-backend",
            Self::NoGrant => "no-grant-declared",
            Self::BadGrant => "invalid-grant",
            Self::Backend(_) => "backend-error",
        }
    }

    fn into_host(self) -> HostError {
        match self {
            Self::NoBackend => failure(StorageKind::Unavailable),
            Self::NoGrant => HostError::Denied("no-grant-declared"),
            Self::BadGrant => HostError::Denied("invalid-grant"),
            Self::Backend(kind) => failure(kind),
        }
    }
}

/// The host as the bus endpoint `tinystoragedrivers-busproxy` serves the four
/// objects from. The proxy owns the wire (positional arguments, the peer-bound
/// stream accept and open, unicast watch signals); every decision stays in
/// [`StorageHost`].
pub(super) struct HostEndpoint {
    host: Arc<StorageHost>,
    /// Where the work runs when the objects are served from a linked module's
    /// own runtime: the host's, which the backend handles were opened on.
    runtime: Option<tokio::runtime::Handle>,
}

impl HostEndpoint {
    pub(super) fn new(host: Arc<StorageHost>, runtime: Option<tokio::runtime::Handle>) -> Self {
        Self { host, runtime }
    }

    /// Runs `work` on the host's runtime when one was given.
    async fn on_host<T, F>(&self, work: F) -> Result<T, HostFailure>
    where
        T: Send + 'static,
        F: std::future::Future<Output = Result<T, HostError>> + Send + 'static,
    {
        match &self.runtime {
            None => work.await.map_err(Into::into),
            Some(runtime) => runtime
                .spawn(work)
                .await
                .map_err(|_| HostFailure::Denied("call-cancelled"))?
                .map_err(Into::into),
        }
    }
}

#[async_trait]
impl StorageEndpoint for HostEndpoint {
    async fn call(&self, call: StorageCall<'_>) -> Result<Value, HostFailure> {
        let host = Arc::clone(&self.host);
        let (object, request, admission) = (call.object, call.request, call.admission);
        let selector = call.selector.to_string();
        let sender = call.sender.cloned();
        self.on_host(async move {
            host.handle(object, &selector, request, sender.as_ref(), admission)
                .await
        })
        .await
    }

    async fn put_blob_stream(
        &self,
        sender: Option<&BusName>,
        head: PutBlobStreamRequest,
        body: BlobReader,
    ) -> Result<Value, HostFailure> {
        let host = Arc::clone(&self.host);
        let sender = sender.cloned();
        self.on_host(async move { host.put_blob_stream(sender.as_ref(), head, body).await })
            .await
    }

    async fn get_blob_stream(
        &self,
        sender: Option<&BusName>,
        request: BlobKeyRequest,
    ) -> Result<Option<(BlobStreamHead, BlobReader)>, HostFailure> {
        let host = Arc::clone(&self.host);
        let sender = sender.cloned();
        self.on_host(async move { host.get_blob_stream(sender.as_ref(), request).await })
            .await
    }

    fn watch_signals(&self, sender: &BusName) -> Option<broadcast::Receiver<WatchChanged>> {
        self.host.watch_signals(sender)
    }
}

/// The scope the host serves `module_id` under right now: what a bus call from
/// that module would be bound to. The host's own one-time legacy imports use it
/// to write a module's records where the module will read them.
///
/// # Errors
///
/// When the acting scope cannot be resolved (SaaS mode with no profile) or the
/// module id is not a plain name under a profile.
pub(crate) fn current_module_scope(module_id: &str) -> Result<Scope, StorageError> {
    let saas = crate::core::runtime::mode::is_saas();
    // A module call arrives on the module bus, outside any turn, so with no
    // tenant the host's default scope applies (refused in SaaS mode).
    let tenant = crate::core::runtime::current_tenant().unwrap_or_default();
    let base =
        crate::storage::scope_from(tenant.profile.as_deref(), tenant.agent.as_deref(), saas)?;
    module_scope(tenant.profile.as_deref(), &base, module_id)
}

/// Production wiring: the broker's name owners, the installed backend, the
/// host's scope rule and the compiled grants.
struct Live {
    connection: tinybus::Connection,
}

#[async_trait]
impl CallerDirectory for Live {
    async fn module_for(&self, sender: &BusName) -> Option<String> {
        for record in super::registry::ALL {
            if let Ok(Some(owner)) = self.connection.name_owner(record.bus_name).await {
                if &owner == sender {
                    return Some(record.id.to_string());
                }
            }
        }
        None
    }
}

#[async_trait]
impl Environment for Live {
    fn backend(&self) -> Option<Arc<dyn StorageBackend>> {
        crate::storage::installed()
    }

    fn module_scope(&self, module_id: &str) -> Result<Scope, StorageError> {
        current_module_scope(module_id)
    }

    fn grant(&self, module_id: &str) -> Option<StorageGrant> {
        declared(module_id)
    }

    fn keys(&self) -> Result<Arc<dyn KeyProvider>, StorageError> {
        crate::storage::secrets::keys()
    }

    async fn admit_write(&self, scope: &Scope) -> Result<(), StorageError> {
        crate::storage::fence::registry()
            .guard(scope.as_str(), "module-write")
            .await
    }
}

/// The production host: the live directory, environment and audit log.
pub(super) fn live_host(connection: &tinybus::Connection) -> Arc<StorageHost> {
    let live = Arc::new(Live {
        connection: connection.clone(),
    });
    Arc::new(StorageHost::new(live.clone(), live, Arc::new(LogAudit)))
}

/// Serves the four objects and claims the interface name on `connection`.
pub(super) async fn serve_objects(
    connection: &tinybus::Connection,
    host: &Arc<StorageHost>,
    runtime: Option<tokio::runtime::Handle>,
) -> tinybus::Result<tinystoragedrivers_busproxy::ServedStorage> {
    tinystoragedrivers_busproxy::serve(
        connection,
        Arc::new(HostEndpoint::new(Arc::clone(host), runtime)),
    )
    .await
}

/// Serves storage to modules.
///
/// Secrets travel only on tinybus' confidential route, and the broker delivers
/// that route to an attested recipient alone: a loaded module whose artifact
/// was verified, or code linked into this executable. The host's own client
/// connection is neither, so serving from it would refuse every secret
/// `NotAttested`. The objects are therefore served by a *linked* module
/// ([`super::storage_host_linked`]). If that cannot be attached the objects
/// are served from the host connection as before, with Secrets unreachable,
/// and the failure is logged.
pub(super) async fn install(
    connection: &tinybus::Connection,
    modules: &tinybus::module::ModuleHost,
) -> tinybus::Result<()> {
    let host = live_host(connection);
    match super::storage_host_linked::attach(modules, connection, Arc::clone(&host)).await {
        Ok(()) => {
            tracing::debug!("[storage_host] serving storage from the attested linked module");
            Ok(())
        }
        Err(error) => {
            tracing::warn!(
                %error,
                "[storage_host] linked storage module unavailable; module secrets are refused"
            );
            serve_objects(connection, &host, None).await.map(|served| {
                // The objects stay registered for the life of the connection.
                drop(served);
            })
        }
    }
}

#[cfg(test)]
#[path = "storage_host_tests.rs"]
mod tests;
