//! The store itself is tested upstream (`tinyflows-drivers`); these pin the
//! host mapping from `Config` to its options and the import of the `jobs.db`
//! tinyflows used to write.

use super::*;
use chrono::Duration as ChronoDuration;
use tempfile::TempDir;

fn test_config(tmp: &TempDir) -> Config {
    let config = Config {
        workspace_dir: tmp.path().join("workspace"),
        action_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("config.toml"),
        ..Config::default()
    };
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    config
}

#[test]
fn database_lives_at_workspace_cron_schedule_db() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    add_job(&config, "*/5 * * * *", "echo ok").unwrap();
    assert!(tmp.path().join("workspace/cron/schedule.db").is_file());
}

#[test]
fn due_jobs_batch_size_follows_scheduler_max_tasks() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp);
    config.scheduler.max_tasks = 2;
    for i in 0..3 {
        add_job(&config, "* * * * *", &format!("echo due-{i}")).unwrap();
    }
    let far_future = Utc::now() + ChronoDuration::days(365);
    assert_eq!(due_jobs(&config, far_future).unwrap().len(), 2);
}

#[test]
fn run_history_cap_follows_cron_max_run_history() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp);
    config.cron.max_run_history = 2;
    let job = add_job(&config, "*/15 * * * *", "echo run").unwrap();
    for i in 0..5 {
        let t = Utc::now() + ChronoDuration::seconds(i);
        record_run(&config, &job.id, t, t, "ok", Some("x"), 1).unwrap();
    }
    assert_eq!(list_runs(&config, &job.id, 10).unwrap().len(), 2);
}

fn agent_context(
    config: &Config,
    agent: &str,
) -> std::sync::Arc<crate::core::runtime::CoreContext> {
    use crate::core::runtime::{ContextOverlay, CoreContext, DomainSet};
    CoreContext::for_test_with_config(DomainSet::full(), config.clone()).derive_with(
        ContextOverlay::new(
            config.clone(),
            DomainSet::full(),
            crate::tools::toolpacks::ToolGroups::none(),
        )
        .session_agent(agent),
    )
}

#[tokio::test]
async fn an_agents_jobs_live_in_its_own_database() {
    use crate::core::runtime::CoreContext;
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);

    let job = CoreContext::scope(agent_context(&config, "alpha"), async {
        add_job(&config, "*/5 * * * *", "echo alpha").unwrap()
    })
    .await;

    assert!(tmp
        .path()
        .join("workspace/agents/alpha/cron/schedule.db")
        .is_file());
    let alpha_ids = CoreContext::scope(agent_context(&config, "alpha"), async {
        list_jobs(&config)
            .unwrap()
            .into_iter()
            .map(|job| job.id)
            .collect::<Vec<_>>()
    })
    .await;
    assert_eq!(alpha_ids, std::slice::from_ref(&job.id));
    let beta_jobs = CoreContext::scope(agent_context(&config, "beta"), async {
        list_jobs(&config).unwrap()
    })
    .await;
    assert!(beta_jobs.is_empty());
    assert!(
        list_jobs(&config).unwrap().is_empty(),
        "the workspace store stays empty"
    );
}

#[tokio::test]
async fn a_job_whose_agent_is_not_live_stays_dormant() {
    use crate::core::runtime::CoreContext;
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let ctx = agent_context(&config, "dormant-agent");
    let job = CoreContext::scope(std::sync::Arc::clone(&ctx), async {
        add_job(&config, "* * * * *", "echo dormant").unwrap()
    })
    .await;
    drop(ctx);

    let mut dispatcher = crate::cron::scheduler::JobDispatcher::new(1);
    crate::cron::scheduler::tick_live_agents(&mut dispatcher).await;
    dispatcher.drain().await;

    let after = CoreContext::scope(agent_context(&config, "dormant-agent"), async {
        get_job(&config, &job.id).unwrap()
    })
    .await;
    assert!(
        after.last_run.is_none(),
        "nothing ran the dormant agent's job"
    );
}

fn legacy_fixture() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/storage/import/fixtures/tinyflows/workspace/cron/jobs.db")
}

#[test]
fn a_legacy_jobs_db_is_imported_on_first_use_and_left_untouched() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let legacy = legacy_db_path(&config);
    std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    std::fs::copy(legacy_fixture(), &legacy).unwrap();
    let before = std::fs::read(&legacy).unwrap();

    let jobs = list_jobs(&config).unwrap();
    assert_eq!(jobs.len(), 3);
    assert!(jobs.iter().any(|job| job.name.as_deref() == Some("inbox")));
    assert!(find_flow_schedule_job(&config, "flow-beta")
        .unwrap()
        .is_some());
    assert_eq!(
        std::fs::read(&legacy).unwrap(),
        before,
        "jobs.db was modified"
    );

    // Imported once: a job removed afterwards stays removed.
    let inbox = jobs
        .iter()
        .find(|job| job.name.as_deref() == Some("inbox"))
        .unwrap();
    remove_job(&config, &inbox.id).unwrap();
    crate::storage::local::forget(&db_path(&config));
    assert_eq!(list_jobs(&config).unwrap().len(), 2);
}
