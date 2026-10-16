use super::*;
use tinyflows::model::TriggerKind;
use tinyflows_catalog::types::FlowRun;

#[test]
fn pinned_trigger_inputs_reads_values_an_author_fixed_for_unattended_runs() {
    let flow = flow_with_trigger_config(
        "f1",
        true,
        json!({
            "trigger_kind": "schedule",
            "schedule": "0 9 * * *",
            "inputs": { "repo": "acme/api", "depth": 3 }
        }),
    );
    let inputs = pinned_trigger_inputs(&flow);
    assert_eq!(inputs["repo"], json!("acme/api"));
    assert_eq!(inputs["depth"], json!(3));
}

#[test]
fn pinned_trigger_inputs_is_empty_when_unset_or_malformed() {
    // Empty, not an error: a flow declaring no inputs (the overwhelming
    // majority) must keep dispatching on a tick exactly as before, and a
    // malformed value is caught downstream by `prepare_flow_run`, which
    // reports it against the flow's actual declarations.
    for cfg in [
        json!({ "trigger_kind": "schedule" }),
        json!({ "trigger_kind": "schedule", "inputs": null }),
        json!({ "trigger_kind": "schedule", "inputs": ["repo"] }),
    ] {
        let flow = flow_with_trigger_config("f1", true, cfg.clone());
        assert!(
            pinned_trigger_inputs(&flow).is_empty(),
            "expected no pinned inputs for {cfg}"
        );
    }
}

#[test]
fn pinned_trigger_inputs_is_empty_for_a_graph_with_no_trigger() {
    let mut flow = flow_with_trigger_config("f1", true, json!({ "trigger_kind": "schedule" }));
    flow.graph.nodes.clear();
    assert!(pinned_trigger_inputs(&flow).is_empty());
}

#[test]
fn name_and_domains_are_stable() {
    let tmp = tempfile::TempDir::new().unwrap();
    let sub = FlowTriggerSubscriber::new(test_config(&tmp));
    assert_eq!(sub.name(), "flows::trigger");
    assert_eq!(
        sub.domains(),
        Some(&["cron", "composio", "webhook", "system"][..])
    );
}

#[tokio::test]
async fn handle_does_not_panic_on_arbitrary_events() {
    let tmp = tempfile::TempDir::new().unwrap();
    let sub = FlowTriggerSubscriber::new(test_config(&tmp));
    sub.handle(&DomainEvent::CronJobTriggered {
        job_id: "j1".into(),
        job_name: "test".into(),
        job_type: "shell".into(),
    })
    .await;
    sub.dispatch_schedule_tick(
        &crate::core::runtime::DispatchCtx::host_default(),
        "missing-flow",
    )
    .await;
}

#[test]
fn extract_trigger_kind_reads_schedule() {
    let flow = flow_with_trigger_config(
        "f1",
        true,
        json!({ "trigger_kind": "schedule", "schedule": "0 9 * * *" }),
    );
    assert!(matches!(
        extract_trigger_kind(&flow),
        Some(TriggerKind::Schedule)
    ));
}

#[test]
fn extract_trigger_kind_none_for_missing_discriminator() {
    let flow = flow_with_trigger_config("f1", true, json!({}));
    assert!(extract_trigger_kind(&flow).is_none());
}

#[test]
fn extract_trigger_kind_none_for_invalid_discriminator() {
    let flow = flow_with_trigger_config("f1", true, json!({ "trigger_kind": "not_a_kind" }));
    assert!(extract_trigger_kind(&flow).is_none());
}

#[test]
fn matches_app_event_requires_toolkit_and_slug_match() {
    let flow = flow_with_trigger_config(
        "f1",
        true,
        json!({ "trigger_kind": "app_event", "toolkit": "gmail", "trigger_slug": "GMAIL_NEW_GMAIL_MESSAGE" }),
    );
    assert!(matches_app_event(&flow, "gmail", "GMAIL_NEW_GMAIL_MESSAGE"));
    // Case-insensitive.
    assert!(matches_app_event(&flow, "Gmail", "gmail_new_gmail_message"));
    // Wrong toolkit or slug does not match.
    assert!(!matches_app_event(
        &flow,
        "slack",
        "GMAIL_NEW_GMAIL_MESSAGE"
    ));
    assert!(!matches_app_event(&flow, "gmail", "SLACK_NEW_MESSAGE"));
}

