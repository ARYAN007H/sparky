use super::*;

#[tokio::test]
async fn turn_factory_checks_privacy_then_explicit_host_session_before_building_or_egress() {
    let _guard = crate::inference::inference_test_guard_async().await;
    let _env = crate::config::TEST_ENV_LOCK.lock().await;
    let tmp = tempfile::tempdir().expect("isolated workspace");
    let builds = install_test_managed_provider();
    let mut config = Config::default();
    config.workspace_dir = tmp.path().join("workspace");
    config.config_path = tmp.path().join("config.toml");
    let config = config;

    let ambient_library = crate::core::runtime::CoreContext::for_test_with_host_kind(
        crate::core::types::HostKind::Library,
        crate::core::runtime::DomainSet::full(),
        Some(tmp.path().join("library")),
    );
    let explicit_cli = crate::core::runtime::CoreContext::for_test_with_host_kind(
        crate::core::types::HostKind::Cli,
        crate::core::runtime::DomainSet::full(),
        Some(tmp.path().join("cli")),
    );
    let ctx = crate::core::runtime::DispatchCtx::new(explicit_cli);
    let egress = EgressLedger::per_turn();
    let factory = OpenHumanModelFactory {
        ctx: &ctx,
        config: &config,
        primary_provider: Some(PROVIDER_OPENHUMAN),
        egress: &egress,
        thread_id: None,
    };

    crate::core::runtime::CoreContext::scope(ambient_library, async {
        let _privacy = crate::security::live_policy::test_privacy_scope(
            crate::config::PrivacyMode::LocalOnly,
        );
        let privacy_result = tinyagents_harness::host::TurnModelSet::<()>::builder(
            &factory,
            &factory,
            "chat",
        )
        .primary_model_pin("hint:chat")
        .without_workload_tiers()
        .build()
        ;
        let privacy_error = match privacy_result {
            Err(error) => error,
            Ok(_) => panic!("LocalOnly must reject managed inference before session lookup"),
        };
        assert!(privacy_error.to_string().contains("Local-only privacy mode"));
    })
    .await;

    let session_result = tinyagents_harness::host::TurnModelSet::<()>::builder(
        &factory,
        &factory,
        "chat",
    )
    .primary_model_pin("hint:chat")
    .without_workload_tiers()
    .build()
    ;
    let session_error = match session_result {
        Err(error) => error,
        Ok(_) => panic!("explicit CLI dispatch must require a session despite ambient Library"),
    };
    assert!(session_error.to_string().contains("session"));
    assert!(builds.lock().expect("build counter").is_empty());
}
