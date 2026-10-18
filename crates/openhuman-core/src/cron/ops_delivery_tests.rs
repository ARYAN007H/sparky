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
async fn manual_run_projects_its_captured_job_after_the_saved_job_is_deleted() {
    use crate::core::events::DomainEvent;
    use tinybus::EventHandler;
    use tokio::sync::{mpsc, oneshot};

    #[derive(Debug, PartialEq, Eq)]
    enum Seen {
        Triggered { id: String, name: String, kind: String },
        Completed { id: String, success: bool },
    }
    struct Events(String, mpsc::UnboundedSender<Seen>);
    #[async_trait]
    impl EventHandler<DomainEvent> for Events {
        fn name(&self) -> &str {
            "test::cron::captured_deleted_manual_job"
        }
        async fn handle(&self, event: &DomainEvent) {
            let seen = match event {
                DomainEvent::CronJobTriggered {
                    job_id,
                    job_name,
                    job_type,
                } if job_id == &self.0 => Some(Seen::Triggered {
                    id: job_id.clone(),
                    name: job_name.clone(),
                    kind: job_type.clone(),
                }),
                DomainEvent::CronJobCompleted {
                    job_id, success, ..
                } if job_id == &self.0 => Some(Seen::Completed {
                    id: job_id.clone(),
                    success: *success,
                }),
                _ => None,
            };
            if let Some(seen) = seen {
                let _ = self.1.send(seen);
            }
        }
    }

    crate::core::bus::init().await.unwrap();
    let tmp = TempDir::new().unwrap();
    let config = tests::test_config(&tmp);
    let name = format!("captured-deleted-{}", uuid::Uuid::new_v4().simple());
    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    let started_tx = Arc::new(Mutex::new(Some(started_tx)));
    let release_rx = Arc::new(Mutex::new(Some(release_rx)));
    let _handler = crate::cron::system_job_handlers::register(
        &name,
        Arc::new(move |_| {
            let started = started_tx.lock().unwrap().take();
            let release = release_rx.lock().unwrap().take();
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
    let mut job = tests::single_flight_system_job(&config, &name);
    job.name = Some("captured before delete".to_string());
    cron::update_job(
        &config,
        &job.id,
        CronJobPatch {
            name: Some(job.name.clone()),
            ..CronJobPatch::default()
        },
    )
    .unwrap();
    job = cron::get_job(&config, &job.id).unwrap();

    let (events_tx, mut events_rx) = mpsc::unbounded_channel();
    let _subscription = crate::core::bus::BUS
        .subscribe(Arc::new(Events(job.id.clone(), events_tx)))
        .unwrap();
    let run_id = format!("captured-{}", uuid::Uuid::new_v4());
    let run_ctx = crate::core::runtime::DispatchCtx::ambient();
    let run_config = config.clone();
    let run_job = job.clone();
    let run_identity = run_id.clone();
    let running = tokio::spawn(async move {
        crate::cron::scheduler::run_job_now(
            &run_ctx,
            &run_config,
            run_job,
            run_identity,
            true,
            tinyflows_bus::schedule::ManualDelivery::Requested,
        )
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(3), started_rx)
        .await
        .expect("captured manual job entered its handler")
        .expect("handler start signal remains open");
    let triggered = tokio::time::timeout(std::time::Duration::from_secs(3), events_rx.recv())
        .await
        .expect("trigger projection arrives")
        .unwrap();
    assert_eq!(
        triggered,
        Seen::Triggered {
            id: job.id.clone(),
            name: "captured before delete".to_string(),
            kind: "Flow".to_string(),
        }
    );

    cron::remove_job(&config, &job.id).unwrap();
    let _ = release_tx.send(());
    let completed = tokio::time::timeout(std::time::Duration::from_secs(3), events_rx.recv())
        .await
        .expect("completion projection arrives for the captured job")
        .unwrap();
    assert_eq!(
        completed,
        Seen::Completed {
            id: job.id.clone(),
            success: true,
        }
    );
    let reply = running.await.unwrap().unwrap();
    assert!(matches!(reply, tinyflows_bus::schedule::RunJobReply::Completed { success: true, .. }));
    let runs = cron::list_runs(&config, &job.id, 10).unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].run_id.as_deref(), Some(run_id.as_str()));
}

#[tokio::test]
async fn cancelled_waiting_caller_keeps_manual_run_callback_served_until_completion() {
    use crate::core::events::DomainEvent;
    use tinybus::EventHandler;
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
            {
                if job_id == &self.job_id {
                    let _ = self.tx.send(*success);
                }
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
    let _ = release_tx.send(());

    let completed = tokio::time::timeout(std::time::Duration::from_secs(3), events_rx.recv())
    .await;
    assert_eq!(
        completed,
        Ok(Some(true)),
        "cancelling the waiter must not orphan the module worker before terminal projection"
    );
    let runs = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let runs = cron::list_runs(&config, &job.id, 10).unwrap();
            if runs.iter().any(|run| run.status == "ok") {
                break runs;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the detached module worker commits its terminal run");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, "ok");
    assert_eq!(runs[0].run_id.as_deref(), Some(first_run_id.as_str()));
}
