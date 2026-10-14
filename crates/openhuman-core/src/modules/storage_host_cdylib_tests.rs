//! The storage host against a REAL loaded module (#7772).
//!
//! `tests/fixtures/storage_probe_module` is a cdylib that relays one raw call
//! to `ai.tinyhumans.tinystorage.Storage` and reports the answer. Here it is
//! built, hash-pinned in a `modules.toml` beside it and loaded through
//! `ModuleHost::load_file`, so every call below crosses the real ABI, the real
//! broker and the real `authenticated_sender` rule of a loaded module. The
//! host double only decides which registry id the probe's peer stands for
//! (`tinymcp` or `tinyjuice`, so the compiled grants apply).
//!
//! What this proves, per the issue:
//! * a module cannot reach another module's collections, another scope, or a
//!   collection its grant never declared;
//! * a blob over the inline cap (1 MiB) is refused and stores nothing;
//! * a blob over the 16 MiB bus frame round-trips by `PutStream`/`GetStream`
//!   on a peer-bound `tinybus.Stream`, and a body no frame could carry is a
//!   clear error instead of a hung call;
//! * a watch invalidation reaches the module that opened the watch, and no
//!   other module sees it;
//! * a second module can neither read the first module's blobs nor take the
//!   stream the first opened.

use super::*;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use serde_json::{json, Value};
use tinybus::broker::Broker;
use tinybus::module::ModuleHost;
use tinybus::transport::memory::MemoryBus;
use tinybus::{Connection, Proxy};
use tinystoragedrivers::secrets::{DerivedKeys, KeyProvider};
use zeroize::Zeroizing;

use crate::modules::storage_grants::{declared, StorageGrant};
use crate::modules::storage_host::{
    module_scope, AuditRecord, AuditSink, CallerDirectory, Environment, StorageHost,
};
use crate::modules::storage_host_linked::attach;
use crate::storage::{Scope, StorageBackend, StorageError};
use async_trait::async_trait;
use std::sync::Arc;
use tinybus::BusName;
use tinystoragedrivers_bus::STORAGE_INTERFACE;

const PROBE: &str = "ai.tinyhumans.test.StorageProbe";

use crate::modules::storage_host_linked::TEST_TURN as TURN;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

/// The built probe library: `OPENHUMAN_STORAGE_PROBE_MODULE` when set, else a
/// build of the fixture under the checkout's own `target/storage-probe`.
fn probe_artifact() -> &'static Path {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT.get_or_init(|| {
        if let Some(path) = std::env::var_os("OPENHUMAN_STORAGE_PROBE_MODULE") {
            return PathBuf::from(path);
        }
        let root = repo_root();
        let target = root.join("target/storage-probe");
        let status =
            std::process::Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
                .args(["build", "--quiet", "--manifest-path"])
                .arg(root.join("tests/fixtures/storage_probe_module/Cargo.toml"))
                .arg("--target-dir")
                .arg(&target)
                // The test run's own flags (coverage instrumentation, a linker choice)
                // must not leak into a fixture that is a different build.
                .env_remove("RUSTFLAGS")
                .env_remove("CARGO_ENCODED_RUSTFLAGS")
                .env_remove("RUSTC_WRAPPER")
                .env_remove("CARGO_TARGET_DIR")
                .status()
                .expect("run cargo to build the storage probe module");
        assert!(status.success(), "building the storage probe module failed");
        target.join("debug").join(format!(
            "{}storage_probe_module{}",
            std::env::consts::DLL_PREFIX,
            std::env::consts::DLL_SUFFIX
        ))
    })
}

/// Stages the probe beside a `modules.toml` pinning its digest, which is what
/// makes a loaded module an attested recipient.
fn stage(directory: &Path) -> PathBuf {
    let artifact = probe_artifact();
    let name = artifact.file_name().expect("artifact name");
    let staged = directory.join(name);
    std::fs::copy(artifact, &staged).expect("stage the probe");
    let hash = tinybus::module::sha256_file(&staged).expect("digest");
    std::fs::write(
        directory.join("modules.toml"),
        format!("{:?} = {hash:?}\n", name.to_str().expect("utf-8 name")),
    )
    .expect("write modules.toml");
    staged
}

