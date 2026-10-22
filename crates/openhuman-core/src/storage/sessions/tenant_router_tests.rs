use super::*;
use crate::storage::MemoryStorage;
use tinystoragedrivers::{Capabilities, ScopedStorage, StorageError};

fn stores() -> TenantSessionStores {
    TenantSessionStores::with_registry(
        Arc::new(MemoryStorage::new()),
        false,
        tinyagents_harness::store::SchemaAdmission::Declare,
        Arc::new(TenantStoreRegistry::default()),
    )
}

#[test]
fn an_unprofiled_key_routes_to_the_local_tenant() {
    let router = stores();
    let (provider, agent) = router.route("asha");
    let destination = provider.for_agent(&agent).transcripts.destination_key();
    let expected = crate::storage::tenant_scope(None).agent("asha").unwrap();
    assert!(
        destination
            .as_deref()
            .is_some_and(|key| key.contains(expected.as_str())),
        "{destination:?}"
    );
    assert_eq!(agent, "asha");
}

#[test]
fn a_profile_key_routes_to_the_profile_tenant() {
    let router = stores();
    let key = crate::core::runtime::session_key(&crate::core::runtime::Tenant {
        profile: Some("alice".into()),
        agent: Some("asha".into()),
    });
    let (provider, agent) = router.route(&key);
    let destination = provider.for_agent(&agent).transcripts.destination_key();
    let expected = crate::storage::scope_for_profile("alice")
        .agent("asha")
        .unwrap();
    assert!(
        destination
            .as_deref()
            .is_some_and(|key| key.contains(expected.as_str())),
        "{destination:?}"
    );
}

#[test]
fn escaped_keys_round_trip_to_the_same_tenant_and_agent() {
    let router = stores();
    let key = crate::core::runtime::session_key(&crate::core::runtime::Tenant {
        profile: Some("a~b%c".into()),
        agent: None,
    });
    let (provider, agent) = router.route(&key);
    let destination = provider.for_agent(&agent).transcripts.destination_key();
    let expected = crate::storage::scope_for_profile("a~b%c")
        .agent("default")
        .unwrap();
    assert!(
        destination
            .as_deref()
            .is_some_and(|key| key.contains(expected.as_str())),
        "{destination:?}"
    );
    assert_eq!(agent, "default");
}

#[test]
fn one_provider_serves_every_agent_of_a_tenant() {
    let router = stores();
    let (first, _) = router.route("one");
    let (second, _) = router.route("two");
    assert!(Arc::ptr_eq(&first, &second));
    let (other, _) = router.route("u~two");
    assert!(!Arc::ptr_eq(&first, &other));
}

#[test]
fn pinned_session_store_leases_keep_profile_and_agent_routing() {
    let router = stores();
    let alice_key = crate::core::runtime::session_key(&crate::core::runtime::Tenant {
        profile: Some("alice".into()),
        agent: Some("asha".into()),
    });
    let bob_key = crate::core::runtime::session_key(&crate::core::runtime::Tenant {
        profile: Some("bob".into()),
        agent: Some("asha".into()),
    });
    let alice = router.lease_for_agent(&alice_key).unwrap();
    let alice_again = router.lease_for_agent(&alice_key).unwrap();
    let bob = router.lease_for_agent(&bob_key).unwrap();

    assert_eq!(
        alice.stores().transcripts.destination_key(),
        alice_again.stores().transcripts.destination_key()
    );
    assert_ne!(
        alice.stores().transcripts.destination_key(),
        bob.stores().transcripts.destination_key()
    );
}

