use super::*;
use crate::agent_registry::host_definition::AgentDefinitionRegistry;
use crate::core::events::DomainEvent;
use serde_json::json;
use std::sync::{Arc, Mutex};
use tinyinference_llm::model::{
    ChatModel, ModelProfile, ModelRequest, ModelResponse, ModelStream, ModelStreamItem,
};
use tokio::time::{sleep, timeout, Duration};

struct TriageReplyModel {
    requests: Arc<Mutex<Vec<ModelRequest>>>,
}

#[async_trait::async_trait]
impl ChatModel<()> for TriageReplyModel {
    fn profile(&self) -> Option<&ModelProfile> {
        static PROFILE: std::sync::LazyLock<ModelProfile> =
            std::sync::LazyLock::new(|| ModelProfile {
                provider: Some("triage-escalation-test".into()),
                tool_calling: true,
                parallel_tool_calls: true,
                streaming: true,
                ..ModelProfile::default()
            });
        Some(&PROFILE)
    }

    async fn invoke(
        &self,
        _state: &(),
        request: ModelRequest,
    ) -> tinyinference_llm::Result<ModelResponse> {
        self.requests.lock().unwrap().push(request);
        Ok(ModelResponse::assistant("handled inline"))
    }

    async fn stream(
        &self,
        _state: &(),
        request: ModelRequest,
    ) -> tinyinference_llm::Result<ModelStream> {
        self.requests.lock().unwrap().push(request);
        let response = ModelResponse::assistant("handled inline");
        Ok(ModelStream::new(Box::pin(futures::stream::iter([
            ModelStreamItem::Started,
            ModelStreamItem::MessageDelta(tinyinference_llm::message::MessageDelta::text(
                "handled inline",
            )),
            ModelStreamItem::Completed(response),
        ]))))
    }
}

fn triage_test_model(
    requests: Arc<Mutex<Vec<ModelRequest>>>,
    builds: Arc<Mutex<Vec<String>>>,
) -> crate::inference::registry::ChatModelEntry {
    crate::inference::registry::ChatModelEntry {
        build: Arc::new(move |request| {
            builds.lock().unwrap().push(format!(
                "role={} model_override={:?} default_model={:?} native_tool_calling={}",
                request.role,
                request.model_override,
                request.default_model,
                request.native_tool_calling
            ));
            Ok((
                Arc::new(TriageReplyModel {
                    requests: Arc::clone(&requests),
                }) as Arc<dyn ChatModel<()>>,
                "triage-test-model".to_string(),
            ))
        }),
        probe: None,
    }
}