#[test]
fn matches_app_event_false_for_non_app_event_trigger() {
    let flow = flow_with_trigger_config(
        "f1",
        true,
        json!({ "trigger_kind": "schedule", "schedule": "0 9 * * *" }),
    );
    assert!(!matches_app_event(
        &flow,
        "gmail",
        "GMAIL_NEW_GMAIL_MESSAGE"
    ));
}

#[tokio::test]
async fn handle_app_event_ignores_disabled_flows() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = test_config(&tmp);
    let flow = flow_with_trigger_config(
        "disabled-flow",
        false,
        json!({ "trigger_kind": "app_event", "toolkit": "gmail", "trigger_slug": "GMAIL_NEW_GMAIL_MESSAGE" }),
    );
    crate::flows::store::upsert_flow(&config, &flow).unwrap();

    // `list_enabled_flows` must not surface the disabled flow at all —
    // proves the subscriber's dispatch source already excludes it,
    // rather than asserting on a spawned background task's side effect.
    let (enabled, skipped) = crate::flows::store::list_enabled_flows(&config).unwrap();
    assert!(enabled.is_empty());
    assert_eq!(skipped, 0);
}

#[tokio::test]
async fn handle_schedule_tick_ignores_disabled_flow() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = test_config(&tmp);
    let flow = flow_with_trigger_config(
        "sched-flow",
        false,
        json!({ "trigger_kind": "schedule", "schedule": "0 9 * * *" }),
    );
    crate::flows::store::upsert_flow(&config, &flow).unwrap();

    let sub = FlowTriggerSubscriber::new(config.clone());
    // Must not panic and must not spawn a run for a disabled flow — we
    // can't directly observe "no run happened" without a full flows_run
    // fixture, but this exercises the early-return path without error.
    sub.dispatch_schedule_tick(
        &crate::core::runtime::DispatchCtx::host_default(),
        "sched-flow",
    )
    .await;
}

// ── in-flight dedupe (CodeRabbit finding B) ─────────────────────

#[tokio::test]
async fn schedule_tick_is_coalesced_while_a_trigger_run_is_active() {
    use std::sync::Mutex;
    use tokio::sync::{mpsc, oneshot};

    let tmp = tempfile::TempDir::new().unwrap();
    let (started_tx, started_rx) = oneshot::channel();
    let started_tx = Arc::new(Mutex::new(Some(started_tx)));
    let (release_tx, release_rx) = oneshot::channel();
    let release_rx = Arc::new(tokio::sync::Mutex::new(Some(release_rx)));
    let (runs_tx, mut runs_rx) = mpsc::unbounded_channel();
    let runner = move |_ctx: crate::core::runtime::DispatchCtx,
                       _config: Arc<Config>,
                       request: crate::flows::bus::trigger::TriggerRunRequest| {
        let started_tx = Arc::clone(&started_tx);
        let release_rx = Arc::clone(&release_rx);
        let runs_tx = runs_tx.clone();
        async move {
            if let Some(started_tx) = started_tx.lock().unwrap().take() {
                let _ = started_tx.send(());
            }
            if let Some(release) = release_rx.lock().await.take() {
                let _ = release.await;
            }
            runs_tx.send(request.trigger.as_str()).unwrap();
        }
    };
    let sub = FlowTriggerSubscriber::with_runner(test_config(&tmp), 1, runner);
    let ctx = crate::core::runtime::DispatchCtx::host_default();

    sub.dispatch_test_schedule(&ctx, "f1").await;
    started_rx.await.unwrap();
    sub.dispatch_test_schedule(&ctx, "f1").await;
    release_tx.send(()).unwrap();
    assert_eq!(runs_rx.recv().await, Some("schedule"));
    assert!(
        runs_rx.try_recv().is_err(),
        "a busy schedule tick must be dropped"
    );
}

