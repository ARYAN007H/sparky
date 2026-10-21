use super::*;
#[cfg(feature = "storage-sqlite")]
use crate::storage::test_support::sqlite_backend;
#[cfg(feature = "storage-sqlite")]
use std::sync::Arc;

#[cfg(feature = "storage-sqlite")]
#[tokio::test]
async fn the_operators_cron_limits_bind_every_profile_whatever_its_config_says() {
    let tmp = tempfile::tempdir().unwrap();
    let fixture = sqlite_backend().unwrap();
    let mut saas = SaasConfig::new(tmp.path());
    saas.cron.agent_jobs_only = true;
    saas.cron.min_interval_secs = 900;
    saas.cron.max_active_jobs = 3;
    let host = ProfileHost::with_backend(
        saas,
        CoreContext::for_test(DomainSet::full(), None),
        Some(Arc::clone(&fixture.backend)),
    )
    .unwrap();
    let id = profile("alice");
    host.provision(&id).await.unwrap();
    // The user's own file tries to switch the mode off.
    let layout = ProfileLayout::new(tmp.path(), &id);
    std::fs::write(
        &layout.config_path,
        "[cron]\nagent_jobs_only = false\nmin_interval_secs = 1\nmax_active_jobs = 999\n",
    )
    .unwrap();
    let forced = host.provisioned_config(&id).await.unwrap();
    let opened = host.open(&id).await.unwrap();
    let loaded =
        crate::config::ops::load_current_or_init(&DispatchCtx::new(Arc::clone(opened.context())))
            .await
            .unwrap();
    for config in [&forced, &opened.config, &loaded] {
        assert!(config.cron.agent_jobs_only);
        assert_eq!(config.cron.min_interval_secs, 900);
        assert_eq!(config.cron.max_active_jobs, 3);
    }
}

#[cfg(feature = "storage-sqlite")]
#[tokio::test]
async fn provisioning_installs_the_cron_cap_in_the_profile_scope() {
    let tmp = tempfile::tempdir().unwrap();
    let fixture = sqlite_backend().unwrap();
    let mut saas = SaasConfig::new(tmp.path());
    saas.cron.agent_jobs_only = true;
    saas.cron.max_active_jobs = 1;
    let host = ProfileHost::with_backend(
        saas,
        CoreContext::for_test(DomainSet::full(), None),
        Some(Arc::clone(&fixture.backend)),
    )
    .unwrap();
    let id = profile("cron-cap-provisioned");
    host.provision(&id).await.unwrap();
    let config = host.provisioned_config(&id).await.unwrap();
    let scope = crate::cron::documents_port::DocScope::for_scope(
        &config,
        crate::storage::scope_for_profile(id.as_str()),
    );
    let operation = || tinyflows_bus::schedule::StoreOperation::AddAgent {
        spec: tinyflows_bus::schedule::AgentJobSpec::new(
            tinyflows_bus::schedule::Schedule::Every { every_ms: 600_000 },
            "profile quota probe",
        ),
    };

    crate::cron::client::store_in::<crate::cron::CronJob>(&config, &scope, operation())
        .await
        .unwrap();
    let second =
        crate::cron::client::store_in::<crate::cron::CronJob>(&config, &scope, operation()).await;
    let error = second.expect_err("the profile store must enforce its provisioned cap");
    assert!(error
        .to_string()
        .contains("Failed to insert cron agent job"));
}

#[cfg(feature = "storage-sqlite")]
#[tokio::test]
async fn opening_an_existing_profile_adopts_the_enforced_cron_cap() {
    let tmp = tempfile::tempdir().unwrap();
    let fixture = sqlite_backend().unwrap();
    let id = profile("cron-cap-adopt-existing");
    let unenforced = ProfileHost::with_backend(
        SaasConfig::new(tmp.path()),
        CoreContext::for_test(DomainSet::full(), None),
        Some(Arc::clone(&fixture.backend)),
    )
    .unwrap();
    unenforced.provision(&id).await.unwrap();

    let mut saas = SaasConfig::new(tmp.path());
    saas.cron.agent_jobs_only = true;
    saas.cron.max_active_jobs = 1;
    let enforced = ProfileHost::with_backend(
        saas,
        CoreContext::for_test(DomainSet::full(), None),
        Some(Arc::clone(&fixture.backend)),
    )
    .unwrap();
    assert!(!enforced.provision(&id).await.unwrap());
    let _profile = enforced.open(&id).await.unwrap();
    let config = enforced.provisioned_config(&id).await.unwrap();
    let scope = crate::cron::documents_port::DocScope::for_scope(
        &config,
        crate::storage::scope_for_profile(id.as_str()),
    );

    crate::cron::client::store_in::<crate::cron::CronJob>(&config, &scope, raw_profile_agent_add())
        .await
        .unwrap();
    let second = crate::cron::client::store_in::<crate::cron::CronJob>(
        &config,
        &scope,
        raw_profile_agent_add(),
    )
    .await;
    let error = second.expect_err("opening a profile should adopt the configured cron cap");
    assert!(error
        .to_string()
        .contains("Failed to insert cron agent job"));
}