/// Names each listed peer after the registry module it stands for, and nobody
/// else: the probe, and any extra peers a test adds.
struct ProbeDirectory {
    connection: Connection,
    peers: Vec<(&'static str, &'static str)>,
}

#[async_trait]
impl CallerDirectory for ProbeDirectory {
    async fn module_for(&self, sender: &BusName) -> Option<String> {
        for (name, module_id) in &self.peers {
            if let Ok(Some(owner)) = self.connection.name_owner(*name).await {
                if &owner == sender {
                    return Some((*module_id).to_string());
                }
            }
        }
        None
    }
}

struct Env(Arc<dyn StorageBackend>);

#[async_trait]
impl Environment for Env {
    fn backend(&self) -> Option<Arc<dyn StorageBackend>> {
        Some(Arc::clone(&self.0))
    }
    fn module_scope(&self, module_id: &str) -> Result<Scope, StorageError> {
        module_scope(None, &Scope::local(), module_id)
    }
    fn grant(&self, module_id: &str) -> Option<StorageGrant> {
        declared(module_id)
    }
    fn keys(&self) -> Result<Arc<dyn KeyProvider>, StorageError> {
        Ok(Arc::new(DerivedKeys::new(Zeroizing::new([9u8; 32]))))
    }
    async fn admit_write(&self, _scope: &Scope) -> Result<(), StorageError> {
        Ok(())
    }
}

#[derive(Default)]
struct Trail(Mutex<Vec<AuditRecord>>);

impl AuditSink for Trail {
    fn record(&self, record: &AuditRecord) {
        self.0.lock().unwrap().push(record.clone());
    }
}

struct Rig {
    backend: Arc<dyn StorageBackend>,
    trail: Arc<Trail>,
    /// A peer that is not the probe: the host must not know it.
    stranger: Connection,
    /// Extra peers standing for registry modules, with the host-side bus name
    /// of the storage interface's owner they talk to.
    peers: Vec<Connection>,
    host: BusName,
    probe: Proxy,
    _modules: ModuleHost,
    _staged: tempfile::TempDir,
}

/// The real probe, loaded, standing for registry module `module_id`.
async fn rig(module_id: &'static str) -> Rig {
    rig_with_peers(module_id, &[]).await
}

/// The bus name of the `n`th extra peer.
fn peer_name(n: usize) -> String {
    format!("ai.tinyhumans.test.Peer{n}")
}