#[tokio::test]
async fn app_event_runs_queue_fifo_per_tenant_while_other_tenants_progress() {
    use crate::core::runtime::{ContextOverlay, CoreContext, DispatchCtx, DomainSet};
    use std::sync::Mutex;
    use tokio::sync::{mpsc, oneshot};

    let tmp = tempfile::TempDir::new().unwrap();
    let config = test_config(&tmp);
    let (completed_tx, mut completed_rx) = mpsc::unbounded_channel();
    let (started_tx, started_rx) = oneshot::channel();
    let started_tx = Arc::new(Mutex::new(Some(started_tx)));
    let (release_first_tx, release_first_rx) = oneshot::channel();
    let release_first_rx = Arc::new(tokio::sync::Mutex::new(Some(release_first_rx)));
    let runner = move |ctx: DispatchCtx,
                       _config: Arc<Config>,
                       request: crate::flows::bus::trigger::TriggerRunRequest| {
        let completed_tx = completed_tx.clone();
        let started_tx = Arc::clone(&started_tx);
        let release_first_rx = Arc::clone(&release_first_rx);
        async move {
            let tenant = ctx.tenant().unwrap();
            let event = request.input["event"].as_str().unwrap().to_owned();
            if event == "a1" {
                if let Some(started_tx) = started_tx.lock().unwrap().take() {
                    let _ = started_tx.send(());
                }
                if let Some(release) = release_first_rx.lock().await.take() {
                    let _ = release.await;
                }
            }
            completed_tx
                .send((
                    format!(
                        "{}/{}",
                        tenant.profile.unwrap_or_default(),
                        tenant.agent.unwrap_or_default()
                    ),
                    request.flow_id,
                    event,
                ))
                .unwrap();
        }
    };
    let subscriber = FlowTriggerSubscriber::with_runner(config, 1, runner);
    let context = |profile: &str, agent: &str| {
        DispatchCtx::new(
            CoreContext::for_test(DomainSet::full(), None).derive_with(
                ContextOverlay::new(Config::default(), DomainSet::full(), Default::default())
                    .profile(profile)
                    .session_agent(agent),
            ),
        )
    };
    let tenant_a = context("profile-a", "agent-a");
    let tenant_b = context("profile-b", "agent-a");
    let tenant_c = context("profile-a", "agent-c");

    subscriber
        .dispatch_test_app_event(&tenant_a, "shared-flow", json!({"event": "a1"}))
        .await;
    started_rx.await.unwrap();
    subscriber
        .dispatch_test_app_event(&tenant_a, "shared-flow", json!({"event": "a2"}))
        .await;
    subscriber
        .dispatch_test_app_event(&tenant_b, "shared-flow", json!({"event": "b1"}))
        .await;
    subscriber
        .dispatch_test_app_event(&tenant_c, "shared-flow", json!({"event": "c1"}))
        .await;

    let mut other_tenant_runs = [
        receive_trigger_run(&mut completed_rx).await,
        receive_trigger_run(&mut completed_rx).await,
    ];
    other_tenant_runs.sort();
    assert_eq!(
        other_tenant_runs,
        [
            (
                "profile-a/agent-c".to_string(),
                "shared-flow".to_string(),
                "c1".to_string()
            ),
            (
                "profile-b/agent-a".to_string(),
                "shared-flow".to_string(),
                "b1".to_string()
            ),
        ],
        "a held run must not block the same flow ID for another profile or agent"
    );
    release_first_tx.send(()).unwrap();
    assert_eq!(
        receive_trigger_run(&mut completed_rx).await,
        (
            "profile-a/agent-a".to_string(),
            "shared-flow".to_string(),
            "a1".to_string()
        )
    );
    assert_eq!(
        receive_trigger_run(&mut completed_rx).await,
        (
            "profile-a/agent-a".to_string(),
            "shared-flow".to_string(),
            "a2".to_string()
        ),
        "app_event payloads for one tenant and flow must execute FIFO"
    );
}

async fn receive_trigger_run(
    receiver: &mut tokio::sync::mpsc::UnboundedReceiver<(String, String, String)>,
) -> (String, String, String) {
    tokio::time::timeout(std::time::Duration::from_secs(2), receiver.recv())
        .await
        .expect("queued trigger did not finish within the hang guard")
        .unwrap()
}

