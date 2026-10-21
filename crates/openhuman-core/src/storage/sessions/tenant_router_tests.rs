use super::*;
use crate::storage::test_support::memory_backend;

fn stores() -> TenantSessionStores {
    TenantSessionStores::new(memory_backend(), false).expect("bridge")
}

#[test]
fn an_unprofiled_key_routes_to_the_local_tenant() {
    let router = stores();
    let (provider, agent) = router.route("asha");
    assert_eq!(agent, "asha");
    let (same_provider, same_agent) = router.route("asha");
    assert!(Arc::ptr_eq(&provider, &same_provider));
    assert_eq!(same_agent, "asha");
    let (profile_provider, _) = router.route("u~asha");
    assert!(!Arc::ptr_eq(&provider, &profile_provider));
}

#[test]
fn a_profile_key_routes_to_the_profile_tenant() {
    let router = stores();
    let key = crate::core::runtime::session_key(&crate::core::runtime::Tenant {
        profile: Some("alice".into()),
        agent: Some("asha".into()),
    });
    let (provider, agent) = router.route(&key);
    assert_eq!(agent, "asha");
    let (same_provider, same_agent) = router.route(&key);
    assert!(Arc::ptr_eq(&provider, &same_provider));
    assert_eq!(same_agent, "asha");
}

#[test]
fn escaped_keys_round_trip_to_the_same_tenant_and_agent() {
    let router = stores();
    let key = crate::core::runtime::session_key(&crate::core::runtime::Tenant {
        profile: Some("a~b%c".into()),
        agent: None,
    });
    let (provider, agent) = router.route(&key);
    assert_eq!(agent, "default");
    let (same_provider, _) = router.route(&key);
    assert!(Arc::ptr_eq(&provider, &same_provider));
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
