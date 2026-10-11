//! A turn in a thread that already made a successful `cron` call must survive
//! a later turn where a tool call is refused. One runtime and one test: SaaS
//! operating mode is process-wide.

mod common;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::{
    chat_completion, chat_requests, runtime_without_master_key, tool_names, PointedTransport,
};
use openhuman_embed::seams::ToolHookDecision;
use openhuman_embed::{
    CronDelivery, CronReply, DirectProfileProvider, JobOrigin, PermissionLevel, ProfileRuntime,
    SaasConfig, ToolScopeSpec,
};
use serde_json::{json, Value};
use wiremock::matchers::{header, method, path_regex};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

const CAP: usize = 3;

/// Scripts the model from the last user message.
struct Script {
    fire_at: String,
}

impl Respond for Script {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        let messages = body["messages"].as_array().unwrap();
        let last_user = messages.iter().rposition(|m| m["role"] == "user").unwrap();
        let results = messages[last_user + 1..]
            .iter()
            .filter(|m| m["role"] == "tool")
            .count();
        let prompt = common::last_user_message(&body);
        let call = |name: &str, args: Value| {
            let mut completion = common::tool_call_completion(name, &args.to_string());
            completion["choices"][0]["message"]["tool_calls"][0]["id"] =
                json!(format!("call_{results}"));
            completion
        };
        let completion = if prompt.contains("loop forever") {
            call("current_time", json!({}))
        } else if prompt.contains("schedule it") && results == 0 {
            call("tool_search", json!({"query":"cron schedule job"}))
        } else if prompt.contains("schedule it") && results == 1 {
            call(
                "cron",
                json!({"action":"add","job_type":"agent","name":"ping",
                    "schedule":{"kind":"at","at":self.fire_at},"prompt":"reminder ping"}),
            )
        } else if prompt.contains("shell job") && results == 0 {
            call("tool_search", json!({"query":"cron schedule job"}))
        } else if prompt.contains("shell job") && results == 1 {
            call(
                "cron",
                json!({"action":"add","job_type":"shell","name":"sh",
                    "schedule":{"kind":"at","at":self.fire_at},"command":"echo hi"}),
            )
        } else if prompt.contains("run shell") && results == 0 {
            call("shell", json!({"command":"ls"}))
        } else if prompt.contains("reminder ping") {
            chat_completion("pong")
        } else {
            chat_completion("done")
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
        let chunk = json!({"id":"f","object":"chat.completion.chunk","created":1700000000_u64,"model":"fixture-model","choices":[{"index":0,"delta":delta,"finish_reason":null}]});
        let done = json!({"id":"f","object":"chat.completion.chunk","created":1700000000_u64,"model":"fixture-model","choices":[{"index":0,"delta":{},"finish_reason":completion["choices"][0]["finish_reason"]}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}});
        ResponseTemplate::new(200).set_body_raw(
            format!("data: {chunk}\n\ndata: {done}\n\ndata: [DONE]\n\n"),
            "text/event-stream",
        )
    }
}

#[derive(Default)]
struct Recorder(Mutex<Vec<CronReply>>);

#[async_trait::async_trait]
impl CronDelivery for Recorder {
    async fn deliver(&self, reply: CronReply) -> Result<(), String> {
        self.0.lock().unwrap().push(reply);
        Ok(())
    }
}

#[test]
fn refused_tool_call_after_a_successful_cron_call_in_the_same_thread_does_not_fail_the_turn() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("warn,openhuman_core=info")
        .with_test_writer()
        .try_init();
    runtime_without_master_key().block_on(async {
        tokio::spawn(scenario()).await.unwrap();
    });
}

async fn scenario() {
    let chat = MockServer::start().await;
    let managed = MockServer::start().await;
    let fire_at = chrono::Utc::now() + chrono::Duration::seconds(30);
    Mock::given(method("POST"))
        .and(path_regex(r"/v1/chat/completions$"))
        .and(header("authorization", "Bearer operator-chat-key"))
        .respond_with(Script {
            fire_at: fire_at.to_rfc3339(),
        })
        .mount(&chat)
        .await;
    Mock::given(wiremock::matchers::any())
        .respond_with(ResponseTemplate::new(500))
        .mount(&managed)
        .await;
    PointedTransport::install(&managed.uri());
    openhuman_core::storage::install(Arc::new(openhuman_core::storage::MemoryStorage::new()));

    let recorder = Arc::new(Recorder::default());
    let root = tempfile::tempdir().unwrap();
    let mut saas = SaasConfig::new(root.path());
    saas.chat_provider = Some(
        DirectProfileProvider::openai_compatible(
            format!("{}/v1", chat.uri()),
            "operator-chat-key",
            "fixture-model",
        )
        .unwrap(),
    );
    saas.cron.agent_jobs_only = true;
    let runtime = ProfileRuntime::builder(saas)
        .configure(|config| config.lease_ttl_secs = 3600)
        .tool_scope(ToolScopeSpec::Named(
            ["cron", "current_time", "resolve_time", "memory"]
                .map(String::from)
                .to_vec(),
        ))
        .can_use_tool(|_| Box::pin(async { ToolHookDecision::Proceed }))
        .channel_permissions([("web_channel", PermissionLevel::Execute)])
        .cron_delivery(recorder.clone())
        .build()
        .await
        .unwrap();
    runtime.provision("alice").await.unwrap();
    let handle = runtime.open("alice").await.unwrap();

    handle.chat("t1", "schedule it").await.unwrap();
    let later = chrono::Utc::now() + chrono::Duration::hours(1);
    assert_eq!(runtime.sweep_cron_at(later).await.ran, 1);
    tokio::time::timeout(Duration::from_secs(30), async {
        while recorder.0.lock().unwrap().is_empty() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    handle
        .chat("t1", "make a shell job")
        .await
        .expect("a refused tool call must not fail the turn");
    handle
        .chat("t1", "run shell")
        .await
        .expect("an unknown tool call must not fail the turn");
    handle.chat("t1", "and once more").await.unwrap();
    let requests = chat_requests(&chat).await;
    assert!(
        requests
            .iter()
            .any(|r| String::from_utf8_lossy(&r.body).contains("only agent cron jobs are allowed")),
        "the refusal reaches the model"
    );
}
