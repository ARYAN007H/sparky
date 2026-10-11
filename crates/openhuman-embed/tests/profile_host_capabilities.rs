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
        if !tool_results(request).is_empty() {
            return ResponseTemplate::new(200).set_body_json(chat_completion("done"));
        }
        let mut completion = tool_call_completion("tenant_probe", "{}");
        let names = [
            "tenant_probe",
            "hidden",
            "shell",
            "delegate",
            "curl",
            "install_workflow_from_url",
        ];
        completion["choices"][0]["message"]["tool_calls"] = json!(names.iter().enumerate().map(|(i, name)| json!({"id":format!("call_{i}"),"type":"function","function":{"name":name,"arguments":"{}"}})).collect::<Vec<_>>());
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        let message = common::last_user_message(&body);
        let user = if message.contains("alice") {
            "alice"
        } else {
            "bob"
        };
        completion["choices"][0]["message"]["tool_calls"].as_array_mut().unwrap().push(json!({
            "id":"native-memory", "type":"function", "function":{"name":"memory", "arguments":json!({"action":"learn", "text":format!("{user}-native-learning")}).to_string()}
        }));
        ResponseTemplate::new(200).set_body_json(completion)
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
        .and(wiremock::matchers::path("/v1/chat/completions"))
        .respond_with(CallTools)
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
        let response: Value = handle
            .call(
                "openhuman.memory_items_list",
                json!({"filter":{"reach":{"at":"","descendants":true,"inherit":true}}}),
            )
            .await
            .unwrap();
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
    runtime.shutdown().await;
}