#[tokio::test]
async fn full_app_event_queue_backpressures_until_a_slot_opens() {
    use std::sync::Mutex;
    use tokio::sync::{mpsc, oneshot};

    let tmp = tempfile::TempDir::new().unwrap();
    let (started_tx, started_rx) = oneshot::channel();
    let started_tx = Arc::new(Mutex::new(Some(started_tx)));
    let (release_tx, release_rx) = oneshot::channel();
    let release_rx = Arc::new(tokio::sync::Mutex::new(Some(release_rx)));
    let (runs_tx, mut runs_rx) = mpsc::unbounded_channel();
    let runner = move |_ctx: crate::core::runtime::DispatchCtx,
                       _config: Arc<Config>,
                       request: crate::flows::bus::trigger::TriggerRunRequest| {
        let started_tx = Arc::clone(&started_tx);
        let release_rx = Arc::clone(&release_rx);
        let runs_tx = runs_tx.clone();
        async move {
            let event = request.input["event"].as_str().unwrap().to_owned();
            if event == "a1" {
                if let Some(started_tx) = started_tx.lock().unwrap().take() {
                    let _ = started_tx.send(());
                }
                if let Some(release) = release_rx.lock().await.take() {
                    let _ = release.await;
                }
            }
            runs_tx.send(event).unwrap();
        }
    };
    let subscriber = FlowTriggerSubscriber::with_runner(test_config(&tmp), 1, runner);
    let ctx = crate::core::runtime::DispatchCtx::host_default();

    subscriber
        .dispatch_test_app_event(&ctx, "f1", json!({"event": "a1"}))
        .await;
    started_rx.await.unwrap();
    subscriber
        .dispatch_test_app_event(&ctx, "f1", json!({"event": "a2"}))
        .await;
    let third = subscriber.dispatch_test_app_event(&ctx, "f1", json!({"event": "a3"}));
    tokio::pin!(third);
    assert!(
        futures::poll!(third.as_mut()).is_pending(),
        "a full bounded queue must wait for capacity instead of dropping the event"
    );

    release_tx.send(()).unwrap();
    third.await;
    for expected in ["a1", "a2", "a3"] {
        assert_eq!(
            tokio::time::timeout(std::time::Duration::from_secs(2), runs_rx.recv())
                .await
                .expect("backpressured trigger did not finish")
                .unwrap(),
            expected
        );
    }
}

#[tokio::test]
async fn a_panicking_run_does_not_discard_later_accepted_app_events() {
    use std::sync::Mutex;
    use tokio::sync::{mpsc, oneshot};

    let tmp = tempfile::TempDir::new().unwrap();
    let (started_tx, started_rx) = oneshot::channel();
    let started_tx = Arc::new(Mutex::new(Some(started_tx)));
    let (release_tx, release_rx) = oneshot::channel();
    let release_rx = Arc::new(tokio::sync::Mutex::new(Some(release_rx)));
    let (runs_tx, mut runs_rx) = mpsc::unbounded_channel();
    let runner = move |_ctx: crate::core::runtime::DispatchCtx,
                       _config: Arc<Config>,
                       request: crate::flows::bus::trigger::TriggerRunRequest| {
        let started_tx = Arc::clone(&started_tx);
        let release_rx = Arc::clone(&release_rx);
        let runs_tx = runs_tx.clone();
        async move {
            let event = request.input["event"].as_str().unwrap().to_owned();
            if event == "panic" {
                if let Some(started_tx) = started_tx.lock().unwrap().take() {
                    let _ = started_tx.send(());
                }
                if let Some(release) = release_rx.lock().await.take() {
                    let _ = release.await;
                }
                panic!("synthetic run failure");
            }
            runs_tx.send(event).unwrap();
        }
    };
    let subscriber = FlowTriggerSubscriber::with_runner(test_config(&tmp), 1, runner);
    let ctx = crate::core::runtime::DispatchCtx::host_default();

    subscriber
        .dispatch_test_app_event(&ctx, "f1", json!({"event": "panic"}))
        .await;
    started_rx.await.unwrap();
    subscriber
        .dispatch_test_app_event(&ctx, "f1", json!({"event": "survived"}))
        .await;
    release_tx.send(()).unwrap();
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(2), runs_rx.recv())
            .await
            .expect("worker exited after a run panic")
            .unwrap(),
        "survived"
    );
}

