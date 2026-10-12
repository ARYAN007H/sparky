use super::*;
use crate::core::runtime::{ContextOverlay, CoreContext, DomainSet};
use crate::tools::toolpacks::ToolGroups;
use std::sync::Arc;

fn config(dir: &std::path::Path) -> Config {
    Config {
        workspace_dir: dir.join("workspace"),
        config_path: dir.join("config.toml"),
        ..Config::default()
    }
}
fn profile(config: &Config, id: &str) -> Arc<CoreContext> {
    CoreContext::for_test_with_config(DomainSet::kernel(), config.clone()).derive_with(
        ContextOverlay::new(config.clone(), DomainSet::kernel(), ToolGroups::none()).profile(id),
    )
}
fn seed(config: &Config, ctx: Arc<CoreContext>, toolkit: &str) -> String {
    CoreContext::sync_scope(ctx, || {
        let key = cache_key(config);
        INTEGRATIONS_CACHE.write().unwrap().insert(
            key.clone(),
            CachedIntegrations {
                entries: vec![ConnectedIntegration {
                    toolkit: toolkit.into(),
                    description: String::new(),
                    tools: vec![],
                    gated_tools: vec![],
                    connected: true,
                    connections: vec![],
                    non_active_status: None,
                }],
                cached_at: Instant::now(),
            },
        );
        key
    })
}

#[test]
fn profiles_with_identical_credentials_cannot_read_each_others_integrations() {
    let _guard = composio_cache_test_lock();
    let dir = tempfile::tempdir().unwrap();
    let config = config(dir.path());
    let alice = profile(&config, "isolation-alice");
    let bob = profile(&config, "isolation-bob");
    let key = seed(&config, alice.clone(), "gmail");
    CoreContext::sync_scope(bob.clone(), || {
        assert!(
            cached_active_integrations(&config).is_none(),
            "identical service credentials must not grant another profile's cached connections"
        );
    });
    let bob_key = seed(&config, bob, "slack");
    assert_ne!(
        key, bob_key,
        "profile identity must participate in the cache key"
    );
    CoreContext::sync_scope(alice, || {
        assert_eq!(
            cached_active_integrations(&config).unwrap()[0].toolkit,
            "gmail"
        );
    });
    let mut guard = INTEGRATIONS_CACHE.write().unwrap();
    guard.remove(&key);
    guard.remove(&bob_key);
}

#[test]
fn reconciliation_for_one_profile_preserves_another_profiles_connections() {
    let _guard = composio_cache_test_lock();
    let dir = tempfile::tempdir().unwrap();
    let config = config(dir.path());
    let alice = profile(&config, "reconciliation-alice");
    let bob = profile(&config, "reconciliation-bob");
    let alice_key = seed(&config, alice.clone(), "gmail");
    let bob_key = seed(&config, bob.clone(), "slack");
    assert!(CoreContext::sync_scope(
        bob,
        || sync_cache_with_connections(&[])
    ));
    CoreContext::sync_scope(alice, || {
        assert_eq!(
            cached_active_integrations(&config)
                .expect("another profile's connection list must preserve this snapshot")[0]
                .toolkit,
            "gmail"
        );
    });
    let mut guard = INTEGRATIONS_CACHE.write().unwrap();
    assert!(!guard.contains_key(&bob_key));
    guard.remove(&alice_key);
}
#[test]
fn profile_invalidation_preserves_another_profiles_snapshot() {
    let _guard = composio_cache_test_lock();
    let dir = tempfile::tempdir().unwrap();
    let config = config(dir.path());
    let alice = profile(&config, "invalidation-alice");
    let bob = profile(&config, "invalidation-bob");
    let alice_key = seed(&config, alice.clone(), "gmail");
    let bob_key = seed(&config, bob.clone(), "slack");
    CoreContext::sync_scope(bob, invalidate_connected_integrations_cache);
    assert!(
        INTEGRATIONS_CACHE.read().unwrap().contains_key(&alice_key),
        "Bob's invalidation must preserve Alice's snapshot"
    );
    assert!(!INTEGRATIONS_CACHE.read().unwrap().contains_key(&bob_key));
    CoreContext::sync_scope(alice, || {
        assert_eq!(
            cached_active_integrations(&config).unwrap()[0].toolkit,
            "gmail"
        )
    });
    INTEGRATIONS_CACHE.write().unwrap().remove(&alice_key);
}
