//! Profile-owned host tools and a shared memory engine stay within each tenant.
//! One runtime and one test: SaaS operating mode is process-wide.

mod common;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::{
    chat_completion, chat_requests, runtime_without_master_key, tool_call_completion, tool_names,
    tool_results, PointedTransport,
};
use openhuman_embed::memory::api::conformance::ReferenceEngine;
use openhuman_embed::memory::api::{LearningKind, MemoryEngine, MemoryMeta, StoreItem};
use openhuman_embed::profiles::ProfileCredentialKind;
use openhuman_embed::seams::ToolHookDecision;
use openhuman_embed::{
    HostTurnTools, PermissionLevel, ProfileRuntime, RelayMessage, SaasConfig, Tool, ToolResult,
    ToolScopeSpec,
};
use serde_json::{json, Value};
use wiremock::{Mock, Request, Respond, ResponseTemplate};

struct TenantTool {
    name: &'static str,
    calls: Arc<Mutex<Vec<String>>>,
}

#[async_trait::async_trait]
impl Tool for TenantTool {
    fn name(&self) -> &str {
        self.name
    }
    fn description(&self) -> &str {
        "Report the current profile"
    }
    fn parameters_schema(&self) -> Value {
        json!({"type":"object","properties":{}})
    }
    async fn execute(&self, _: Value) -> anyhow::Result<ToolResult> {
        let tenant = openhuman_core::core::runtime::current_tenant()?;
        let profile = tenant.profile.expect("profile tool always has a tenant");
        match openhuman_embed::current_turn_origin().expect("a host turn has its original route") {
            openhuman_embed::AgentTurnOrigin::WebChat { thread_id, .. } => {
                assert_eq!(thread_id, "t1")
            }
            openhuman_embed::AgentTurnOrigin::ExternalChannel {
                channel,
                sender,
                reply_target,
                message_id,
                ..
            } => {
                assert_eq!(channel, "telegram");
                assert_eq!(sender.as_deref(), Some("sender"));
                assert_eq!(reply_target, "chat");
                assert_eq!(message_id, "message");
            }
            origin => panic!("unexpected turn origin: {origin:?}"),
        }
        self.calls
            .lock()
            .unwrap()
            .push(format!("{}:{profile}", self.name));
        Ok(ToolResult::success(profile))
    }
}