#[test]
fn upstream_rebind_keeps_tenant_routing_and_moves_only_future_leases() {
    let registry = Arc::new(TenantStoreRegistry::default());
    let first: Arc<dyn StorageBackend> = Arc::new(MemoryStorage::new());
    let second: Arc<dyn StorageBackend> = Arc::new(MemoryStorage::new());
    let router = TenantSessionStores::with_registry(
        Arc::clone(&first),
        false,
        tinyagents_harness::store::SchemaAdmission::Declare,
        Arc::clone(&registry),
    );
    registry.register_backend(&router.backend, &router.tenants);
    let alice_key = crate::core::runtime::session_key(&crate::core::runtime::Tenant {
        profile: Some("alice".into()),
        agent: Some("asha".into()),
    });
    let bob_key = crate::core::runtime::session_key(&crate::core::runtime::Tenant {
        profile: Some("bob".into()),
        agent: Some("asha".into()),
    });

    let alice_before = router.lease_for_agent(&alice_key).unwrap();
    let bob_before = router.lease_for_agent(&bob_key).unwrap();
    let alice_destination = alice_before.stores().transcripts.destination_key();
    let bob_destination = bob_before.stores().transcripts.destination_key();
    assert_ne!(
        alice_destination, bob_destination,
        "profiles keep separate scopes"
    );

    assert_eq!(registry.rebind(&second).unwrap(), 2);
    assert!(
        Arc::ptr_eq(&*router.backend.read().unwrap(), &second),
        "new tenants open on the newly bound workspace backend"
    );
    let alice_after = router.lease_for_agent(&alice_key).unwrap();
    let bob_after = router.lease_for_agent(&bob_key).unwrap();
    let charlie_key = crate::core::runtime::session_key(&crate::core::runtime::Tenant {
        profile: Some("charlie".into()),
        agent: Some("asha".into()),
    });
    let charlie_after = router.lease_for_agent(&charlie_key).unwrap();
    assert_ne!(
        alice_destination,
        alice_after.stores().transcripts.destination_key(),
        "a future lease follows the upstream rebind"
    );
    assert_ne!(
        bob_destination,
        bob_after.stores().transcripts.destination_key(),
        "each tenant provider follows the same backend rebind"
    );
    assert_eq!(
        alice_destination,
        alice_before.stores().transcripts.destination_key(),
        "an already-issued lease remains pinned"
    );
    assert!(
        charlie_after
            .stores()
            .transcripts
            .destination_key()
            .is_some(),
        "a tenant first seen after rebind still gets a session store"
    );
}

#[test]
fn failed_rebind_rolls_back_tenant_providers_already_moved() {
    let registry = Arc::new(TenantStoreRegistry::default());
    let first: Arc<dyn StorageBackend> = Arc::new(MarkerStorage::new("original", false));
    let second: Arc<dyn StorageBackend> = Arc::new(MarkerStorage::new("candidate", true));
    let router = TenantSessionStores::with_registry(
        Arc::clone(&first),
        false,
        tinyagents_harness::store::SchemaAdmission::Declare,
        Arc::clone(&registry),
    );
    registry.register_backend(&router.backend, &router.tenants);
    let alice_key = crate::core::runtime::session_key(&crate::core::runtime::Tenant {
        profile: Some("alice".into()),
        agent: Some("asha".into()),
    });
    let bob_key = crate::core::runtime::session_key(&crate::core::runtime::Tenant {
        profile: Some("bob".into()),
        agent: Some("asha".into()),
    });

    let alice_before = router.for_agent(&alice_key);
    let bob_before = router.for_agent(&bob_key);
    assert!(alice_before
        .transcripts
        .destination_key()
        .is_some_and(|key| key.starts_with("original://")));
    assert!(bob_before
        .transcripts
        .destination_key()
        .is_some_and(|key| key.starts_with("original://")));

    assert!(registry.rebind(&second).is_err());
    assert!(Arc::ptr_eq(&*router.backend.read().unwrap(), &first));

    assert!(
        router
            .for_agent(&alice_key)
            .transcripts
            .destination_key()
            .is_some_and(|key| key.starts_with("original://")),
        "the earlier provider must return to its original backend when a later tenant fails"
    );
    assert!(
        router
            .for_agent(&bob_key)
            .transcripts
            .destination_key()
            .is_some_and(|key| key.starts_with("original://")),
        "the failing provider remains on its original backend"
    );
}

#[derive(Debug)]
struct MarkerStorage {
    inner: MemoryStorage,
    driver: &'static str,
    reject_bob: bool,
}

impl MarkerStorage {
    fn new(driver: &'static str, reject_bob: bool) -> Self {
        Self {
            inner: MemoryStorage::new(),
            driver,
            reject_bob,
        }
    }
}

impl StorageBackend for MarkerStorage {
    fn driver(&self) -> &'static str {
        self.driver
    }

    fn capabilities(&self) -> Capabilities {
        self.inner.capabilities()
    }

    fn for_scope(&self, scope: &Scope) -> Result<ScopedStorage, StorageError> {
        if self.reject_bob && scope.as_str().starts_with("u:bob/") {
            return Err(StorageError::invalid_input("injected Bob tenant failure"));
        }
        let scoped = self.inner.for_scope(scope)?;
        Ok(ScopedStorage::new(
            scope.clone(),
            self.driver,
            Arc::clone(scoped.documents()),
            Arc::clone(scoped.streams()),
            Arc::clone(scoped.blobs()),
        ))
    }
}