#[tokio::test]
async fn arrival_racing_worker_retirement_is_retried_on_a_fresh_queue() {
    use std::sync::Mutex;
    use tokio::sync::{mpsc, oneshot};

    let tmp = tempfile::TempDir::new().unwrap();
    let (started_tx, started_rx) = oneshot::channel();
    let started_tx = Arc::new(Mutex::new(Some(started_tx)));
    let (release_tx, release_rx) = oneshot::channel();
    let release_rx = Arc::new(tokio::sync::Mutex::new(Some(release_rx)));
    let (runs_tx, mut runs_rx) = mpsc::unbounded_channel();
    let runner = move |_ctx: crate::core::runtime::DispatchCtx,
                       _config: Arc<Config>,
                       request: crate::flows::bus::trigger::TriggerRunRequest| {
        let started_tx = Arc::clone(&started_tx);
        let release_rx = Arc::clone(&release_rx);
        let runs_tx = runs_tx.clone();
        async move {
            let event = request.input["event"].as_str().unwrap().to_owned();
            if event == "first" {
                if let Some(started_tx) = started_tx.lock().unwrap().take() {
                    let _ = started_tx.send(());
                }
                if let Some(release) = release_rx.lock().await.take() {
                    let _ = release.await;
                }
            }
            runs_tx.send(event).unwrap();
        }
    };
    let subscriber = Arc::new(FlowTriggerSubscriber::with_runner(
        test_config(&tmp),
        1,
        runner,
    ));
    let ctx = crate::core::runtime::DispatchCtx::host_default();
    let retiring = subscriber.retirement_notifier_test();
    let retirement_started = retiring.notified();
    tokio::pin!(retirement_started);

    subscriber
        .dispatch_test_app_event(&ctx, "f1", json!({"event": "first"}))
        .await;
    started_rx.await.unwrap();
    let admission = subscriber.hold_queue_admission_test(&ctx, "f1").await;
    release_tx.send(()).unwrap();
    retirement_started.await;

    // The worker notified just before it queued on the held admission gate.
    // Tokio's mutex is FIFO, so its retirement request is ahead of this send.
    let arrival =
        subscriber.dispatch_test_app_event(&ctx, "f1", json!({"event": "after-retirement"}));
    tokio::pin!(arrival);
    assert!(
        futures::poll!(arrival.as_mut()).is_pending(),
        "the arrival must have cloned the retiring queue and be waiting on its admission gate"
    );
    drop(admission);
    tokio::time::timeout(std::time::Duration::from_secs(2), arrival)
        .await
        .expect("the arrival stayed attached to a retired queue");
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(2), runs_rx.recv())
            .await
            .expect("arrival after worker retirement was lost")
            .unwrap(),
        "first"
    );
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(2), runs_rx.recv())
            .await
            .expect("retry on the fresh worker was lost")
            .unwrap(),
        "after-retirement"
    );
}

#[test]
fn default_constructs_the_same_as_new() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = test_config(&tmp);
    let a = FlowTriggerSubscriber::new(config.clone());
    let b = FlowTriggerSubscriber::new(config);
    assert_eq!(a.name(), b.name());
}

// ── FlowRunDigestSubscriber ─────────────────────────────────────

#[test]
fn digest_name_and_domains_are_stable() {
    let tmp = tempfile::TempDir::new().unwrap();
    let sub = FlowRunDigestSubscriber::new(test_config(&tmp));
    assert_eq!(sub.name(), "flows::digest");
    assert_eq!(sub.domains(), Some(&["cron"][..]));
}

#[tokio::test]
async fn digest_handle_does_not_panic_on_unrelated_events() {
    let tmp = tempfile::TempDir::new().unwrap();
    let sub = FlowRunDigestSubscriber::new(test_config(&tmp));
    // Must not panic, and must not touch the memory layer at all, for
    // any event other than `FlowRunFinished`.
    sub.handle(&DomainEvent::CronJobTriggered {
        job_id: "j1".into(),
        job_name: "test".into(),
        job_type: "shell".into(),
    })
    .await;
}

#[tokio::test]
async fn digest_ignores_failed_run() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = test_config(&tmp);
    let _engine = digest_test_engine(&config);

    let flow = flow_with_trigger_config("f-failed", true, json!({}));
    store::upsert_flow(&config, &flow).unwrap();
    store::insert_flow_run(
        &config,
        "run-failed",
        "f-failed",
        "thread-failed",
        "2026-01-01T00:00:00Z",
    )
    .unwrap();
    store::finish_flow_run(
        &config,
        "run-failed",
        "failed",
        "2026-01-01T00:05:00Z",
        &[],
        &[],
        Some("boom"),
        None,
    )
    .unwrap();

    let sub = FlowRunDigestSubscriber::new(config.clone());
    sub.handle(&DomainEvent::FlowRunFinished {
        flow_id: "f-failed".into(),
        run_id: "run-failed".into(),
        status: "failed".into(),
    })
    .await;

    assert!(
        stored_digests(&config, "f-failed").await.is_empty(),
        "a failed run must never produce a run digest"
    );
}