/// [`rig`], plus one extra in-process peer per entry of `others`, each
/// standing for that registry module under the bus name [`peer_name`].
async fn rig_with_peers(module_id: &'static str, others: &[&'static str]) -> Rig {
    let transport = MemoryBus::new();
    let broker = Broker::new();
    broker.spawn(transport.clone());
    let modules = ModuleHost::new(broker);
    let host_connection = Connection::connect(transport.connect().await.unwrap())
        .await
        .unwrap();
    let stranger = Connection::connect(transport.connect().await.unwrap())
        .await
        .unwrap();
    let backend: Arc<dyn StorageBackend> =
        Arc::new(crate::modules::storage_host_watching::WatchingStorage::new());
    let trail = Arc::new(Trail::default());
    let mut peers = Vec::new();
    let mut named = vec![(PROBE, module_id)];
    for (n, other) in others.iter().enumerate() {
        let connection = Connection::connect(transport.connect().await.unwrap())
            .await
            .unwrap();
        let name: &'static str = Box::leak(peer_name(n).into_boxed_str());
        connection.request_name(name).await.unwrap();
        named.push((name, *other));
        peers.push(connection);
    }
    let host = Arc::new(StorageHost::new(
        Arc::new(ProbeDirectory {
            connection: host_connection.clone(),
            peers: named,
        }),
        Arc::new(Env(Arc::clone(&backend))),
        trail.clone(),
    ));
    attach(&modules, &host_connection, host).await.unwrap();
    let staged = tempfile::tempdir().unwrap();
    let artifact = stage(staged.path());
    modules.load_file(&artifact).expect("load the probe cdylib");
    // `load_file` can return before the module's setup claims its name.
    let probe_name = BusName::new(PROBE).expect("probe bus name");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !matches!(stranger.name_owner(&probe_name).await, Ok(Some(_))) {
        assert!(
            std::time::Instant::now() < deadline,
            "the probe module never claimed {PROBE}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let probe = stranger.proxy(PROBE, "/probe", PROBE).unwrap();
    let host = stranger
        .name_owner(STORAGE_INTERFACE)
        .await
        .unwrap()
        .expect("the storage host owns its name");
    Rig {
        backend,
        trail,
        stranger,
        peers,
        host,
        probe,
        _modules: modules,
        _staged: staged,
    }
}

impl Rig {
    /// One raw storage call made by the loaded module.
    async fn call(&self, object: &str, selector: &str, request: Value) -> Value {
        self.probe
            .call("Call", (object, selector, request, false))
            .await
            .expect("the probe answers")
    }

    fn scope(&self, module_id: &str) -> Scope {
        module_scope(None, &Scope::local(), module_id).unwrap()
    }
}

fn denied(answer: &Value, label: &str) -> bool {
    answer["err"]["name"] == "ai.tinyhumans.tinystorage.Error.Denied"
        && answer["err"]["message"] == label
}

fn put(collection: &str, id: &str, document: &Value) -> Value {
    json!({
        "collection": collection,
        "document_id": id,
        "document_json": document.to_string(),
        "precondition": {"kind": "none"},
    })
}

fn get(collection: &str, id: &str) -> Value {
    json!({"collection": collection, "document_id": id})
}

#[tokio::test(flavor = "multi_thread")]
async fn a_loaded_module_reads_and_writes_its_own_granted_collection() {
    let _turn = TURN.lock().await;
    let rig = rig("tinymcp").await;
    let stored = rig
        .call(
            "Documents",
            "Put",
            put("mcp_servers", "files", &json!({"qualified_name": "files"})),
        )
        .await;
    assert!(stored.get("ok").is_some(), "{stored}");
    let read = rig
        .call("Documents", "Get", get("mcp_servers", "files"))
        .await;
    assert!(read.get("ok").is_some(), "{read}");
    assert!(read.to_string().contains("qualified_name"), "{read}");
    // It landed in the module's own scope, in the backend the host opened.
    let own = rig.backend.for_scope(&rig.scope("tinymcp")).unwrap();
    assert!(own
        .documents()
        .get("mcp_servers", "files")
        .await
        .unwrap()
        .is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_loaded_module_cannot_reach_another_modules_collections() {
    let _turn = TURN.lock().await;
    let rig = rig("tinymcp").await;
    // Another module's collection, seeded in that module's own scope.
    let theirs = crate::modules::storage_grants::declared("tinychannels")
        .expect("tinychannels declares storage")
        .collections[0]
        .name
        .clone();
    let victim = rig.backend.for_scope(&rig.scope("tinychannels")).unwrap();
    victim
        .documents()
        .ensure_collection(
            &declared("tinychannels")
                .unwrap()
                .collections
                .into_iter()
                .next()
                .unwrap(),
        )
        .await
        .unwrap();
    victim
        .documents()
        .put(
            &theirs,
            "session",
            json!({"marker": "OTHER-MODULES-SECRET"}),
            Default::default(),
        )
        .await
        .unwrap();

    for (selector, request) in [
        ("Get", get(&theirs, "session")),
        (
            "Put",
            put(&theirs, "session", &json!({"marker": "overwritten"})),
        ),
        (
            "Query",
            json!({"collection": theirs, "filter": null, "limit": 10}),
        ),
        (
            "AtomicBatch",
            json!({"ops": [{"kind": "delete", "collection": theirs, "document_id": "session"}]}),
        ),
    ] {
        let answer = rig.call("Documents", selector, request).await;
        assert!(
            denied(&answer, "target-not-granted"),
            "{selector}: {answer}"
        );
        assert!(!answer.to_string().contains("OTHER-MODULES-SECRET"));
    }
    // Untouched: the refusal came before the backend.
    let after = victim.documents().get(&theirs, "session").await.unwrap();
    assert!(after
        .expect("the other module's document survives")
        .doc
        .to_string()
        .contains("OTHER-MODULES-SECRET"));
    // And its own scope never got the collection.
    let own = rig.backend.for_scope(&rig.scope("tinymcp")).unwrap();
    assert!(!own
        .documents()
        .collections("")
        .await
        .unwrap()
        .contains(&theirs));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_loaded_module_cannot_name_or_reach_another_scope() {
    let _turn = TURN.lock().await;
    let rig = rig("tinymcp").await;
    // The same collection and id in the host's default scope and in another
    // tenant's: neither may be visible to the module.
    for scope in [Scope::local(), Scope::new("someone-else").unwrap()] {
        let handle = rig.backend.for_scope(&scope).unwrap();
        let spec = declared("tinymcp")
            .unwrap()
            .collections
            .into_iter()
            .next()
            .unwrap();
        handle.documents().ensure_collection(&spec).await.unwrap();
        handle
            .documents()
            .put(
                "mcp_servers",
                "victim",
                json!({"marker": "OTHER-SCOPE-SECRET"}),
                Default::default(),
            )
            .await
            .unwrap();
    }
    // Same id, own scope: not found, not the other scope's document.
    let read = rig
        .call("Documents", "Get", get("mcp_servers", "victim"))
        .await;
    assert!(read["ok"].is_null(), "{read}");
    assert!(!read.to_string().contains("OTHER-SCOPE-SECRET"), "{read}");
    // A scope smuggled into the request is not a selector the wire knows.
    for field in ["scope", "agent", "module", "profile"] {
        let mut request = get("mcp_servers", "victim");
        request[field] = json!("local");
        let answer = rig.call("Documents", "Get", request).await;
        assert!(answer.get("err").is_some(), "{field}: {answer}");
        assert!(!answer.to_string().contains("OTHER-SCOPE-SECRET"));
    }
    // A write lands only in the module's own scope.
    let wrote = rig
        .call(
            "Documents",
            "Put",
            put("mcp_servers", "mine", &json!({"qualified_name": "mine"})),
        )
        .await;
    assert!(wrote.get("ok").is_some(), "{wrote}");
    for scope in [Scope::local(), Scope::new("someone-else").unwrap()] {
        let handle = rig.backend.for_scope(&scope).unwrap();
        assert!(handle
            .documents()
            .get("mcp_servers", "mine")
            .await
            .unwrap()
            .is_none());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_loaded_module_cannot_use_a_collection_it_never_declared() {
    let _turn = TURN.lock().await;
    let rig = rig("tinymcp").await;
    for selector in ["Get", "Query", "Count"] {
        let request = match selector {
            "Get" => get("never_declared", "x"),
            _ => json!({"collection": "never_declared"}),
        };
        let answer = rig.call("Documents", selector, request).await;
        assert!(
            denied(&answer, "target-not-granted"),
            "{selector}: {answer}"
        );
    }
    let wrote = rig
        .call(
            "Documents",
            "Put",
            put("never_declared", "x", &json!({"a": 1})),
        )
        .await;
    assert!(denied(&wrote, "target-not-granted"), "{wrote}");
    // Schema is the host's alone: a module cannot create the collection, nor
    // change a declared one, nor drop one.
    let created = rig
        .call(
            "Documents",
            "EnsureCollection",
            json!({"spec": {"name": "never_declared"}}),
        )
        .await;
    assert!(denied(&created, "schema-is-host-only"), "{created}");
    let dropped = rig
        .call(
            "Documents",
            "DropCollection",
            json!({"collection": "mcp_servers"}),
        )
        .await;
    assert!(denied(&dropped, "schema-is-host-only"), "{dropped}");
    let own = rig.backend.for_scope(&rig.scope("tinymcp")).unwrap();
    let listed = own.documents().collections("").await.unwrap();
    assert!(
        !listed.contains(&"never_declared".to_string()),
        "{listed:?}"
    );
    // The module's listing shows its own names only.
    let seen = rig.call("Documents", "Collections", json!({})).await;
    assert!(!seen.to_string().contains("never_declared"), "{seen}");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_undeclared_object_and_prefix_are_refused() {
    let _turn = TURN.lock().await;
    let rig = rig("tinymcp").await;
    // tinymcp declares no blobs at all.
    let blob = rig
        .call("Blobs", "Get", json!({"key": "anything/at/all"}))
        .await;
    assert!(denied(&blob, "object-not-granted"), "{blob}");
    // A stream outside its audit prefix.
    let stream = rig
        .call("Streams", "Len", json!({"stream": "someone_elses_stream"}))
        .await;
    assert!(denied(&stream, "target-not-granted"), "{stream}");
    // A secret outside its prefixes, on the confidential route.
    let secret: Value = rig
        .probe
        .call(
            "Call",
            ("Secrets", "Get", json!({"name": "someone_elses/key"}), true),
        )
        .await
        .unwrap();
    assert!(denied(&secret, "target-not-granted"), "{secret}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_peer_that_is_not_a_registry_module_is_refused() {
    let _turn = TURN.lock().await;
    let rig = rig("tinymcp").await;
    let proxy = rig
        .stranger
        .proxy(
            STORAGE_INTERFACE,
            "/ai/tinyhumans/tinystorage/Documents",
            STORAGE_INTERFACE,
        )
        .unwrap();
    let refused: tinybus::Result<Value> = proxy.call("Get", (get("mcp_servers", "files"),)).await;
    match refused {
        Err(tinybus::Error::MethodFailed { name, message }) => {
            assert_eq!(name, "ai.tinyhumans.tinystorage.Error.Denied");
            assert_eq!(message, "unknown-caller");
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_blob_over_the_inline_cap_is_refused_and_stores_nothing() {
    let _turn = TURN.lock().await;
    let rig = rig("tinyjuice").await;
    let small: Value = rig
        .probe
        .call("PutBlob", ("tinyjuice_ccr/small", 1024usize))
        .await
        .unwrap();
    assert!(
        small.get("ok").is_some(),
        "a small inline blob works: {small}"
    );
    // Past the contract's inline cap (1 MiB) the server refuses the body and
    // stores nothing; a body that size or bigger goes by `PutStream`, which
    // the tests below cover. (A body past the 16 MiB frame is covered by
    // `an_inline_blob_no_frame_can_carry_fails_fast_instead_of_hanging`.)
    let big: Value = rig
        .probe
        .call("PutBlob", ("tinyjuice_ccr/over-cap", 2usize * 1024 * 1024))
        .await
        .unwrap();
    assert!(
        big.get("err").is_some(),
        "an oversized inline blob must fail: {big}"
    );
    let handle = rig.backend.for_scope(&rig.scope("tinyjuice")).unwrap();
    assert!(handle
        .blobs()
        .head("tinyjuice_ccr/over-cap")
        .await
        .unwrap()
        .is_none());
    // The refusal does not take the module's connection down with it.
    let after: Value = rig
        .probe
        .call("PutBlob", ("tinyjuice_ccr/after", 2048usize))
        .await
        .unwrap();
    assert!(
        after.get("ok").is_some(),
        "the module keeps working: {after}"
    );
    // A blob key outside the CCR prefix is not the module's.
    let outside = rig
        .call("Blobs", "Get", json!({"key": "tinyjuice_other/x"}))
        .await;
    assert!(denied(&outside, "target-not-granted"), "{outside}");
}

const MIB: u64 = 1024 * 1024;

impl Rig {
    /// One of the probe's own members.
    async fn member(&self, member: &str, args: impl serde::Serialize) -> Value {
        self.probe
            .call(member, args)
            .await
            .expect("the probe answers")
    }

    fn peer_proxy(&self, n: usize, object: &str) -> Proxy {
        self.peers[n]
            .proxy(
                STORAGE_INTERFACE,
                format!("/ai/tinyhumans/tinystorage/{object}"),
                STORAGE_INTERFACE,
            )
            .unwrap()
            .with_timeout(std::time::Duration::from_secs(10))
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_blob_over_the_bus_frame_round_trips_through_a_peer_bound_stream() {
    let _turn = TURN.lock().await;
    let rig = rig("tinyjuice").await;
    // 20 MiB: over the 16 MiB frame no inline call could carry.
    let len = 20 * MIB;
    let stored = rig
        .member("StreamPut", ("tinyjuice_ccr/streamed", len))
        .await;
    assert_eq!(stored["ok"]["len"], len, "{stored}");
    // It is in the module's own scope, byte for byte.
    let handle = rig.backend.for_scope(&rig.scope("tinyjuice")).unwrap();
    let meta = handle
        .blobs()
        .head("tinyjuice_ccr/streamed")
        .await
        .unwrap()
        .expect("the streamed blob is stored");
    assert_eq!(meta.len, len);
    let bytes = handle
        .blobs()
        .get_range("tinyjuice_ccr/streamed", 0..1024)
        .await
        .unwrap()
        .expect("range");
    assert!(bytes.iter().all(|byte| *byte == 0xAB));
    // And it comes back whole by `GetStream`.
    let read = rig.member("StreamGet", ("tinyjuice_ccr/streamed",)).await;
    assert_eq!(read["ok"]["len"], len, "{read}");
    assert_eq!(read["ok"]["meta_len"], len, "{read}");
    assert_eq!(read["ok"]["all_fill"], true, "{read}");
    // A key that is not there is `null`, not an error.
    let missing = rig.member("StreamGet", ("tinyjuice_ccr/absent",)).await;
    assert!(missing["ok"].is_null() && missing.get("ok").is_some(), "{missing}");
    let trail = rig.trail.0.lock().unwrap().clone();
    assert!(trail
        .iter()
        .any(|r| r.op == "PutStream" && r.outcome == "ok" && r.module == "tinyjuice"));
    assert!(trail
        .iter()
        .any(|r| r.op == "GetStream" && r.outcome == "ok" && r.module == "tinyjuice"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_streamed_blob_is_still_bound_by_the_grant_and_the_length() {
    let _turn = TURN.lock().await;
    let rig = rig("tinyjuice").await;
    // Outside the CCR prefix: refused before anything is stored.
    let outside = rig.member("StreamPut", ("tinyjuice_other/x", 2 * MIB)).await;
    assert!(outside.get("err").is_some(), "{outside}");
    let handle = rig.backend.for_scope(&rig.scope("tinyjuice")).unwrap();
    assert!(handle.blobs().head("tinyjuice_other/x").await.unwrap().is_none());
    let denied_get = rig.member("StreamGet", ("tinyjuice_other/x",)).await;
    assert!(
        denied_get["err"].to_string().contains("target-not-granted"),
        "{denied_get}"
    );
    // The module keeps working after a refusal.
    let after = rig.member("StreamPut", ("tinyjuice_ccr/after", 3 * MIB)).await;
    assert_eq!(after["ok"]["len"], 3 * MIB, "{after}");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_inline_blob_no_frame_can_carry_fails_fast_instead_of_hanging() {
    let _turn = TURN.lock().await;
    let rig = rig("tinyjuice").await;
    // 5 000 000 bytes as a JSON array of numbers (`255,` each) is a 20 MB
    // frame, over the 16 MiB cap.
    let started = std::time::Instant::now();
    let big: Value = rig
        .member("PutBlob", ("tinyjuice_ccr/over-frame", 5_000_000usize))
        .await;
    assert!(big.get("err").is_some(), "an oversized frame must fail: {big}");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(60),
        "took {:?}",
        started.elapsed()
    );
    let handle = rig.backend.for_scope(&rig.scope("tinyjuice")).unwrap();
    assert!(handle
        .blobs()
        .head("tinyjuice_ccr/over-frame")
        .await
        .unwrap()
        .is_none());
    let after: Value = rig.member("PutBlob", ("tinyjuice_ccr/after", 2048usize)).await;
    assert!(after.get("ok").is_some(), "the module keeps working: {after}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_watch_signal_reaches_the_module_that_opened_the_watch() {
    let _turn = TURN.lock().await;
    let rig = rig("tinymcp").await;
    let seen = rig.member("WatchPut", ("mcp_servers", "watched")).await;
    assert_eq!(seen["ok"], "invalidated", "{seen}");
    // A collection the grant never declared cannot be watched.
    let refused = rig
        .call(
            "Documents",
            "WatchOpen",
            json!({"collection": "never_declared", "filter": {"op": "all"}, "interval_ms": 50}),
        )
        .await;
    assert!(denied(&refused, "target-not-granted"), "{refused}");
    let trail = rig.trail.0.lock().unwrap().clone();
    assert!(trail
        .iter()
        .any(|r| r.op == "WatchOpen" && r.outcome == "ok" && r.module == "tinymcp"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_second_module_cannot_receive_the_first_modules_watch_signals() {
    let _turn = TURN.lock().await;
    // The probe stands for tinymcp; peer 0 stands for tinychannels, which also
    // holds documents.
    let rig = rig_with_peers("tinymcp", &["tinychannels"]).await;
    let theirs = declared("tinychannels").unwrap().collections[0].name.clone();
    // The second module listens to every watch signal the host sends anyone
    // (any path), and opens a watch of its own as a positive control.
    let mut everything = rig.peers[0]
        .add_match(
            tinybus::MatchRule::new()
                .signals()
                .interface(tinybus::InterfaceName::new(STORAGE_INTERFACE).unwrap()),
        )
        .await
        .unwrap();
    let documents = rig.peer_proxy(0, "Documents");
    let own: Value = documents
        .call(
            "WatchOpen",
            (json!({"collection": theirs, "filter": {"op": "all"}, "interval_ms": 50}),),
        )
        .await
        .unwrap();
    let own_id = own["watch_id"].as_str().unwrap().to_string();
    // The first module opens a watch and changes its document.
    let first = rig.member("WatchPut", ("mcp_servers", "w1")).await;
    assert_eq!(first["ok"], "invalidated", "{first}");
    // The second module's own change reaches the second module.
    let _: Value = documents
        .call(
            "Put",
            (put(&theirs, "d", &json!({"v": 1})),),
        )
        .await
        .unwrap();
    let mut ids = Vec::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while std::time::Instant::now() < deadline && !ids.contains(&own_id) {
        if let Ok(Ok(message)) =
            tokio::time::timeout(std::time::Duration::from_millis(200), everything.recv()).await
        {
            ids.push(message.body[0]["watch_id"].as_str().unwrap().to_string());
        }
    }
    assert!(ids.contains(&own_id), "its own signal never arrived: {ids:?}");
    // Nothing else did: the first module's watch id is not among them.
    assert!(
        ids.iter().all(|id| *id == own_id),
        "a signal for another module's watch leaked: {ids:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_second_module_cannot_read_or_take_the_first_modules_streams() {
    let _turn = TURN.lock().await;
    // The probe and peer 0 both stand for tinyjuice (same grant, same scope),
    // so only the peer binding keeps the second out; peer 1 stands for tinymcp,
    // which has no blobs at all.
    let rig = rig_with_peers("tinyjuice", &["tinyjuice", "tinymcp"]).await;
    let stored = rig
        .member("StreamPut", ("tinyjuice_ccr/private", 2 * MIB))
        .await;
    assert_eq!(stored["ok"]["len"], 2 * MIB, "{stored}");

    // A module without blob rights is refused outright.
    let nosy: tinybus::Result<Value> = rig
        .peer_proxy(1, "Blobs")
        .call("GetStream", (json!({"key": "tinyjuice_ccr/private"}),))
        .await;
    match nosy {
        Err(tinybus::Error::MethodFailed { name, message }) => {
            assert_eq!(name, "ai.tinyhumans.tinystorage.Error.Denied");
            assert_eq!(message, "object-not-granted");
        }
        other => panic!("expected a refusal, got {other:?}"),
    }

    // Taking a stream the probe opened toward the host: the peer is granted
    // the key, but the host reads an upload only from the peer that opened it.
    let opened = rig.member("OpenStream", (MIB,)).await;
    let reference: tinybus::StreamRef = serde_json::from_value(opened["ok"].clone())
        .unwrap_or_else(|_| panic!("a stream reference: {opened}"));
    let hijack: tinybus::Result<Value> = rig
        .peer_proxy(0, "Blobs")
        .call(
            "PutStream",
            (json!({"key": "tinyjuice_ccr/hijacked", "len": MIB}), reference),
        )
        .await;
    assert!(hijack.is_err(), "a peer took another peer's stream: {hijack:?}");
    let handle = rig.backend.for_scope(&rig.scope("tinyjuice")).unwrap();
    assert!(handle
        .blobs()
        .head("tinyjuice_ccr/hijacked")
        .await
        .unwrap()
        .is_none());
    // The refused attempt did not spend the stream: its owner still uses it.
    let used = rig.member("UseStream", ("tinyjuice_ccr/owned", MIB)).await;
    assert!(used.get("ok").is_some(), "the owner lost its stream: {used}");
    assert_eq!(
        handle
            .blobs()
            .head("tinyjuice_ccr/owned")
            .await
            .unwrap()
            .expect("stored by the owner")
            .len,
        MIB
    );

    // A download the host opened toward the probe is not readable by a peer.
    let reply = rig
        .member("RawGetStream", ("tinyjuice_ccr/private",))
        .await;
    let download: tinybus::StreamRef = serde_json::from_value(reply["ok"][1].clone())
        .unwrap_or_else(|_| panic!("a download reference: {reply}"));
    assert!(
        rig.peers[0]
            .accept_stream_from(&download, &rig.host)
            .is_err(),
        "a peer accepted a stream opened toward another peer"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn every_call_is_audited_under_the_module_id_without_a_payload() {
    let _turn = TURN.lock().await;
    let rig = rig("tinymcp").await;
    let _ = rig
        .call(
            "Documents",
            "Put",
            put(
                "mcp_servers",
                "audited",
                &json!({"marker": "PAYLOAD-MARKER"}),
            ),
        )
        .await;
    let _ = rig
        .call("Documents", "Get", get("never_declared", "x"))
        .await;
    let trail = rig.trail.0.lock().unwrap().clone();
    assert!(trail.iter().all(|record| record.module == "tinymcp"));
    assert!(trail
        .iter()
        .any(|record| record.op == "Put" && record.outcome == "ok"));
    assert!(trail
        .iter()
        .any(|record| record.outcome == "target-not-granted"));
    assert!(!format!("{trail:?}").contains("PAYLOAD-MARKER"));
}
