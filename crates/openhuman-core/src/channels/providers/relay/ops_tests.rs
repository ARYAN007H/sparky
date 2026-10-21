use super::super::channel::CHANNEL_OUTBOUND_EVENT;
use super::*;
use crate::agent::host_agents::{self, HostAgent, HostAgentResolver};
use crate::agent::turn_origin::AgentTurnOrigin;
use crate::core::runtime::{ContextOverlay, CoreContext, DomainSet};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tinytools::{PermissionLevel, Tool, ToolResult};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

#[tokio::test]
async fn relayed_thread_and_external_authority_survive_scoped_spawn() {
    let tmp = tempfile::tempdir().unwrap();
    let context = CoreContext::for_test(DomainSet::full(), None).derive_with(
        ContextOverlay::new(config(&tmp), DomainSet::full(), Default::default())
            .session_agent("relay-agent"),
    );
    let origin = AgentTurnOrigin::ExternalChannel {
        channel: "telegram".into(),
        sender: Some("42".into()),
        sender_name: None,
        reply_target: "-100".into(),
        message_id: "spawned-turn".into(),
        history_key: None,
        channel_session: None,
    };
    let thread = params("spawned-turn", "hello").thread_id();
    CoreContext::scope(context, async {
        CoreContext::scope(
            CoreContext::scoped()
                .unwrap()
                .derive_channel_turn(Some(thread.clone()), None),
            async {
                crate::agent::turn_origin::with_origin(origin, async {
                    let (child_thread, child_agent, child_origin) =
                        crate::core::runtime::DispatchCtx::ambient()
                            .spawn(async {
                                (
                                    CoreContext::current_channel_thread(),
                                    CoreContext::scoped()
                                        .and_then(|ctx| ctx.session_agent().map(str::to_owned)),
                                    crate::agent::turn_origin::current(),
                                )
                            })
                            .await
                            .unwrap();
                    assert_eq!(child_thread.as_deref(), Some(thread.as_str()));
                    assert_eq!(child_agent.as_deref(), Some("relay-agent"));
                    assert!(matches!(
                        child_origin,
                        Some(AgentTurnOrigin::ExternalChannel { channel, sender, .. })
                            if channel == "telegram" && sender.as_deref() == Some("42")
                    ));
                })
                .await;
            },
        )
        .await;
        assert!(CoreContext::current_channel_thread().is_none());
        assert!(CoreContext::current_turn_origin().is_none());
    })
    .await;
}

fn config(tmp: &tempfile::TempDir) -> Config {
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let mut config = Config {
        workspace_dir: workspace.clone(),
        action_dir: workspace,
        config_path: tmp.path().join("config.toml"),
        ..Config::default()
    };
    config.local_ai.runtime_enabled = false;
    config.memory.conversations.enabled = false;
    config.agent.session_dual_write = false;
    config.agent.session_shadow_reads = false;
    config
}

fn params(message_id: &str, text: &str) -> RelayInboundParams {
    serde_json::from_value(json!({
        "channel": "telegram",
        "chat_id": "-100",
        "sender_id": "42",
        "sender_name": "Ada",
        "message_id": message_id,
        "text": text,
        "client_id": "gw-ops-test",
    }))
    .expect("params")
}

/// What the actual SessionHost tool execution saw.
#[derive(Default)]
struct Seen {
    turns: AtomicUsize,
    origins: Mutex<Vec<Option<AgentTurnOrigin>>>,
    threads: Mutex<Vec<Option<String>>>,
    busy: Mutex<Vec<bool>>,
}

struct RelayProbe {
    dispatch_ctx: crate::core::runtime::DispatchCtx,
    thread_id: String,
    seen: Arc<Seen>,
}

#[async_trait::async_trait]
impl Tool for RelayProbe {
    fn name(&self) -> &str {
        "relay_probe"
    }
    fn description(&self) -> &str {
        "Record trusted relay turn context."
    }
    fn parameters_schema(&self) -> Value {
        json!({"type":"object","properties":{}})
    }
    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::ReadOnly
    }
    async fn execute(&self, _args: Value) -> anyhow::Result<ToolResult> {
        self.seen.turns.fetch_add(1, Ordering::SeqCst);
        self.seen
            .origins
            .lock()
            .unwrap()
            .push(CoreContext::current_turn_origin());
        self.seen
            .threads
            .lock()
            .unwrap()
            .push(CoreContext::current_channel_thread());
        self.seen
            .busy
            .lock()
            .unwrap()
            .push(crate::agent::orchestration::busy_guard::is_busy(
                &self.dispatch_ctx,
                &self.thread_id,
            ));
        Ok(ToolResult::success("relay context recorded"))
    }
}

struct RelayAgentResolver {
    config: Config,
    context: Arc<CoreContext>,
    dispatch_ctx: crate::core::runtime::DispatchCtx,
    seen: Arc<Seen>,
    thread_id: String,
}

