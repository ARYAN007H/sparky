use super::*;
use crate::agent::host_runtime::NativeRuntime;
use crate::tools::ops::all_tools_with_runtime;
use tempfile::TempDir;
use tinytools::ToolResult;

struct FakeTool(&'static str);

#[async_trait::async_trait]
impl Tool for FakeTool {
    fn name(&self) -> &str {
        self.0
    }

    fn description(&self) -> &str {
        "fake tool from a registered source"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({ "type": "object", "properties": {} })
    }

    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult::success("ok"))
    }
}

fn assemble(tmp: &TempDir) -> Vec<String> {
    let cfg = Config {
        workspace_dir: tmp.path().join("workspace"),
        action_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("config.toml"),
        ..Config::default()
    };
    let security = Arc::new(SecurityPolicy::default());
    let browser = BrowserConfig {
        enabled: false,
        ..BrowserConfig::default()
    };
    let http = HttpRequestConfig::default();
    all_tools_with_runtime(
        Arc::new(cfg.clone()),
        &security,
        Arc::new(NativeRuntime::new()),
        AuditLogger::disabled(),
        &browser,
        &http,
        tmp.path(),
        &HashMap::new(),
        &cfg,
        None,
    )
    .iter()
    .map(|t| t.name().to_string())
    .collect()
}

#[test]
fn nothing_registered_leaves_the_catalogue_without_source_tools() {
    let tmp = TempDir::new().unwrap();
    let names = assemble(&tmp);
    assert!(!names.is_empty());
    assert!(!names.iter().any(|n| n.starts_with("source_fake_")));
}

#[test]
fn registered_sources_append_in_registration_order_after_built_ins() {
    let tmp = TempDir::new().unwrap();
    let baseline = assemble(&tmp);

    register_tool_source("test::second", |_| {
        vec![Box::new(FakeTool("source_fake_b"))]
    });
    register_tool_source("test::first", |_| {
        vec![
            Box::new(FakeTool("source_fake_a1")),
            Box::new(FakeTool("source_fake_a2")),
        ]
    });
    let names = assemble(&tmp);
    unregister_tool_source("test::second");
    unregister_tool_source("test::first");

    let position = |name: &str| names.iter().position(|n| n == name).unwrap();
    assert!(position("source_fake_b") < position("source_fake_a1"));
    assert_eq!(position("source_fake_a2"), position("source_fake_a1") + 1);
    // Built-ins precede registered sources.
    assert!(position("file_read") < position("source_fake_b"));
    assert_eq!(names.len(), baseline.len() + 3);
}

#[test]
fn a_source_receives_the_catalogue_context() {
    let tmp = TempDir::new().unwrap();
    let seen = Arc::new(std::sync::Mutex::new(None));
    let sink = Arc::clone(&seen);
    register_tool_source("test::ctx", move |ctx| {
        *sink.lock().unwrap() = Some((ctx.action_dir.to_path_buf(), ctx.agents.len()));
        Vec::new()
    });
    let _ = assemble(&tmp);
    unregister_tool_source("test::ctx");
    let (action_dir, agents) = seen.lock().unwrap().clone().expect("source ran");
    assert_eq!(action_dir, tmp.path());
    assert_eq!(agents, 0);
}

#[test]
fn unregistering_a_source_removes_its_tools() {
    let tmp = TempDir::new().unwrap();
    register_tool_source("test::gone", |_| {
        vec![Box::new(FakeTool("source_fake_gone"))]
    });
    assert!(assemble(&tmp).contains(&"source_fake_gone".to_string()));
    assert!(unregister_tool_source("test::gone"));
    assert!(!assemble(&tmp).contains(&"source_fake_gone".to_string()));
}
