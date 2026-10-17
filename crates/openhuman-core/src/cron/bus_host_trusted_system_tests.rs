use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::TempDir;
use tinyflows_bus::schedule::{callbacks, Attempt, CALLBACK_INTERFACE};

fn config(tmp: &TempDir) -> Config {
    let config = Config {
        workspace_dir: tmp.path().join("workspace"),
        action_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("config.toml"),
        ..Config::default()
    };
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    config
}

async fn agent_only_host(config: &Config) -> (HostRegistration, tinybus::Proxy) {
    crate::modules::tinyflows::proxy(config)
        .await
        .expect("module attaches");
    let security = Arc::new(crate::security::SecurityPolicy::from_config(
        &config.autonomy,
        &config.workspace_dir,
        &config.action_dir,
    ));
    let binding = Binding::new(&DispatchCtx::ambient(), config, security).agent_only();
    let registration = register(binding).await.expect("host object is served");
    let address = registration.address();
    let proxy = crate::modules::runtime::runtime()
        .await
        .unwrap()
        .connection()
        .proxy(
            &address.destination,
            &address.object_path,
            CALLBACK_INTERFACE,
        )
        .expect("the contract callback interface is served");
    (registration, proxy)
}

#[tokio::test]
async fn schedule_trigger_runs_an_internally_seeded_memory_job_in_agent_only_scope() {
    let tmp = TempDir::new().unwrap();
    let mut config = config(&tmp);
    crate::cron::system_jobs::ensure_memory_jobs(&config).unwrap();
    config.cron.agent_jobs_only = true;
    let job = crate::cron::list_jobs(&config)
        .unwrap()
        .into_iter()
        .find(|job| {
            crate::cron::system_jobs::system_job_name(job)
                == Some(crate::memory::lifecycle::jobs::BACKGROUND_JOB)
        })
        .unwrap();

    let calls = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&calls);
    let _handler = crate::cron::system_job_handlers::register(
        crate::memory::lifecycle::jobs::BACKGROUND_JOB,
        Arc::new(move |context| {
            assert_eq!(context.name, crate::memory::lifecycle::jobs::BACKGROUND_JOB);
            let seen = Arc::clone(&seen);
            Box::pin(async move {
                seen.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        }),
    );
    let (_host, proxy) = agent_only_host(&config).await;
    let attempt: Attempt = proxy
        .call(
            callbacks::SCHEDULE_TRIGGER,
            (&job.command, "run-memory-agent-only"),
        )
        .await
        .unwrap();

    assert!(attempt.success, "{}", attempt.output);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