impl HostAgentResolver for RelayAgentResolver {
    fn resolve(&self, agent_id: &str) -> Option<HostAgent> {
        (agent_id == "relay-agent").then(|| {
            let mut definition =
                crate::agent::harness::host_definition::AgentDefinitionRegistry::builtins_only()
                    .get("orchestrator")
                    .cloned()
                    .expect("the orchestrator is built in");
            definition.id = agent_id.to_owned();
            definition.subagents.clear();
            definition.system_prompt =
                tinyagents_definition::PromptSource::Inline("Relay integration test".into());
            definition.tools = tinyagents_definition::ToolScope::Named(vec!["relay_probe".into()]);
            let dispatch_ctx = self.dispatch_ctx.clone();
            let thread_id = self.thread_id.clone();
            let seen = Arc::clone(&self.seen);
            HostAgent {
                definition,
                config: self.config.clone(),
                host_tools: Some(Arc::new(move |_turn| {
                    crate::agent::HostTurnTools::advertised(vec![Box::new(RelayProbe {
                        dispatch_ctx: dispatch_ctx.clone(),
                        thread_id: thread_id.clone(),
                        seen: Arc::clone(&seen),
                    })])
                })),
                hooks: Default::default(),
                context: Arc::clone(&self.context),
            }
        })
    }
}

fn tool_call() -> Value {
    json!({
        "id":"relay-tool-call", "object":"chat.completion", "created":1,"model":"fixture",
        "choices":[{"index":0,"finish_reason":"tool_calls","message":{"role":"assistant","content":null,
            "tool_calls":[{"id":"call_relay_probe","type":"function","function":{"name":"relay_probe","arguments":"{}"}}]}}],
        "usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}
    })
}

fn answer(text: &str) -> Value {
    json!({
        "id":"relay-answer", "object":"chat.completion", "created":1,"model":"fixture",
        "choices":[{"index":0,"finish_reason":"stop","message":{"role":"assistant","content":text}}],
        "usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}
    })
}

struct RelayResponses(AtomicUsize);

impl Respond for RelayResponses {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        let response = match self.0.fetch_add(1, Ordering::SeqCst) {
            0 | 2 => tool_call(),
            1 => answer("answer 1"),
            _ => answer("answer 2"),
        };
        ResponseTemplate::new(200).set_body_json(response)
    }
}

async fn relay_provider() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(RelayResponses(AtomicUsize::new(0)))
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn a_relayed_turn_runs_as_an_external_channel_and_keeps_its_thread() {
    let _slot = host_agents::tests::lock();
    let tmp = tempfile::tempdir().unwrap();
    let server = relay_provider().await;
    let mut config = config(&tmp);
    config
        .agent
        .channel_agents
        .insert("telegram".into(), "relay-agent".into());
    config.default_model = Some("fixture".into());
    let route = crate::config::schema::EphemeralRoute::from_params(
        Some(format!("{}/v1", server.uri())),
        Some("sk-fixture".into()),
    )
    .unwrap();
    crate::config::schema::ephemeral_route::apply(&mut config, route);
    let core_context = CoreContext::for_test(DomainSet::full(), None).derive_with(
        ContextOverlay::new(config.clone(), DomainSet::full(), Default::default())
            .session_agent("relay-agent"),
    );
    let dispatch_ctx = crate::core::runtime::DispatchCtx::new(core_context);
    let seen = Arc::new(Seen::default());
    let relayed_thread = params("m1", "x").thread_id();
    let resolver: Arc<dyn HostAgentResolver> = Arc::new(RelayAgentResolver {
        config: config.clone(),
        context: dispatch_ctx.core().unwrap().clone(),
        dispatch_ctx: dispatch_ctx.clone(),
        seen: Arc::clone(&seen),
        thread_id: relayed_thread.clone(),
    });
    host_agents::install(Arc::clone(&resolver));
    let workspace = config.workspace_dir.clone();
    let mut events = crate::web_chat::subscribe_web_channel_events();

    for (id, text) in [("m1", "first question"), ("m2", "second question")] {
        let p = params(id, text);
        let thread = p.thread_id();
        assert_eq!(
            store::record_inbound(&dispatch_ctx, &workspace, &p, &thread).await,
            Ok(Recorded::New)
        );
        let replies = run_relay_turn(
            dispatch_ctx.clone(),
            config.clone(),
            p,
            thread,
            format!("req-{id}"),
        )
        .await;
        assert_eq!(
            replies,
            vec![format!("answer {}", if id == "m1" { 1 } else { 2 })]
        );
    }

    assert!(host_agents::clear_if(&resolver));
    assert_eq!(
        seen.turns.load(Ordering::SeqCst),
        2,
        "one tool execution per agent turn"
    );
    assert_eq!(
        *seen.busy.lock().unwrap(),
        vec![true, true],
        "a relayed turn marks its thread busy, so a background result waits"
    );
    assert!(
        !crate::agent::orchestration::busy_guard::is_busy(
            &dispatch_ctx,
            &params("m1", "x").thread_id()
        ),
        "the thread is idle once the turn ends"
    );
    for origin in seen.origins.lock().unwrap().iter() {
        match origin {
            Some(AgentTurnOrigin::ExternalChannel {
                channel, sender, ..
            }) => {
                assert_eq!(channel, "telegram");
                assert_eq!(sender.as_deref(), Some("42"));
            }
            other => panic!("relayed turns run as an external channel, got {other:?}"),
        }
    }
    assert_eq!(
        *seen.threads.lock().unwrap(),
        vec![Some(relayed_thread.clone()); 2]
    );

    let requests = server.received_requests().await.unwrap();
    let chats: Vec<Value> = requests
        .iter()
        .filter(|request| request.url.path().ends_with("/chat/completions"))
        .map(|request| serde_json::from_slice(&request.body).unwrap())
        .collect();
    assert!(chats.len() >= 4, "{chats:#?}");
    let serialized = serde_json::to_string(&chats[2..]).unwrap();
    assert!(serialized.contains("first question"), "{serialized}");
    assert!(serialized.contains("answer 1"), "{serialized}");
    assert!(serialized.contains("second question"), "{serialized}");

    let thread = params("m1", "x").thread_id();
    let messages =
        crate::threads::store::get_messages(&dispatch_ctx, workspace.clone(), &thread).unwrap();
    let ids: Vec<&str> = messages.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["user:m1", "assistant:m1", "user:m2", "assistant:m2"]
    );
    assert_eq!(messages[1].content, "answer 1");

    let mut outbound = Vec::new();
    while let Ok(event) = events.try_recv() {
        if event.context().client_id == "gw-ops-test" {
            outbound.push(event);
        }
    }
    assert_eq!(outbound.len(), 2, "one channel_outbound per reply");
    assert!(
        outbound
            .iter()
            .all(|event| event.event_name() == CHANNEL_OUTBOUND_EVENT)
    );
    assert!(
        outbound
            .iter()
            .all(|event| event.context().thread_id == thread)
    );
    assert_eq!(outbound[0].context().request_id, "req-m1");
    assert!(matches!(
        outbound[0].wire_event(),
        crate::web_chat::WebChannelEvent::ChannelOutbound {
            full_response: Some(text),
            structured: Some(structured),
            ..
        } if text == "answer 1" && structured["chat_id"] == "-100"
    ));
}

