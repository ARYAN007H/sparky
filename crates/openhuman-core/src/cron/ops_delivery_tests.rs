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

#[tokio::test]
async fn run_job_now_pairs_trigger_and_completion_after_delivery() {
    use crate::core::events::DomainEvent;
    use tinybus::EventHandler;

    struct Events(String, mpsc::UnboundedSender<&'static str>);
    #[async_trait]
    impl EventHandler<DomainEvent> for Events {
        fn name(&self) -> &str {
            "test::cron::run_now_event_pair"
        }
        async fn handle(&self, event: &DomainEvent) {
            let (job_id, label) = match event {
                DomainEvent::CronJobTriggered { job_id, .. } => (job_id, "triggered"),
                DomainEvent::CronJobCompleted { job_id, .. } => (job_id, "completed"),
                _ => return,
            };
            if job_id == &self.0 {
                let _ = self.1.send(label);
            }
        }
    }

    crate::core::bus::init().await.unwrap();
    let tmp = TempDir::new().unwrap();
    let config = tests::test_config(&tmp);
    let name = "ops-run-now-event-pair";
    let _handler =
        crate::cron::system_job_handlers::register(name, Arc::new(|_| Box::pin(async { Ok(()) })));
    let job = tests::single_flight_system_job(&config, name);
    let (tx, mut rx) = mpsc::unbounded_channel();
    let _subscription = crate::core::bus::BUS
        .subscribe(Arc::new(Events(job.id.clone(), tx)))
        .unwrap();

    let (success, _) = run_job_now(
        &crate::core::runtime::DispatchCtx::ambient(),
        &config,
        &job.id,
    )
    .await
    .unwrap();
    assert!(success);
    let pair = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        let mut seen = Vec::new();
        loop {
            let event = rx.recv().await.expect("cron event subscriber remains open");
            seen.push(event);
            if event == "completed" {
                break;
            }
        }
        seen
    })
    .await
    .expect("Run Now completion event arrives");
    assert_eq!(pair, vec!["triggered", "completed"]);
    let runs = cron::list_runs(&config, &job.id, 10).unwrap();
    assert_eq!(runs[0].delivery_status, Some(DeliveryStatus::NotRequested));
}

#[tokio::test]
async fn completion_events_keep_their_agent_owner_on_repeated_reads() {
    use crate::core::events::DomainEvent;
    use tinybus::EventHandler;

    struct Completions(mpsc::UnboundedSender<DomainEvent>);
    #[async_trait]
    impl EventHandler<DomainEvent> for Completions {
        fn name(&self) -> &str {
            "test::cron::completion_owner_is_event_data"
        }

        async fn handle(&self, event: &DomainEvent) {
            if matches!(event, DomainEvent::CronJobCompleted { job_id, .. } if job_id == "shared-completion-id") {
                let _ = self.0.send(event.clone());
            }
        }
    }

    crate::core::bus::init().await.unwrap();
    let (tx, mut rx) = mpsc::unbounded_channel();
    let _subscription = crate::core::bus::BUS
        .subscribe(Arc::new(Completions(tx)))
        .unwrap();

    let context_for = |agent: &str| {
        crate::core::runtime::CoreContext::for_test(
            crate::core::runtime::DomainSet::full(),
            None,
        )
        .derive_with(
            crate::core::runtime::ContextOverlay::new(
                crate::config::Config::default(),
                crate::core::runtime::DomainSet::full(),
                Default::default(),
            )
            .session_agent(agent),
        )
    };
    let job = cron::CronJob {
        id: "shared-completion-id".into(),
        ..tests::single_flight_system_job(
            &tests::test_config(&TempDir::new().unwrap()),
            "completion-owner-test",
        )
    };

    crate::cron::scheduler::run_delivery::publish_completion(
        &crate::core::runtime::DispatchCtx::new(context_for("completion-owner-a")),
        &job,
        true,
        "done",
    );
    crate::cron::scheduler::run_delivery::publish_completion(
        &crate::core::runtime::DispatchCtx::new(context_for("completion-owner-b")),
        &job,
        true,
        "done",
    );

    let events = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        let mut events = Vec::new();
        while events.len() < 2 {
            events.push(rx.recv().await.expect("completion subscriber remains open"));
        }
        events
    })
    .await
    .expect("both completion events arrive");
    let owners = futures::future::join_all(events.iter().map(|event| {
        event_owner(
            &crate::core::runtime::DispatchCtx::host_default(),
            &crate::config::Config::default(),
            event,
        )
    }))
    .await;

    assert_eq!(owners[0], Ok(Some("completion-owner-a".into())));
    assert_eq!(owners[1], Ok(Some("completion-owner-b".into())));
    assert_eq!(
        event_owner(
            &crate::core::runtime::DispatchCtx::host_default(),
            &crate::config::Config::default(),
            &events[0],
        )
        .await,
        Ok(Some("completion-owner-a".into())),
        "reading an event repeatedly must not consume or change its owner"
    );
}
