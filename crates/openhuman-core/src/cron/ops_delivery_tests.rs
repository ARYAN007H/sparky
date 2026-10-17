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
    assert_eq!(runs[0].delivery_status, None);
}

#[tokio::test]
async fn cancelled_waiting_caller_keeps_manual_run_callback_served_until_completion() {
    use crate::core::events::DomainEvent;
    use tinybus::EventHandler;
    use tinyflows_bus::schedule::callbacks;
    use tokio::sync::{mpsc, oneshot};

    struct Completions {
        job_id: String,
        name: String,
        tx: mpsc::UnboundedSender<bool>,
    }
    #[async_trait]
    impl EventHandler<DomainEvent> for Completions {
        fn name(&self) -> &str {
            &self.name
        }
        async fn handle(&self, event: &DomainEvent) {
            if let DomainEvent::CronJobCompleted {
                job_id, success, ..
            } = event
                && job_id == &self.job_id
            {
                let _ = self.tx.send(*success);
            }
        }
    }

    crate::core::bus::init().await.unwrap();
    let tmp = TempDir::new().unwrap();
    let config = tests::test_config(&tmp);
    let name = format!("ops-run-now-cancel-{}", uuid::Uuid::new_v4().simple());
    let job = crate::cron::system_jobs::ensure_system_job(
        &config,
        &name,
        tinyflows_bus::schedule::types::Schedule::Every {
            every_ms: 3_600_000,
        },
    )
    .unwrap();
    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    let started_tx = Arc::new(Mutex::new(Some(started_tx)));
    let release_rx = Arc::new(Mutex::new(Some(release_rx)));
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let handler_calls = Arc::clone(&calls);
    let _handler = crate::cron::system_job_handlers::register(
        &name,
        Arc::new(move |_| {
            let first_call = handler_calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0;
            let started = first_call.then(|| started_tx.lock().unwrap().take().unwrap());
            let release = first_call.then(|| release_rx.lock().unwrap().take().unwrap());
            Box::pin(async move {
                if let Some(started) = started {
                    let _ = started.send(());
                }
                if let Some(release) = release {
                    release.await.map_err(|error| error.to_string())?;
                }
                Ok(())
            })
        }),
    );

    let (events_tx, mut events_rx) = mpsc::unbounded_channel();
    let _subscription = crate::core::bus::BUS
        .subscribe(Arc::new(Completions {
            job_id: job.id.clone(),
            name: format!("test::cron::cancelled_waiter::{}", job.id),
            tx: events_tx,
        }))
        .unwrap();
    let first_run_id = format!("cancelled-{}", uuid::Uuid::new_v4());
    let first_ctx = crate::core::runtime::DispatchCtx::ambient();
    let first_config = config.clone();
    let first_job = job.clone();
    let first_id = first_run_id.clone();
    let waiting_call = tokio::spawn(async move {
        crate::cron::scheduler::run_job_now(
            &first_ctx,
            &first_config,
            first_job,
            first_id,
            true,
            tinyflows_bus::schedule::ManualDelivery::Requested,
        )
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(3), started_rx)
        .await
        .expect("the module started the admitted system job")
        .expect("the job-start signal remains open");

    waiting_call.abort();
    assert!(waiting_call.await.unwrap_err().is_cancelled());
    let address = crate::cron::bus_host::take_callback_address_for_test(&first_run_id)
        .expect("the cancelled request's callback address was recorded");
    let probe = crate::modules::runtime::runtime()
        .await
        .unwrap()
        .connection()
        .proxy(
            &address.destination,
            &address.object_path,
            tinyflows_bus::schedule::CALLBACK_INTERFACE,
        )
        .unwrap();
    let failed = tinyflows_bus::schedule::Attempt {
        success: false,
        output: "probe".into(),
        retryable: false,
        raw_detail: None,
    };
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let result: std::result::Result<(), _> = probe
                .call(callbacks::FAILED, (&job, "lifecycle-probe", &failed))
                .await;
            if result.is_err() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the current caller cancellation unregisters its callback");
    let _ = release_tx.send(());

    let second_run_id = format!("after-cancel-{}", uuid::Uuid::new_v4());
    let second_ctx = crate::core::runtime::DispatchCtx::ambient();
    let second_config = config.clone();
    let second_job = job.clone();
    let second_id = second_run_id.clone();
    tokio::time::timeout(std::time::Duration::from_secs(5), async move {
        loop {
            match crate::cron::scheduler::run_job_now(
                &second_ctx,
                &second_config,
                second_job.clone(),
                second_id.clone(),
                true,
                tinyflows_bus::schedule::ManualDelivery::Requested,
            )
            .await
            {
                Ok(tinyflows_bus::schedule::RunJobReply::Completed {
                    success: true, ..
                }) => break,
                Ok(tinyflows_bus::schedule::RunJobReply::Refused {
                    reason: tinyflows_bus::schedule::RunJobRefusal::AlreadyRunning,
                }) => {
                    tokio::task::yield_now().await;
                }
                Err(error) if error.to_string().contains("AlreadyRunning") => {
                    tokio::task::yield_now().await;
                }
                other => panic!("second manual run did not complete after the first settled: {other:?}"),
            }
        }
    })
    .await
    .expect("the first manual worker settles and releases its claim");

    let completions = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        vec![events_rx.recv().await.unwrap(), events_rx.recv().await.unwrap()]
    })
    .await;
    assert_eq!(
        completions,
        Ok(vec![true, true]),
        "cancelling the waiter must not orphan the module worker before terminal projection"
    );
    let runs = cron::list_runs(&config, &job.id, 10).unwrap();
    assert_eq!(runs.len(), 2);
    assert!(
        runs.iter().all(|run| run.status == "ok"),
        "the cancelled caller's worker must not leave a queued occurrence behind: {runs:?}"
    );
}