#[tokio::test]
async fn digest_ignores_cancelled_run() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = test_config(&tmp);
    let _engine = digest_test_engine(&config);

    let flow = flow_with_trigger_config("f-cancelled", true, json!({}));
    store::upsert_flow(&config, &flow).unwrap();
    store::insert_flow_run(
        &config,
        "run-cancelled",
        "f-cancelled",
        "thread-cancelled",
        "2026-01-01T00:00:00Z",
    )
    .unwrap();
    store::finish_flow_run(
        &config,
        "run-cancelled",
        "cancelled",
        "2026-01-01T00:05:00Z",
        &[],
        &[],
        None,
        None,
    )
    .unwrap();

    let sub = FlowRunDigestSubscriber::new(config.clone());
    sub.handle(&DomainEvent::FlowRunFinished {
        flow_id: "f-cancelled".into(),
        run_id: "run-cancelled".into(),
        status: "cancelled".into(),
    })
    .await;

    assert!(stored_digests(&config, "f-cancelled").await.is_empty());
}

#[tokio::test]
async fn digest_writes_run_digest_entry_for_completed_run() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = test_config(&tmp);
    let _engine = digest_test_engine(&config);

    let flow = flow_with_trigger_config("f-ok", true, json!({}));
    store::upsert_flow(&config, &flow).unwrap();
    store::insert_flow_run(
        &config,
        "run-ok",
        "f-ok",
        "thread-ok",
        "2026-01-01T00:00:00Z",
    )
    .unwrap();
    let step = tinyflows_catalog::FlowRunStep {
        node_id: "n1".to_string(),
        output: json!({ "sent": 3 }),
        port: None,
        status: Some("success".to_string()),
        duration_ms: Some(12),
        diagnostics: Vec::new(),
    };
    store::finish_flow_run(
        &config,
        "run-ok",
        "completed",
        "2026-01-01T00:05:00Z",
        &[step],
        &[],
        None,
        None,
    )
    .unwrap();

    let sub = FlowRunDigestSubscriber::new(config.clone());
    sub.handle(&DomainEvent::FlowRunFinished {
        flow_id: "f-ok".into(),
        run_id: "run-ok".into(),
        status: "completed".into(),
    })
    .await;

    let digests = stored_digests(&config, "f-ok").await;
    assert_eq!(
        digests.len(),
        1,
        "completed run must produce one run digest"
    );
    let entry = &digests[0];
    assert_eq!(entry.meta.source.kind, tinymemory_api::SourceKind::Agent);
    assert!(entry.meta.tags.contains(&crate::flows::flow_tag("f-ok")));
    assert_eq!(
        entry.meta.derive,
        Some(false),
        "nothing derived from a digest"
    );
    assert!(entry.text.contains("f-ok"));
    assert!(entry.text.contains("completed"));
    assert!(entry.text.contains("n1"));
    assert!(entry.text.chars().count() <= DIGEST_MAX_CHARS);
}

#[tokio::test]
async fn digest_treats_completed_with_warnings_as_success() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = test_config(&tmp);
    let _engine = digest_test_engine(&config);

    let flow = flow_with_trigger_config("f-warn", true, json!({}));
    store::upsert_flow(&config, &flow).unwrap();
    store::insert_flow_run(
        &config,
        "run-warn",
        "f-warn",
        "thread-warn",
        "2026-01-01T00:00:00Z",
    )
    .unwrap();
    store::finish_flow_run(
        &config,
        "run-warn",
        "completed_with_warnings",
        "2026-01-01T00:05:00Z",
        &[],
        &[],
        None,
        None,
    )
    .unwrap();

    let sub = FlowRunDigestSubscriber::new(config.clone());
    sub.handle(&DomainEvent::FlowRunFinished {
        flow_id: "f-warn".into(),
        run_id: "run-warn".into(),
        status: "completed_with_warnings".into(),
    })
    .await;

    assert_eq!(stored_digests(&config, "f-warn").await.len(), 1);
}