#[tokio::test]
async fn react_dispatch_keeps_routing_without_creating_child_lifecycle_records() {
    let _events_guard = test_events_guard().await;
    let temp = tempfile::tempdir().expect("isolated triage workspace");
    let workspace = temp.path().join("workspace");
    let mut config = Config {
        workspace_dir: workspace.clone(),
        config_path: temp.path().join("config.toml"),
        action_dir: temp.path().join("actions"),
        ..Config::default()
    };
    config.default_model = Some(crate::config::MODEL_MANAGED_DEFAULT.to_string());

    let requests = Arc::new(Mutex::new(Vec::new()));
    let builds = Arc::new(Mutex::new(Vec::new()));
    let extensions = crate::core::slots::Extensions::new();
    crate::inference::registry::register_chat_model_in(
        &extensions,
        crate::inference::provider::factory::PROVIDER_OPENHUMAN,
        triage_test_model(Arc::clone(&requests), Arc::clone(&builds)),
    );
    let runtime = crate::core::runtime::CoreBuilder::new(crate::core::types::HostKind::Library)
        .services(crate::core::runtime::ServiceSet::none())
        .domains(crate::core::runtime::DomainSet::none())
        .config(config)
        .extensions(extensions)
        .build()
        .await
        .expect("isolated core runtime");
    let ctx = crate::core::runtime::DispatchCtx::new(Arc::clone(runtime.context()));
    let envelope = envelope("esc-react-inline");
    crate::core::bus::init().await.expect("bus init");

    runtime
        .run_in(
            Arc::clone(runtime.context()),
            crate::security::turn_origin::with_origin(
                crate::triage::local_trigger_origin(),
                apply_decision(
                    &ctx,
                    run_with_target(
                        TriageAction::React,
                        "trigger_reactor",
                        "handle this trigger inline",
                    ),
                    &envelope,
                ),
            ),
        )
        .await
        .expect("react escalation should complete");

    // Prove that the requested route reached the model before checking the
    // lifecycle boundary below, so a skipped dispatch cannot satisfy it.
    let requests = requests.lock().unwrap();
    let build_summaries = builds.lock().unwrap();
    eprintln!(
        "triage diagnostic: model_requests={} provider_builds={:?}",
        requests.len(),
        *build_summaries
    );
    eprintln!(
        "triage diagnostic: request_summaries={:?}",
        requests
            .iter()
            .map(|request| (
                request.messages.len(),
                request
                    .messages
                    .iter()
                    .any(|message| message.text().contains("handle this trigger inline")),
            ))
            .collect::<Vec<_>>()
    );
    assert_eq!(requests.len(), 1, "triage should make one model request");
    assert!(
        requests[0]
            .messages
            .iter()
            .any(|message| message.text().contains("handle this trigger inline")),
        "the target agent should receive the triage prompt"
    );
    drop(requests);
    let captured = events::test_events_for_external_id("esc-react-inline");
    assert!(captured.iter().any(|event| matches!(
        event,
        DomainEvent::TriggerEscalated {
            external_id,
            target_agent,
            ..
        } if external_id == "esc-react-inline" && target_agent == "trigger_reactor"
    )));
    assert!(!captured.iter().any(|event| matches!(
        event,
        DomainEvent::TriggerEscalationFailed {
            external_id,
            ..
        } if external_id == "esc-react-inline"
    )));

    let child_lifecycle = runtime
        .run_in(Arc::clone(runtime.context()), async {
            crate::storage::state_kv::StateKv::open(&workspace, "subagents", "lifecycle-neutral")
                .expect("open scoped neutral subagent lifecycle records")
                .list::<serde_json::Value>("record/")
        })
        .await
        .expect("read scoped neutral subagent lifecycle records");
    assert!(
        child_lifecycle.is_empty(),
        "a triage one-shot turn must not persist a child subagent lifecycle: {child_lifecycle:?}"
    );
}

/// The triage parent is the shared root context with one override: nested
/// spawns scoped to the dispatched target. Guards the #4369 collapse against
/// drift from `build_root_parent`.
#[tokio::test]
async fn build_triage_parent_scopes_allowed_subagents_to_target() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = Config {
        workspace_dir: dir.path().to_path_buf(),
        ..Config::default()
    };

    let _ = AgentDefinitionRegistry::init_global_builtins();
    let ctx = build_triage_parent(
        &crate::core::runtime::DispatchCtx::ambient(),
        &config,
        "researcher",
    )
    .await
    .expect("build triage parent");

    assert_eq!(ctx.agent_definition_id, "triage");
    assert_eq!(ctx.channel, "triage");
    assert!(
        ctx.session_id.starts_with("triage-"),
        "session_id namespaced by triage prefix, got {}",
        ctx.session_id
    );
    assert_eq!(
        ctx.allowed_subagent_ids,
        ["researcher".to_string()].into_iter().collect(),
        "nested spawns must be scoped to the single dispatched target"
    );
}

