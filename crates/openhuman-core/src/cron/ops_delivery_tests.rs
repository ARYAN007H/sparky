use super::*;
use crate::cron;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use tinychannels_bus::{Channel, ChannelMessage, SendMessage};
use tinyflows_bus::schedule::types::{DeliveryConfig, DeliveryStatus};
use tokio::sync::mpsc;

struct FailingTelegram;

#[async_trait]
impl Channel for FailingTelegram {
    fn name(&self) -> &str {
        "telegram"
    }

    async fn send(&self, _message: &SendMessage) -> anyhow::Result<()> {
        anyhow::bail!("mock telegram send failed")
    }

    async fn listen(&self, _tx: mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        Ok(())
    }
}

pub(super) fn install_failing_telegram_bridge() {
    crate::cron::channel_bridge::register_channel_bridge(
        Arc::new(Mutex::default()),
        Arc::new(std::collections::HashMap::from([(
            "telegram".to_string(),
            Arc::new(FailingTelegram) as Arc<dyn Channel>,
        )])),
        crate::cron::channel_bridge::MAX_HISTORY_MESSAGES,
    );
}

#[tokio::test]
async fn run_job_now_keeps_best_effort_failure_successful_and_records_failed_delivery() {
    let _bridge_lock = crate::cron::channel_bridge::TEST_BRIDGE_LOCK.lock().await;
    let tmp = TempDir::new().unwrap();
    let mut config = tests::test_config(&tmp);
    config.channels_config.telegram = Some(crate::config::TelegramConfig {
        bot_token: "test-token".into(),
        chat_id: None,
        allowed_users: vec!["target".into()],
        stream_mode: Default::default(),
        draft_update_interval_ms: 1000,
        silent_streaming: true,
        mention_only: false,
    });
    install_failing_telegram_bridge();
    let _handler = crate::cron::system_job_handlers::register(
        "ops-run-now-best-effort-delivery-failure",
        std::sync::Arc::new(|_ctx| Box::pin(async { Ok(()) })),
    );
    let job = tests::single_flight_system_job(&config, "ops-run-now-best-effort-delivery-failure");
    cron::update_job(
        &config,
        &job.id,
        CronJobPatch {
            delivery: Some(DeliveryConfig {
                mode: "announce".into(),
                channel: Some("telegram".into()),
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
