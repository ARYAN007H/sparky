use super::*;
use tempfile::TempDir;
use tinyflows_schedule::types::Schedule;

fn config(tmp: &TempDir) -> Config {
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    Config {
        workspace_dir: workspace.clone(),
        action_dir: workspace,
        config_path: tmp.path().join("config.toml"),
        ..Config::default()
    }
}

fn shell_job(config: &Config, name: &str) -> String {
    crate::cron::add_shell_job(
        config,
        Some(name.into()),
        Schedule::Every { every_ms: 60_000 },
        "echo policy",
    )
    .unwrap()
    .id
}

#[test]
fn a_job_without_a_policy_row_gets_the_default() {
    let tmp = TempDir::new().unwrap();
    let config = config(&tmp);
    assert_eq!(
        get_policy(&config, "no-such-job").unwrap(),
        JobPolicy::default()
    );
}

#[test]
fn a_policy_round_trips_including_zero_retries() {
    let tmp = TempDir::new().unwrap();
    let config = config(&tmp);
    let id = shell_job(&config, "round-trip");
    let policy = JobPolicy {
        retries: Some(0),
        single_flight: true,
    };
    set_policy(&config, &id, policy).unwrap();
    assert_eq!(get_policy(&config, &id).unwrap(), policy);
}

#[test]
fn setting_the_default_policy_reverts_the_job_to_the_default() {
    let tmp = TempDir::new().unwrap();
    let config = config(&tmp);
    let id = shell_job(&config, "back-to-default");
    set_policy(
        &config,
        &id,
        JobPolicy {
            retries: Some(5),
            single_flight: false,
        },
    )
    .unwrap();
    set_policy(&config, &id, JobPolicy::default()).unwrap();
    assert_eq!(get_policy(&config, &id).unwrap(), JobPolicy::default());
    assert_eq!(
        crate::cron::get_job(&config, &id).unwrap().policy,
        JobPolicy::default()
    );
}

#[test]
fn removing_a_job_takes_its_policy_with_it() {
    let tmp = TempDir::new().unwrap();
    let config = config(&tmp);
    let id = shell_job(&config, "removed");
    set_policy(
        &config,
        &id,
        JobPolicy {
            retries: Some(1),
            single_flight: true,
        },
    )
    .unwrap();
    crate::cron::remove_job(&config, &id).unwrap();
    assert_eq!(get_policy(&config, &id).unwrap(), JobPolicy::default());
    assert!(crate::cron::get_job(&config, &id).is_err());
}

#[test]
fn clearing_every_job_clears_every_policy() {
    let tmp = TempDir::new().unwrap();
    let config = config(&tmp);
    for name in ["one", "two"] {
        let id = shell_job(&config, name);
        set_policy(
            &config,
            &id,
            JobPolicy {
                retries: None,
                single_flight: true,
            },
        )
        .unwrap();
    }
    crate::cron::clear_all_jobs(&config).unwrap();
    assert!(crate::cron::list_jobs(&config).unwrap().is_empty());
}

#[test]
fn effective_retries_prefers_the_job_override() {
    let mut config = Config::default();
    config.reliability.scheduler_retries = 2;
    assert_eq!(effective_retries(&config, &JobPolicy::default()), 2);
    let none = JobPolicy {
        retries: Some(0),
        single_flight: false,
    };
    assert_eq!(effective_retries(&config, &none), 0);
}

#[test]
fn a_policy_stays_with_its_job_across_other_updates() {
    let tmp = TempDir::new().unwrap();
    let config = config(&tmp);
    let id = shell_job(&config, "renamed");
    let policy = JobPolicy {
        retries: Some(3),
        single_flight: true,
    };
    set_policy(&config, &id, policy).unwrap();
    crate::cron::update_job(
        &config,
        &id,
        crate::cron::CronJobPatch {
            name: Some("renamed again".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(get_policy(&config, &id).unwrap(), policy);
}

#[test]
fn clearing_the_policy_of_a_missing_job_is_not_an_error() {
    let tmp = TempDir::new().unwrap();
    let config = config(&tmp);
    clear_policy(&config, "no-such-job").unwrap();
}

#[test]
fn only_the_stores_own_missing_job_message_counts_as_missing() {
    assert!(is_missing(
        &anyhow::anyhow!("Cron job 'j1' not found"),
        "j1"
    ));
    assert!(!is_missing(
        &anyhow::anyhow!("Cron job 'j2' not found"),
        "j1"
    ));
    // A backend failure that merely mentions "not found" must fail closed.
    let backend = anyhow::anyhow!("collection not found").context("cron store: backend failed");
    assert!(!is_missing(&backend, "j1"));
}