static TEST_EVENTS_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct TestEventsGuard(tokio::sync::MutexGuard<'static, ()>);

impl Drop for TestEventsGuard {
    fn drop(&mut self) {
        events::clear_test_events();
    }
}

async fn test_events_guard() -> TestEventsGuard {
    let guard = TEST_EVENTS_LOCK.lock().await;
    events::clear_test_events();
    TestEventsGuard(guard)
}

fn envelope(external_id: &str) -> TriggerEnvelope {
    TriggerEnvelope::from_composio(
        "gmail",
        "GMAIL_NEW_GMAIL_MESSAGE",
        "triage-escalation",
        external_id,
        json!({ "subject": "hello" }),
    )
}

fn run(action: TriageAction) -> TriageRun {
    TriageRun {
        decision: super::super::decision::TriageDecision {
            action,
            target_agent: None,
            prompt: None,
            reason: "because".into(),
        },
        used_local: false,
        latency_ms: 9,
        resolution_path: super::super::evaluator::TriageResolutionPath::Cloud,
    }
}

fn run_with_target(action: TriageAction, target_agent: &str, prompt: &str) -> TriageRun {
    TriageRun {
        decision: super::super::decision::TriageDecision {
            action,
            target_agent: Some(target_agent.into()),
            prompt: Some(prompt.into()),
            reason: "because".into(),
        },
        used_local: false,
        latency_ms: 9,
        resolution_path: super::super::evaluator::TriageResolutionPath::Cloud,
    }
}

async fn collect_trigger_events_until(
    external_id: &str,
    expected: impl Fn(&[DomainEvent]) -> bool,
) -> Vec<DomainEvent> {
    let external_id = external_id.to_string();
    timeout(Duration::from_secs(5), async {
        loop {
            let captured = events::test_events_for_external_id(&external_id);
            if expected(&captured) {
                return captured;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("expected triage event should arrive")
}

#[tokio::test]
async fn apply_decision_drop_only_publishes_evaluated() {
    let _events_guard = test_events_guard().await;
    let envelope = envelope("esc-drop");
    crate::core::bus::init().await.expect("bus init");
    let collect = tokio::spawn(collect_trigger_events_until("esc-drop", |events| {
        events.iter().any(|event| {
            matches!(
                event,
                DomainEvent::TriggerEvaluated {
                    decision,
                    external_id,
                    ..
                } if decision == "drop" && external_id == "esc-drop"
            )
        })
    }));

    apply_decision(
        &crate::core::runtime::DispatchCtx::ambient(),
        run(TriageAction::Drop),
        &envelope,
    )
    .await
    .expect("drop should not fail");

    let captured = collect.await.expect("event collector should not panic");
    assert!(captured.iter().any(|event| matches!(
        event,
        DomainEvent::TriggerEvaluated {
            decision,
            external_id,
            ..
        } if decision == "drop" && external_id == "esc-drop"
    )));
    assert!(!captured.iter().any(|event| matches!(
        event,
        DomainEvent::TriggerEscalated { external_id, .. }
            | DomainEvent::TriggerEscalationFailed { external_id, .. }
            if external_id == "esc-drop"
    )));
}

#[tokio::test]
async fn apply_decision_acknowledge_only_publishes_evaluated() {
    let _events_guard = test_events_guard().await;
    let envelope = envelope("esc-ack");
    crate::core::bus::init().await.expect("bus init");
    let collect = tokio::spawn(collect_trigger_events_until("esc-ack", |events| {
        events.iter().any(|event| {
            matches!(
                event,
                DomainEvent::TriggerEvaluated {
                    decision,
                    external_id,
                    ..
                } if decision == "acknowledge" && external_id == "esc-ack"
            )
        })
    }));

    apply_decision(
        &crate::core::runtime::DispatchCtx::ambient(),
        run(TriageAction::Acknowledge),
        &envelope,
    )
    .await
    .expect("acknowledge should not fail");

    let captured = collect.await.expect("event collector should not panic");
    assert!(captured.iter().any(|event| matches!(
        event,
        DomainEvent::TriggerEvaluated {
            decision,
            external_id,
            ..
        } if decision == "acknowledge" && external_id == "esc-ack"
    )));
    assert!(!captured.iter().any(|event| matches!(
        event,
        DomainEvent::TriggerEscalated { external_id, .. }
            | DomainEvent::TriggerEscalationFailed { external_id, .. }
            if external_id == "esc-ack"
    )));
}

#[tokio::test]
async fn apply_decision_react_failure_publishes_failed_event() {
    let _events_guard = test_events_guard().await;
    let envelope = envelope("esc-react-fail");
    crate::core::bus::init().await.expect("bus init");
    let _ = AgentDefinitionRegistry::init_global_builtins();
    let missing_target = format!("missing-agent-{}", uuid::Uuid::new_v4());
    let collect = tokio::spawn(collect_trigger_events_until("esc-react-fail", |events| {
        events.iter().any(|event| {
            matches!(
                event,
                DomainEvent::TriggerEvaluated {
                    decision,
                    external_id,
                    ..
                } if decision == "react" && external_id == "esc-react-fail"
            )
        }) && events.iter().any(|event| {
            matches!(
                event,
                DomainEvent::TriggerEscalationFailed { external_id, .. }
                    if external_id == "esc-react-fail"
            )
        })
    }));

    let result = apply_decision(
        &crate::core::runtime::DispatchCtx::ambient(),
        run_with_target(TriageAction::React, &missing_target, "handle this"),
        &envelope,
    )
    .await;
    if let Err(err) = result {
        assert!(err.to_string().contains(&missing_target));
    }

    let captured = collect.await.expect("event collector should not panic");
    assert!(captured.iter().any(|event| matches!(
        event,
        DomainEvent::TriggerEvaluated {
            decision,
            external_id,
            ..
        } if decision == "react" && external_id == "esc-react-fail"
    )));
    assert!(captured.iter().any(|event| matches!(
        event,
        DomainEvent::TriggerEscalationFailed { external_id, .. }
            if external_id == "esc-react-fail"
    )));
}

#[tokio::test]
async fn apply_decision_escalate_failure_publishes_failed_event() {
    let _events_guard = test_events_guard().await;
    let envelope = envelope("esc-escalate-fail");
    crate::core::bus::init().await.expect("bus init");
    let _ = AgentDefinitionRegistry::init_global_builtins();
    let missing_target = format!("missing-agent-{}", uuid::Uuid::new_v4());
    let collect = tokio::spawn(collect_trigger_events_until(
        "esc-escalate-fail",
        |events| {
            events.iter().any(|event| {
                matches!(
                    event,
                    DomainEvent::TriggerEvaluated {
                        decision,
                        external_id,
                        ..
                    } if decision == "escalate" && external_id == "esc-escalate-fail"
                )
            }) && events.iter().any(|event| {
                matches!(
                    event,
                    DomainEvent::TriggerEscalationFailed { external_id, .. }
                        if external_id == "esc-escalate-fail"
                )
            })
        },
    ));

    let result = apply_decision(
        &crate::core::runtime::DispatchCtx::ambient(),
        run_with_target(TriageAction::Escalate, &missing_target, "escalate this"),
        &envelope,
    )
    .await;
    if let Err(err) = result {
        assert!(err.to_string().contains(&missing_target));
    }

    let captured = collect.await.expect("event collector should not panic");
    assert!(captured.iter().any(|event| matches!(
        event,
        DomainEvent::TriggerEvaluated {
            decision,
            external_id,
            ..
        } if decision == "escalate" && external_id == "esc-escalate-fail"
    )));
    assert!(captured.iter().any(|event| matches!(
        event,
        DomainEvent::TriggerEscalationFailed { external_id, .. }
            if external_id == "esc-escalate-fail"
    )));
}

/// Only the composio path archives its input before triage. Naming a
/// trigger-history record for a webhook or cron acknowledge would send an
/// operator looking for a file that was never written.
#[test]
fn only_composio_claims_a_retained_input() {
    assert_eq!(
        retained_input_note(&TriggerSource::Composio {
            toolkit: "gmail".into(),
            trigger: "GMAIL_NEW_GMAIL_MESSAGE".into(),
        }),
        "trigger-history archive"
    );

    for source in [
        TriggerSource::Webhook {
            tunnel_id: "t".into(),
            method: "POST".into(),
            path: "/x".into(),
        },
        TriggerSource::Cron {
            job_id: "j".into(),
            job_name: "nightly".into(),
        },
        TriggerSource::WebviewIntegration {
            provider: "gmail".into(),
            account_id: "a".into(),
        },
    ] {
        assert_eq!(
            retained_input_note(&source),
            "none — verdict only",
            "{} has no archive to point at",
            source.slug()
        );
    }
}