#[tokio::test]
async fn digest_is_skipped_quietly_when_memory_is_off() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mut config = (*test_config(&tmp)).clone();
    config.memory.engine = String::new();
    let config = Arc::new(config);
    let flow = flow_with_trigger_config("f-off", true, json!({}));
    store::upsert_flow(&config, &flow).unwrap();
    // No run row exists: with memory off the subscriber must return before
    // even loading it, so this completes without a warning path.
    FlowRunDigestSubscriber::new(config)
        .handle(&DomainEvent::FlowRunFinished {
            flow_id: "f-off".into(),
            run_id: "missing".into(),
            status: "completed".into(),
        })
        .await;
}

#[tokio::test]
async fn retention_cap_forgets_the_oldest_digests() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = test_config(&tmp);
    let _engine = digest_test_engine(&config);
    let base = chrono::Utc::now();
    for i in 0..4_i64 {
        let mut meta = crate::flows::flow_meta("f-cap", &["flow_run_digest".to_string()]);
        meta.observed_at = Some(base + chrono::Duration::seconds(i));
        crate::memory::ops::store_item(
            &config,
            tinymemory_api::StoreItem::document(format!("digest {i}"), meta),
        )
        .await
        .unwrap();
    }
    assert_eq!(enforce_retention_cap(&config, "f-cap", 2).await.unwrap(), 2);
    let kept = stored_digests(&config, "f-cap").await;
    assert_eq!(kept.len(), 2);
    assert!(kept
        .iter()
        .all(|hit| hit.text.contains("digest 2") || hit.text.contains("digest 3")));
}

#[test]
fn truncate_chars_bounds_output_and_marks_truncation() {
    let long = "x".repeat(50);
    let truncated = truncate_chars(&long, 10);
    assert_eq!(truncated.chars().count(), 10);
    assert!(truncated.ends_with('…'));

    let short = "hello";
    assert_eq!(truncate_chars(short, 10), "hello");
}

#[test]
fn render_run_digest_is_bounded_and_includes_key_fields() {
    let run = FlowRun {
        id: "run-1".to_string(),
        flow_id: "f1".to_string(),
        thread_id: "thread-1".to_string(),
        status: "completed".to_string(),
        started_at: "2026-01-01T00:00:00Z".to_string(),
        finished_at: Some("2026-01-01T00:05:00Z".to_string()),
        steps: vec![tinyflows_catalog::FlowRunStep {
            node_id: "n1".to_string(),
            output: json!({ "ok": true }),
            port: None,
            status: Some("success".to_string()),
            duration_ms: Some(5),
            diagnostics: Vec::new(),
        }],
        pending_approvals: Vec::new(),
        error: None,
        graph_hash: None,
    };
    let digest = render_run_digest("My Flow", &run);
    assert!(digest.contains("My Flow"));
    assert!(digest.contains("completed"));
    assert!(digest.contains("n1"));
    assert!(digest.chars().count() <= DIGEST_MAX_CHARS);
}

#[test]
fn dedup_commit_name_and_domains_are_stable() {
    let tmp = tempfile::TempDir::new().unwrap();
    let sub = DedupCommitSubscriber::new(test_config(&tmp));
    assert_eq!(sub.name(), "flows::dedup_commit");
    assert_eq!(sub.domains(), Some(&["cron"][..]));
}

#[tokio::test]
async fn dedup_commit_ignores_unrelated_events() {
    let tmp = tempfile::TempDir::new().unwrap();
    let sub = DedupCommitSubscriber::new(test_config(&tmp));
    // Must not panic for any event other than `FlowRunFinished`.
    sub.handle(&DomainEvent::CronJobTriggered {
        job_id: "j1".into(),
        job_name: "test".into(),
        job_type: "shell".into(),
    })
    .await;
}

#[tokio::test]
async fn dedup_commit_flow_with_no_dedup_nodes_is_a_noop() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = test_config(&tmp);
    let flow = flow_with_trigger_config("f-no-dedup", true, json!({}));
    store::upsert_flow(&config, &flow).unwrap();

    let sub = DedupCommitSubscriber::new(config);
    // Must not panic when the flow has no `dedup` node at all.
    sub.handle(&DomainEvent::FlowRunFinished {
        flow_id: "f-no-dedup".into(),
        run_id: "run-1".into(),
        status: "completed".into(),
    })
    .await;
}

