use super::*;
use crate::cron;
use tempfile::TempDir;
use tinyflows_bus::schedule::types::{DeliveryConfig, DeliveryStatus};

#[tokio::test]
async fn run_job_now_keeps_best_effort_failure_successful_and_records_failed_delivery() {
    let tmp = TempDir::new().unwrap();
    let config = tests::test_config(&tmp);
    let _handler = crate::cron::system_job_handlers::register(
        "ops-run-now-best-effort-delivery-failure",
        std::sync::Arc::new(|_ctx| Box::pin(async { Ok(()) })),
    );
    let job =
        tests::single_flight_system_job(&config, "ops-run-now-best-effort-delivery-failure");
    let missing_channel = format!("missing-{}", uuid::Uuid::new_v4());
    cron::update_job(
        &config,
        &job.id,
        CronJobPatch {
            delivery: Some(DeliveryConfig {
                mode: "announce".into(),
                channel: Some(missing_channel),
                to: Some("target".into()),
                best_effort: true,
            }),
            ..CronJobPatch::default()
        },
    )
    .unwrap();

    let (success, _output) = run_job_now(
        &crate::core::runtime::DispatchCtx::ambient(),
        &config,
        &job.id,
    )
    .await
    .unwrap();

    assert!(
        success,
        "best-effort delivery failure preserves execution success"
    );
    let runs = cron::list_runs(&config, &job.id, 10).unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, "ok");
    assert_eq!(runs[0].delivery_status, Some(DeliveryStatus::Failed));
}
