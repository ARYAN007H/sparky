use super::*;
use tinyagents_graph::observability::GraphObservation;
use tinyagents_graph::stream::GraphEvent;
use tinyagents_harness::ids::{EventId, GraphId, NodeId, RunId, ThreadId};
use tinyflows_bus::runs::FlowRunTrigger;

fn sample_observations(thread_id: &str) -> Vec<GraphObservation> {
    let node = NodeId::new("fetch");
    let mk = |offset: u64, step: usize, ts_ms: u64, event: GraphEvent| GraphObservation {
        event_id: EventId::new(format!("evt-{offset}")),
        run_id: RunId::new("run-9"),
        root_run_id: RunId::new("run-9"),
        parent_run_id: None,
        thread_id: Some(ThreadId::new(thread_id)),
        graph_id: GraphId::new("workflow"),
        checkpoint_id: None,
        namespace: Vec::new(),
        step,
        offset,
        ts_ms,
        event,
    };
    vec![
        mk(
            0,
            0,
            1_000,
            GraphEvent::RunStarted {
                run_id: RunId::new("run-9"),
            },
        ),
        mk(
            1,
            1,
            1_010,
            GraphEvent::StepStarted {
                step: 1,
                active: vec![node.clone()],
            },
        ),
        mk(
            2,
            1,
            1_020,
            GraphEvent::NodeStarted {
                node: node.clone(),
                step: 1,
            },
        ),
        mk(3, 1, 1_050, GraphEvent::NodeCompleted { node, step: 1 }),
        mk(4, 1, 1_060, GraphEvent::StepCompleted { step: 1 }),
    ]
}

fn otlp_attr<'a>(span: &'a Value, key: &str) -> Option<&'a str> {
    span["attributes"]
        .as_array()?
        .iter()
        .find(|item| item["key"] == key)?["value"]["stringValue"]
        .as_str()
}

#[test]
fn graph_export_keeps_flow_identity_and_graph_view_attributes() {
    let thread_id = "flow:flow-1:uuid-1";
    let requests = graph_otlp_requests(
        "Daily digest",
        "flow-1",
        thread_id,
        "completed",
        FlowRunTrigger::Rpc,
        &sample_observations(thread_id),
    )
    .expect("OTLP requests");

    let resource = &requests[0]["resourceSpans"][0];
    assert_eq!(
        resource["resource"]["attributes"][0]["value"]["stringValue"],
        "openhuman"
    );
    let spans = resource["scopeSpans"][0]["spans"]
        .as_array()
        .expect("spans");
    assert!(spans.len() >= 3, "run, step and node spans");
    for span in spans {
        assert_eq!(
            otlp_attr(span, "langfuse.trace.name"),
            Some("flow.run:Daily digest")
        );
        assert_eq!(otlp_attr(span, "langfuse.session.id"), Some("flow:flow-1"));
        assert_eq!(otlp_attr(span, "langfuse.release"), Some(APP_VERSION));
        assert_eq!(
            otlp_attr(span, "langfuse.trace.metadata.flow_id"),
            Some("flow-1")
        );
        assert_eq!(
            otlp_attr(span, "langfuse.trace.metadata.trigger"),
            Some("rpc")
        );
        assert!(otlp_attr(span, "langfuse.trace.input").is_none());
    }
    let node = spans
        .iter()
        .find(|span| span["name"] == "fetch")
        .expect("node span");
    assert_eq!(
        otlp_attr(node, "langfuse.observation.metadata.langgraph_node"),
        Some("fetch")
    );
    assert_eq!(
        otlp_attr(node, "langfuse.observation.metadata.flow_id"),
        Some("flow-1")
    );
}

#[test]
fn flow_trace_config_uses_bus_trigger_and_flow_coordinates() {
    let trace = build_flow_trace_config(
        "Daily digest",
        "flow-1",
        "flow:flow-1:uuid-1",
        "completed",
        FlowRunTrigger::Schedule,
    );
    assert_eq!(trace.trace_id.as_deref(), Some("flow:flow-1:uuid-1"));
    assert_eq!(trace.session_id.as_deref(), Some("flow:flow-1"));
    assert_eq!(trace.name.as_deref(), Some("flow.run:Daily digest"));
    assert_eq!(trace.tags, vec!["run:flow", "trigger:schedule"]);
    assert_eq!(trace.metadata["flow_id"], "flow-1");
    assert_eq!(trace.metadata["status"], "completed");
    assert_eq!(trace.metadata["source"], "flows");
    assert_eq!(trace.metadata["run_type"], "flow");
    assert_eq!(trace.metadata["trigger"], "schedule");
    assert_eq!(trace.release.as_deref(), Some(APP_VERSION));
    assert_eq!(trace.metadata["app_version"], APP_VERSION);
}

#[test]
fn flow_export_uses_the_backend_v4_otlp_route() {
    let client = LangfuseClient::new(
        "https://staging-api.tinyhumans.ai/telemetry/langfuse/ingestion",
        LangfuseAuth::Bearer {
            token: "token".to_string(),
        },
    )
    .expect("client");
    assert_eq!(
        build_flow_exporter(client, "flow-1")
            .client()
            .otlp_endpoint(),
        "https://staging-api.tinyhumans.ai/telemetry/langfuse/otel/v1/traces"
    );
}

#[tokio::test]
async fn flow_export_is_a_noop_when_usage_sharing_is_disabled() {
    let mut config = Config::default();
    config.observability.share_usage_data = false;
    export_graph_run_trace(
        &config,
        "Daily digest",
        "flow-1",
        "flow:flow-1:uuid-1",
        "completed",
        FlowRunTrigger::Rpc,
        &sample_observations("flow:flow-1:uuid-1"),
    )
    .await;
}

#[tokio::test]
async fn flow_export_skips_api_key_credentials() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let mut config = Config::default();
    config.config_path = dir.path().join("config.toml");
    config.workspace_dir = dir.path().join("workspace");
    config.secrets.encrypt = false;
    config.api_url = Some(format!("http://{address}"));
    config.observability.share_usage_data = true;
    crate::security::credentials::api_key::store_api_key(&config, "th_live_test").unwrap();

    export_graph_run_trace(
        &config,
        "Daily digest",
        "flow-1",
        "flow:flow-1:uuid-1",
        "completed",
        FlowRunTrigger::Rpc,
        &sample_observations("flow:flow-1:uuid-1"),
    )
    .await;

    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(250), listener.accept())
            .await
            .is_err(),
        "API-key credentials must not send a Langfuse export request"
    );
}