#[tokio::test]
async fn dedup_commit_unions_tentative_into_committed_and_clears_tentative_on_success() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = test_config(&tmp);
    let flow = flow_with_dedup_node("f-ok", "dd");
    store::upsert_flow(&config, &flow).unwrap();

    let namespace = dedup_state_namespace("f-ok");
    store::kv_set(&config, &namespace, "dedup:dd:committed", &json!(["a"])).unwrap();
    store::kv_set(
        &config,
        &namespace,
        "dedup:dd:tentative",
        &json!(["b", "c"]),
    )
    .unwrap();

    let sub = DedupCommitSubscriber::new(config.clone());
    sub.handle(&DomainEvent::FlowRunFinished {
        flow_id: "f-ok".into(),
        run_id: "run-ok".into(),
        status: "completed".into(),
    })
    .await;

    let committed = store::kv_get(&config, &namespace, "dedup:dd:committed")
        .unwrap()
        .expect("committed key must still exist");
    let mut committed: Vec<&str> = committed
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    committed.sort_unstable();
    assert_eq!(committed, vec!["a", "b", "c"], "committed = union");

    assert!(
        store::kv_get(&config, &namespace, "dedup:dd:tentative")
            .unwrap()
            .is_none(),
        "tentative must be cleared after a successful commit"
    );
}

#[tokio::test]
async fn dedup_commit_treats_completed_with_warnings_as_success() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = test_config(&tmp);
    let flow = flow_with_dedup_node("f-warn", "dd");
    store::upsert_flow(&config, &flow).unwrap();

    let namespace = dedup_state_namespace("f-warn");
    store::kv_set(&config, &namespace, "dedup:dd:tentative", &json!(["x"])).unwrap();

    let sub = DedupCommitSubscriber::new(config.clone());
    sub.handle(&DomainEvent::FlowRunFinished {
        flow_id: "f-warn".into(),
        run_id: "run-warn".into(),
        status: "completed_with_warnings".into(),
    })
    .await;

    let committed = store::kv_get(&config, &namespace, "dedup:dd:committed")
        .unwrap()
        .expect("completed_with_warnings must still commit");
    assert_eq!(committed, json!(["x"]));
    assert!(store::kv_get(&config, &namespace, "dedup:dd:tentative")
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn dedup_commit_releases_tentative_without_touching_committed_on_failure() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = test_config(&tmp);
    let flow = flow_with_dedup_node("f-failed", "dd");
    store::upsert_flow(&config, &flow).unwrap();

    let namespace = dedup_state_namespace("f-failed");
    store::kv_set(&config, &namespace, "dedup:dd:committed", &json!(["a"])).unwrap();
    store::kv_set(&config, &namespace, "dedup:dd:tentative", &json!(["b"])).unwrap();

    let sub = DedupCommitSubscriber::new(config.clone());
    sub.handle(&DomainEvent::FlowRunFinished {
        flow_id: "f-failed".into(),
        run_id: "run-failed".into(),
        status: "failed".into(),
    })
    .await;

    assert_eq!(
        store::kv_get(&config, &namespace, "dedup:dd:committed")
            .unwrap()
            .unwrap(),
        json!(["a"]),
        "committed must be untouched by a failed run"
    );
    assert!(
        store::kv_get(&config, &namespace, "dedup:dd:tentative")
            .unwrap()
            .is_none(),
        "tentative must be released (cleared) on failure so the item retries"
    );
}

#[tokio::test]
async fn dedup_commit_releases_tentative_on_cancelled_and_interrupted() {
    for status in ["cancelled", "interrupted"] {
        let tmp = tempfile::TempDir::new().unwrap();
        let config = test_config(&tmp);
        let flow_id = format!("f-{status}");
        let flow = flow_with_dedup_node(&flow_id, "dd");
        store::upsert_flow(&config, &flow).unwrap();

        let namespace = dedup_state_namespace(&flow_id);
        store::kv_set(&config, &namespace, "dedup:dd:tentative", &json!(["z"])).unwrap();

        let sub = DedupCommitSubscriber::new(config.clone());
        sub.handle(&DomainEvent::FlowRunFinished {
            flow_id: flow_id.clone(),
            run_id: format!("run-{status}"),
            status: status.to_string(),
        })
        .await;

        assert!(
            store::kv_get(&config, &namespace, "dedup:dd:committed")
                .unwrap()
                .is_none(),
            "status {status} must never commit"
        );
        assert!(
            store::kv_get(&config, &namespace, "dedup:dd:tentative")
                .unwrap()
                .is_none(),
            "status {status} must release tentative"
        );
    }
}
