use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use tinymemory_api::conformance::ReferenceEngine;
use tinymemory_api::{
    EngineDescriptor, FetchRequest, ForgetReport, ForgetTarget, LearningKind, ListRequest,
    MemoryEngine, MemoryMeta, RecallRequest, StoreItem, StoreReceipt,
};

use super::*;

fn ns(raw: &str) -> Namespace {
    raw.parse().unwrap()
}

fn user_config(root: &str) -> Config {
    let mut config = Config::default();
    config.memory.root = Some(root.to_string());
    config
}

#[test]
fn only_saas_configs_are_confined() {
    assert!(confinement_in(false, &user_config("user:u-a"))
        .unwrap()
        .is_none());
    assert_eq!(
        confinement_in(true, &user_config("user:u-a")).unwrap(),
        Some(ns("user:u-a"))
    );
}

#[test]
fn a_saas_config_without_a_valid_root_is_refused() {
    // No root, an unparseable one, or the global root: memory refuses rather
    // than reaching every user.
    assert!(confinement_in(true, &Config::default()).is_err());
    assert!(confinement_in(true, &user_config("")).is_err());
    assert!(confinement_in(true, &user_config("not a namespace!")).is_err());
}

#[test]
fn invalid_saas_identity_keeps_the_existing_rpc_error() {
    for config in [
        Config::default(),
        user_config("not a namespace!"),
        user_config("root"),
    ] {
        let error = confinement_in(true, &config).unwrap_err();
        assert!(matches!(
            error,
            Error::InvalidRequest(message)
                if message == "memory is unavailable: this agent has no valid memory root"
        ));
    }
}

#[test]
fn invalid_saas_root_is_refused_before_store_reaches_the_engine() {
    const CHILD: &str = "OPENHUMAN_TEST_SAAS_MEMORY_STORE_GUARD";
    const TEST_NAME: &str = "memory::user_scope::tests::invalid_saas_root_is_refused_before_store_reaches_the_engine";

    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST_NAME, "--nocapture"])
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success()
                && String::from_utf8_lossy(&output.stdout)
                    .contains("SAAS_MEMORY_STORE_GUARD_CHILD_PASSED"),
            "isolated SaaS test process failed or ran zero tests:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }

    crate::core::runtime::mode::lock_mode(crate::core::runtime::Mode::Saas).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    for root in [None, Some("not a namespace!"), Some("root")] {
        let tmp = tempfile::tempdir().unwrap();
        let mut config = crate::memory::test_fixtures::config_in(&tmp);
        config.memory.root = root.map(str::to_string);
        let calls = Arc::new(AtomicUsize::new(0));
        crate::memory::engine::install_test_engine(
            &config.workspace_dir,
            Arc::new(CountingEngine {
                inner: ReferenceEngine::new(),
                calls: calls.clone(),
            }),
        );
        let item = StoreItem::learning(
            "should not be stored",
            LearningKind::Fact,
            0.9,
            MemoryMeta::default(),
        );
        let error = runtime
            .block_on(crate::memory::ops::store_item(&config, item))
            .unwrap_err();

        assert!(matches!(
            error,
            Error::InvalidRequest(message)
                if message == "memory is unavailable: this agent has no valid memory root"
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0, "root: {root:?}");
    }
    println!("SAAS_MEMORY_STORE_GUARD_CHILD_PASSED");
}

struct CountingEngine {
    inner: ReferenceEngine,
    calls: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl MemoryEngine for CountingEngine {
    fn descriptor(&self) -> &EngineDescriptor {
        self.inner.descriptor()
    }

    async fn health(&self) -> tinymemory_api::EngineHealth {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.health().await
    }

    async fn recall(
        &self,
        request: RecallRequest,
    ) -> tinymemory_api::Result<tinymemory_api::RecallAnswer> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.recall(request).await
    }

    async fn fetch(
        &self,
        request: FetchRequest,
    ) -> tinymemory_api::Result<tinymemory_api::FetchPage> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.fetch(request).await
    }

    async fn store(&self, item: StoreItem) -> tinymemory_api::Result<StoreReceipt> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.store(item).await
    }

    async fn forget(
        &self,
        target: ForgetTarget,
    ) -> tinymemory_api::Result<ForgetReport> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.forget(target).await
    }

    async fn list(
        &self,
        request: ListRequest,
    ) -> tinymemory_api::Result<tinymemory_api::ListPage> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.list(request).await
    }
}

#[test]
fn reads_never_climb_out_of_the_root() {
    let root = ns("user:u-a");
    assert_eq!(clamp_reach(None, &root), Reach::subtree(root.clone()));
    assert_eq!(
        clamp_reach(Some(Reach::of(Namespace::ROOT)), &root),
        Reach::subtree(root.clone()),
        "the tree's root reads everyone"
    );
    assert_eq!(
        clamp_reach(Some(Reach::subtree(ns("user:u-b"))), &root),
        Reach::subtree(root.clone()),
        "another user's tree"
    );
    let inside = clamp_reach(Some(Reach::of(ns("user:u-a/agent:u-a"))), &root);
    assert_eq!(inside.at, ns("user:u-a/agent:u-a"));
    assert!(!inside.inherit, "ancestors would include the shared root");
}

#[test]
fn writes_land_inside_the_root() {
    let root = ns("user:u-a");
    let mut meta = MemoryMeta::default();
    meta.namespace = ns("user:u-b");
    let mut item = StoreItem::learning("x", LearningKind::Fact, 0.8, meta);
    clamp_item(&mut item, &root);
    assert_eq!(item.meta().namespace, root);

    let mut meta = MemoryMeta::default();
    meta.namespace = ns("user:u-a/agent:u-a");
    let mut item = StoreItem::learning("x", LearningKind::Fact, 0.8, meta);
    clamp_item(&mut item, &root);
    assert_eq!(item.meta().namespace, ns("user:u-a/agent:u-a"));
}

/// Two users on one shared engine: a clamped filter lists only the caller's
/// items, even when the caller asked for the whole tree.
#[tokio::test]
async fn a_shared_engine_keeps_users_apart_once_clamped() {
    let engine: Arc<dyn MemoryEngine> = Arc::new(ReferenceEngine::new());
    for (user, text) in [("user:u-a", "alice's fact"), ("user:u-b", "bob's fact")] {
        let mut item = StoreItem::learning(text, LearningKind::Fact, 0.8, MemoryMeta::default());
        clamp_item(&mut item, &ns(user));
        engine.store(item).await.unwrap();
    }
    let everyone = MetaFilter {
        reach: Some(Reach::subtree(Namespace::ROOT)),
        ..MetaFilter::default()
    };
    let unclamped = engine
        .list(ListRequest {
            filter: everyone.clone(),
            limit: 100,
            cursor: None,
        })
        .await
        .unwrap();
    assert_eq!(
        unclamped.items.len(),
        2,
        "unclamped, the tree root reads both"
    );

    let clamped = engine
        .list(ListRequest {
            filter: clamp_filter(everyone, &ns("user:u-a")),
            limit: 100,
            cursor: None,
        })
        .await
        .unwrap();
    let texts: Vec<_> = clamped.items.iter().map(|hit| format!("{hit:?}")).collect();
    assert_eq!(clamped.items.len(), 1, "{texts:?}");
    assert!(texts[0].contains("alice"), "{texts:?}");
}