#[tokio::test]
async fn invalid_or_repeated_messages_start_no_turn() {
    let tmp = tempfile::tempdir().unwrap();
    let config = config(&tmp);
    let workspace = config.workspace_dir.clone();
    let ctx = CoreContext::for_test(DomainSet::full(), Some(workspace.clone())).derive_with(
        ContextOverlay::new(config, DomainSet::full(), Default::default()),
    );

    {
        let scope_ctx = crate::core::runtime::DispatchCtx::new(ctx);

        let mut bad = params("m1", "hello");
        bad.chat_id = "a/b".into();
        let err = channel_relay_inbound(&scope_ctx, bad)
            .await
            .expect_err("invalid");
        assert!(err.contains("chat_id"), "{err}");

        let p = params("m1", "hello");
        let thread = p.thread_id();
        store::record_inbound(&scope_ctx, &workspace, &p, &thread)
            .await
            .unwrap();
        let outcome = channel_relay_inbound(&scope_ctx, p)
            .await
            .expect("duplicate is not an error");
        let value = outcome.into_cli_compatible_json().unwrap();
        let text = value.to_string();
        assert!(text.contains("\"duplicate\":true"), "{text}");
        assert!(text.contains(&thread), "{text}");
    };
}

#[tokio::test]
async fn relay_preflight_failure_uses_trusted_channel_locale_without_running_agent() {
    let agent_runs = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed_runs = Arc::clone(&agent_runs);
    let _bus = mock_agent_run_turn(move |_request: AgentTurnRequest| {
        observed_runs.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        async { Ok(AgentTurnResponse::new("unexpected answer")) }
    })
    .await;

    let tmp = tempfile::tempdir().unwrap();
    let mut config = config(&tmp);
    config.modules.enabled = false;
    config.channels_locale = Some("es".into());
    let workspace = config.workspace_dir.clone();
    let params = params("preflight-error", "hello");
    let thread_id = params.thread_id();
    let dispatch = crate::core::runtime::DispatchCtx::ambient();
    assert_eq!(
        store::record_inbound(&dispatch, &workspace, &params, &thread_id).await,
        Ok(Recorded::New)
    );

    let replies = run_relay_turn(
        dispatch,
        config,
        params,
        thread_id,
        "request-preflight-error".into(),
    )
    .await;

    assert_eq!(agent_runs.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(replies, vec!["Algo salió mal. Inténtalo de nuevo."]);
    assert!(!replies[0].contains("COMPONENT_DISABLED"));
}
