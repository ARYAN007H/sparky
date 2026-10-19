mod common;
use common::{chat_completion, chat_requests, offline_config, runtime, stub_backend};
use openhuman_embed::{
    Agent, AgentDefinitionSpec, AgentSpec, HostTools, HostTurnTools, Provider, Runtime, Tool,
    ToolScopeSpec, Workspace,
};
use std::sync::Arc;
use tinyagents_session::transcript::{SessionRef, TranscriptLocator};
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};
// Runtime is process-wide. Keep the guard on the synchronous test caller;
// it outlives block_on and the scoped Runtime/Agent teardown inside its future.
static RUNTIME_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct Marker(&'static str);
#[async_trait::async_trait]
impl Tool for Marker {
    fn exposure(&self) -> openhuman_embed::ToolExposure {
        openhuman_embed::ToolExposure::Deferred
    }
    fn name(&self) -> &str {
        self.0
    }
    fn description(&self) -> &str {
        "Send a message to the hive"
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({"type":"object", "properties":{"message":{"type":"string"}}, "required":["message"]})
    }
    async fn execute(&self, _: serde_json::Value) -> anyhow::Result<tinytools::ToolResult> {
        Ok(tinytools::ToolResult::success("accepted"))
    }
}

fn transcript_snapshot(agent: &Agent, session: &SessionRef) -> serde_json::Value {
    let transcripts =
        tinyagents_session::port::WorkspaceStores::open(agent.workspace_dir(), agent.id())
            .expect("the agent's durable workspace store")
            .transcripts();
    let handle = transcripts
        .read_session_transcript(session)
        .expect("the durable session transcript exists");
    let transcript = handle
        .read_session()
        .expect("the durable session transcript can be read")
        .expect("the durable session transcript has content");
    serde_json::json!({
        "meta": transcript.meta,
        "messages": transcript.messages,
        "tools": transcript.tools,
    })
}

#[test]
fn attached_tools_survive_clones_and_session_resume() {
    let _runtime = RUNTIME_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let collision = openhuman_core::agent::OpenHumanSessionHost::builder()
        .tools(vec![Box::new(Marker("mcp_fixture_send"))])
        .synthesized_tools(vec![Box::new(Marker("mcp_fixture_send"))])
        .permanent_tool_names(std::collections::HashSet::from(["mcp_fixture_send".into()]))
        .build()
        .err()
        .expect("a permanent source cannot shadow a synthesized MCP tool");
    assert!(collision
        .to_string()
        .contains("collision with synthesized tool"));
    runtime().block_on(async {
        tokio::spawn(async {
            let backend = stub_backend().await;
            let provider = wiremock::MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/v1/chat/completions"))
                .respond_with(ResponseTemplate::new(200).set_body_json(chat_completion("reply")))
                .mount(&provider)
                .await;
            let runtime = Runtime::builder()
                .config(offline_config())
                .workspace(Workspace::Ephemeral)
                .backend_url(backend.uri())
                .build()
                .await
                .unwrap();
            let agent = runtime
                .agent(
                    AgentSpec::new("attached")
                        .provider(
                            Provider::openai_compatible(
                                format!("{}/v1", provider.uri()),
                                "fixture",
                            )
                            .model("fixture"),
                        )
                        .definition(
                            AgentDefinitionSpec::new()
                                .system_prompt("FROZEN_HOST_PROMPT")
                                .tools(ToolScopeSpec::Named(vec![
                                    "shell".into(),
                                    "tool_search".into(),
                                ])),
                        ),
                )
                .unwrap();
            let original_config = agent.config().clone();
            assert!(agent
                .attach_tools("", Arc::new(|_| HostTurnTools::default()))
                .is_err());
            assert!(agent
                .attach_tools(
                    "builtin-collision",
                    Arc::new(|_| HostTurnTools::advertised(vec![Box::new(Marker("shell"))]))
                )
                .is_err());
            let peer = runtime.agent(AgentSpec::new("peer")).unwrap();
            assert!(!agent.same_agent(&peer));
            assert_eq!(agent.runtime_id(), peer.runtime_id());
            let clone = agent.clone();
            assert!(agent.same_agent(&clone));
            assert_eq!(agent.runtime_id(), clone.runtime_id());
            let factory: HostTools =
                Arc::new(|_| HostTurnTools::advertised(vec![Box::new(Marker("hivemind_message"))]));
            agent.attach_tools("tinyhivemind", factory.clone()).unwrap();
            clone.attach_tools("tinyhivemind", factory).unwrap();
            assert!(clone
                .attach_tools("tinyhivemind", Arc::new(|_| HostTurnTools::default()))
                .is_err());
            assert!(clone
                .attach_tools(
                    "collision",
                    Arc::new(|_| HostTurnTools::advertised(vec![Box::new(Marker(
                        "hivemind_message"
                    ))]))
                )
                .is_err());
            agent
                .turn("first user")
                .session("continuing")
                .send()
                .await
                .unwrap();
            clone
                .turn("second user")
                .session("continuing")
                .send()
                .await
                .unwrap();
            let continuing_session = SessionRef::scoped("continuing", agent.id());
            let transcript_store =
                tinyagents_session::port::WorkspaceStores::open(agent.workspace_dir(), agent.id())
                    .expect("the agent's durable workspace store");
            assert_eq!(
                transcript_store
                    .transcripts()
                    .session_chain(&continuing_session)
                    .len(),
                1
            );
            drop(transcript_store);
            let frozen_generation_zero = transcript_snapshot(&agent, &continuing_session);
            agent
                .attach_tools(
                    "second",
                    Arc::new(|_| {
                        HostTurnTools::advertised(vec![Box::new(Marker("hivemind_list"))])
                    }),
                )
                .unwrap();
            // A resumed session restores the exact tool prefix it committed.
            // The newly attached tool is available to sessions started after
            // this point, while `continuing` keeps the catalogue it began with.
            clone
                .turn("third user")
                .session("after-attach")
                .send()
                .await
                .unwrap();
            clone
                .turn("fourth user")
                .session("after-attach")
                .send()
                .await
                .unwrap();
            assert_eq!(agent.config().action_dir, original_config.action_dir);
            assert_eq!(
                transcript_snapshot(&agent, &continuing_session),
                frozen_generation_zero,
                "the continuing session must leave its durable generation unchanged"
            );
            let after_attach_session = SessionRef::scoped("after-attach", agent.id());
            let durable = transcript_snapshot(&agent, &after_attach_session);
            let durable_messages = durable["messages"]
                .as_array()
                .expect("the new session's durable messages");
            let durable_system = durable_messages
                .iter()
                .filter(|message| message["role"] == "system")
                .filter_map(|message| message["content"].as_str())
                .collect::<Vec<_>>();
            let has_frozen_prompt = durable_system
                .iter()
                .any(|content| content.contains("FROZEN_HOST_PROMPT"));
            let has_permanent_tools_message = durable_system.iter().any(|content| {
                content.contains("openhuman-permanent-tools")
                    && content.contains("hivemind_message")
                    && content.contains("hivemind_list")
            });
            if !has_frozen_prompt || !has_permanent_tools_message {
                eprintln!("durable system messages: {durable_system:#?}");
            }
            assert!(
                has_frozen_prompt,
                "the new session must persist its frozen prompt"
            );
            assert!(
                has_permanent_tools_message,
                "the new session must persist its permanent-tools system message"
            );
            let declared_tools = durable["tools"]
                .as_array()
                .expect("the new session's durable tool catalogue");
            let tool_names = declared_tools
                .iter()
                .filter_map(|tool| tool["name"].as_str())
                .collect::<Vec<_>>();
            if !tool_names.contains(&"hivemind_message") || !tool_names.contains(&"hivemind_list") {
                eprintln!("durable tool declarations: {declared_tools:#?}");
            }
            assert!(
                tool_names.contains(&"hivemind_message") && tool_names.contains(&"hivemind_list"),
                "the new session must persist the updated permanent-tool catalogue"
            );
            let sealed = frozen_generation_zero.to_string();
            assert!(
                sealed.contains("FROZEN_HOST_PROMPT"),
                "sealed prefix: {sealed}"
            );
            assert!(
                sealed.contains("hivemind_message"),
                "sealed catalogue: {sealed}"
            );
            assert!(
                !sealed.contains("hivemind_list"),
                "the original generation cannot contain the later attachment: {sealed}"
            );
            let requests = chat_requests(&provider).await;
            assert_eq!(requests.len(), 4);
            for (index, request) in requests.iter().enumerate() {
                let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
                let messages = body["messages"].as_array().unwrap();
                let system = messages
                    .iter()
                    .filter(|m| m["role"] == "system")
                    .map(|m| m["content"].as_str().unwrap_or(""))
                    .collect::<Vec<_>>()
                    .join("\n");
                assert!(system.contains("FROZEN_HOST_PROMPT"));
                assert_eq!(system.matches("hivemind_message").count(), 1, "{system}");
                let tools = body["tools"].as_array().unwrap();
                assert_eq!(
                    tools
                        .iter()
                        .filter(|t| t["function"]["name"] == "hivemind_message")
                        .count(),
                    1,
                    "request {index}: {body}"
                );
                if index >= 2 {
                    assert_eq!(system.matches("hivemind_list").count(), 1);
                    assert_eq!(
                        tools
                            .iter()
                            .filter(|t| t["function"]["name"] == "hivemind_list")
                            .count(),
                        1
                    );
                }
                if index == 1 {
                    assert!(messages.iter().any(|m| m["role"] == "user"
                        && m["content"].as_str().unwrap_or("").contains("first user")));
                }
                if index == 3 {
                    assert!(messages.iter().any(|m| m["role"] == "user"
                        && m["content"].as_str().unwrap_or("").contains("third user")));
                    assert!(
                        !messages.iter().any(|m| {
                            let content = m["content"].as_str().unwrap_or("");
                            content.contains("first user") || content.contains("second user")
                        }),
                        "a new session must not inherit the continuing session's history"
                    );
                }
                if index == 2 {
                    assert!(
                        !messages.iter().any(|m| {
                            m["content"].as_str().unwrap_or("").contains("first user")
                        }),
                        "a new session must start without the continuing session's history"
                    );
                }
            }
        })
        .await
        .unwrap();
    });
}

#[test]
fn attaching_before_first_turn_preserves_and_executes_native_tools() {
    let _runtime = RUNTIME_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    runtime().block_on(async {
        tokio::spawn(async {
            struct NativeCall(std::sync::atomic::AtomicUsize);
            impl wiremock::Respond for NativeCall {
                fn respond(&self, _: &wiremock::Request) -> ResponseTemplate {
                    let body = if self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                        serde_json::json!({
                            "id":"fresh-wildcard", "object":"chat.completion", "model":"fixture",
                            "choices":[{"index":0,"finish_reason":"tool_calls","message":{
                                "role":"assistant","content":null,"tool_calls":[{
                                    "id":"native-shell","type":"function","function":{
                                        "name":"shell","arguments":"{\"command\":\"printf native-wildcard-survived\"}"
                                    }
                                }]
                            }}]
                        })
                    } else {
                        chat_completion("native tool completed")
                    };
                    ResponseTemplate::new(200).set_body_json(body)
                }
            }
            let backend = stub_backend().await;
            let provider = wiremock::MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/v1/chat/completions"))
                .respond_with(NativeCall(std::sync::atomic::AtomicUsize::new(0)))
                .mount(&provider).await;
            let runtime = Runtime::builder()
                .config(offline_config()).workspace(Workspace::Ephemeral)
                .backend_url(backend.uri()).build().await.unwrap();
            let agent = runtime.agent(AgentSpec::new("fresh-attached")
                .provider(Provider::openai_compatible(format!("{}/v1",provider.uri()),"fixture").model("fixture"))
                .definition(AgentDefinitionSpec::new().system_prompt("ORIGINAL_FRESH_HOST_PROMPT")
                    .tools(ToolScopeSpec::Wildcard))).unwrap();
            let attachment: HostTools = Arc::new(|_| HostTurnTools::advertised(vec![Box::new(Marker("hivemind_message"))]));
            agent.attach_tools("hivemind", attachment).unwrap();
            agent.run("Run the native tool").await.unwrap();
            let requests = chat_requests(&provider).await;
            assert_eq!(requests.len(), 2);
            let first: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
            let names: Vec<_> = first["tools"].as_array().unwrap().iter()
                .filter_map(|tool| tool["function"]["name"].as_str()).collect();
            assert!(names.contains(&"shell"), "native wildcard tool missing: {names:?}");
            assert!(names.contains(&"hivemind_message"));
            let second: serde_json::Value = serde_json::from_slice(&requests[1].body).unwrap();
            assert!(second["messages"].as_array().unwrap().iter().any(|message|
                message["role"] == "tool" && message["content"].as_str().is_some_and(|text|
                    text.contains("native-wildcard-survived") && !text.contains("unknown tool"))),
                "the native executor must actually run: {second}");
            assert!(first["messages"].to_string().contains("ORIGINAL_FRESH_HOST_PROMPT"));
        }).await.unwrap();
    });
}

#[test]
fn permanent_metadata_requires_a_real_source() {
    let factory = HostTurnTools {
        permanent: std::collections::HashSet::from(["missing_source".into()]),
        ..Default::default()
    };
    assert!(!factory.is_empty(), "metadata must reach source validation");
}