struct CallTools;
impl Respond for CallTools {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        let completion = if !tool_results(request).is_empty()
            || !tool_names(request)
                .iter()
                .any(|name| name == "tenant_probe")
        {
            chat_completion("done")
        } else {
            let mut completion = tool_call_completion("tenant_probe", "{}");
            let names = [
                "tenant_probe",
                "hidden",
                "shell",
                "delegate",
                "curl",
                "install_workflow_from_url",
                "http_request",
                "run_workflow",
                "delegate_to",
            ];
            completion["choices"][0]["message"]["tool_calls"] = json!(names.iter().enumerate().map(|(i, name)| json!({"id":format!("call_{i}"),"type":"function","function":{"name":name,"arguments":"{}"}})).collect::<Vec<_>>());
            let message = common::last_user_message(&body);
            let user = if message.contains("alice") {
                "alice"
            } else {
                "bob"
            };
            completion["choices"][0]["message"]["tool_calls"].as_array_mut().unwrap().push(json!({
                "id":"native-memory", "type":"function", "function":{"name":"memory", "arguments":json!({"action":"learn", "text":format!("{user}-native-learning")}).to_string()}
            }));
            completion
        };
        if body.get("stream").and_then(Value::as_bool) != Some(true) {
            return ResponseTemplate::new(200).set_body_json(completion);
        }
        let mut delta = completion["choices"][0]["message"].clone();
        if let Some(calls) = delta["tool_calls"].as_array_mut() {
            for (index, call) in calls.iter_mut().enumerate() {
                call["index"] = json!(index);
            }
        }
        let chunk = json!({"id":"fixture", "object":"chat.completion.chunk", "created":1700000000_u64, "model":"fixture", "choices":[{"index":0,"delta":delta,"finish_reason":null}]});
        let done = json!({"id":"fixture", "object":"chat.completion.chunk", "created":1700000000_u64, "model":"fixture", "choices":[{"index":0,"delta":{},"finish_reason":completion["choices"][0]["finish_reason"]}], "usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}});
        ResponseTemplate::new(200).set_body_raw(
            format!("data: {chunk}\n\ndata: {done}\n\ndata: [DONE]\n\n"),
            "text/event-stream",
        )
    }
}

#[test]
fn chat_and_relay_use_only_selected_tools_inside_their_profile() {
    runtime_without_master_key().block_on(async {
        tokio::spawn(scenario()).await.unwrap();
    });
}

async fn scenario() {
    let inference = wiremock::MockServer::start().await;
    Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path_regex(r"chat/completions$"))
        .respond_with(CallTools)
        .with_priority(1)
        .mount(&inference)
        .await;
    Mock::given(wiremock::matchers::any())
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success":true,"data":[]})))
        .with_priority(10)
        .mount(&inference)
        .await;
    PointedTransport::install(&inference.uri());
    let root = tempfile::tempdir().unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let factory_calls = calls.clone();
    let sessions = Arc::new(Mutex::new(Vec::new()));
    let factory_sessions = sessions.clone();
    let engine = Arc::new(ReferenceEngine::new());
    let runtime = ProfileRuntime::builder(SaasConfig::new(root.path()))
        .tools(move |turn| {
            let profile = openhuman_core::core::runtime::current_tenant()
                .unwrap()
                .profile
                .unwrap();
            factory_sessions
                .lock()
                .unwrap()
                .push((profile, turn.session_id().map(str::to_owned)));
            HostTurnTools::advertised(
                [
                    "tenant_probe",
                    "hidden",
                    "shell",
                    "delegate",
                    "curl",
                    "install_workflow_from_url",
                    "http_request",
                    "run_workflow",
                    "delegate_to",
                ]
                .into_iter()
                .map(|name| {
                    Box::new(TenantTool {
                        name,
                        calls: factory_calls.clone(),
                    }) as Box<dyn Tool>
                })
                .collect(),
            )
        })
        .tool_scope(ToolScopeSpec::Named(vec![
            "tenant_probe".into(),
            "memory".into(),
            "shell".into(),
            "delegate".into(),
            "curl".into(),
            "install_workflow_from_url".into(),
            "http_request".into(),
            "run_workflow".into(),
            "delegate_to".into(),
        ]))
        .can_use_tool(|_| Box::pin(async { ToolHookDecision::Proceed }))
        .channel_permissions([
            ("cli", PermissionLevel::Execute),
            ("web_channel", PermissionLevel::Execute),
            ("telegram", PermissionLevel::Execute),
        ])
        .memory_engine(engine.clone())
        .build()
        .await
        .unwrap();
    let mut handles = Vec::new();
    for user in ["alice", "bob"] {
        let provisioned = runtime.provision(user).await.unwrap();
        runtime
            .set_credential(
                &provisioned.profile_id,
                ProfileCredentialKind::ApiKey,
                "fixture-key",
            )
            .await
            .unwrap();
        let handle = runtime.open(user).await.unwrap();
        assert!(handle.context().embedder_config().unwrap().autonomy.enabled);
        assert_eq!(
            handle
                .context()
                .embedder_config()
                .unwrap()
                .memory
                .root
                .as_deref(),
            Some(format!("user:{user}").as_str())
        );
        tokio::time::timeout(
            Duration::from_secs(30),
            handle.chat(
                "t1",
                &format!("remember {user}-native-learning and call tenant_probe"),
            ),
        )
        .await
        .unwrap()
        .unwrap();
        handles.push(handle);
    }
    for handle in &handles {
        let mut events = handle.events();
        let accepted = handle
            .relay_inbound(RelayMessage::new(
                "telegram",
                "chat",
                "sender",
                "message",
                &format!("{}: call tenant_probe", handle.id()),
            ))
            .await
            .unwrap();
        assert!(accepted.accepted);
        tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                let event = events.recv().await.unwrap();
                if event.event == "channel_outbound" {
                    break;
                }
            }
        })
        .await
        .unwrap();
    }
    assert_eq!(
        *calls.lock().unwrap(),
        [
            "tenant_probe:alice",
            "tenant_probe:bob",
            "tenant_probe:alice",
            "tenant_probe:bob"
        ]
    );
    assert_eq!(
        *sessions.lock().unwrap(),
        [
            ("alice".into(), Some("t1".into())),
            ("bob".into(), Some("t1".into())),
            ("alice".into(), Some("channel:telegram/sender/chat".into())),
            ("bob".into(), Some("channel:telegram/sender/chat".into())),
        ]
    );
    // Persistent hooks fail closed if reused under a different profile or
    // after the owning lease is fenced, before the host callback is consulted.
    let hook_context = serde_json::from_value(json!({"event":"PreToolUse", "call_id":"fixture", "tool_name":"tenant_probe", "arguments":{}, "success":null, "duration_ms":null})).unwrap();
    let alice_hook = handles[0]
        .context()
        .host_overrides()
        .unwrap()
        .tool_hooks()
        .into_iter()
        .find(|hook| hook.name() == "profile.can_use_tool")
        .unwrap();
    assert!(matches!(
        handles[0]
            .scope(alice_hook.before_tool_decision(&hook_context))
            .await,
        ToolHookDecision::Proceed
    ));
    assert!(handles[1]
        .scope(alice_hook.before_tool_decision(&hook_context))
        .await
        .is_deny());
    let no_callback = Arc::new(
        openhuman_core::profiles::capabilities::ProfileCapabilities::new(
            None,
            Some(["tenant_probe".to_string()].into_iter().collect()),
            None,
            Default::default(),
            Vec::new(),
            None,
        ),
    );
    let alice_profile = handles[0]
        .scope(async { openhuman_core::profiles::current().unwrap() })
        .await;
    let closed_hook = no_callback.hook(handles[0].id(), alice_profile.lease_fence().clone());
    assert!(handles[0]
        .scope(closed_hook.before_tool_decision(&hook_context))
        .await
        .is_deny());
    let requests = chat_requests(&inference).await;
    assert!(requests
        .iter()
        .any(|request| !tool_names(request).is_empty()));
    for request in requests {
        let mut names = tool_names(&request);
        names.sort();
        if !names.is_empty() {
            assert_eq!(
                names,
                ["memory", "tenant_probe"],
                "the model sees the selected belt exactly"
            );
        }
    }
    // Shared backend contents are independent of the actor: actual contract
    // requests, including a malicious global-root filter, must stay confined.
    for user in ["alice", "bob"] {
        engine
            .store(StoreItem::learning(
                format!("{user}-secret"),
                LearningKind::Fact,
                0.9,
                MemoryMeta {
                    namespace: format!("user:{user}").parse().unwrap(),
                    ..Default::default()
                },
            ))
            .await
            .unwrap();
    }
    for handle in &handles {
        let response: Value = tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                let response: Value = handle
                    .call(
                        "openhuman.memory_items_list",
                        json!({"filter":{"reach":{"at":"","descendants":true,"inherit":true}}}),
                    )
                    .await
                    .unwrap();
                if response
                    .to_string()
                    .contains(&format!("{}-native-learning", handle.id()))
                {
                    break response;
                }
                // Native writes acknowledge their durable local queue. Await the
                // queued worker's store before checking the real engine contents.
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("the native memory write reaches the shared engine");
        let text = response.to_string();
        assert!(
            text.contains(&format!("{}-secret", handle.id())),
            "{response}"
        );
        assert!(
            text.contains(&format!("{}-native-learning", handle.id())),
            "native memory mutation was persisted: {response}"
        );
        let other = if handle.id().as_str() == "alice" {
            "bob"
        } else {
            "alice"
        };
        assert!(!text.contains(&format!("{other}-secret")), "{response}");
    }
    alice_profile.lease_fence().fence();
    assert!(handles[0]
        .scope(alice_hook.before_tool_decision(&hook_context))
        .await
        .is_deny());
    runtime.shutdown().await;
}
