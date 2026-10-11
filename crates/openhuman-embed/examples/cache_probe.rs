//! Title: Prompt-cache probe
//! Summary: Drives several session turns against a live OpenAI-compatible route.
//! Run: live only (CACHE_PROBE=1, OPENROUTER_API_KEY); local scratch.
//! Feature: default

mod support;
use openhuman_embed::{AgentDefinitionSpec, AgentSpec, Provider, Runtime, ToolScopeSpec, Workspace};

fn main() -> anyhow::Result<()> {
    support::run(run())
}

fn long_prompt() -> String {
    let mut s = String::from("You are a careful analyst seat in a research hive. Follow the protocol.\n");
    for i in 0..220 {
        s.push_str(&format!(
            "Rule {i}: when a claim about item {i} is made, check it against the ledger entry for item {i}, record the verdict, and never assume entry {} matches.\n",
            i + 1
        ));
    }
    s
}

async fn run() -> anyhow::Result<()> {
    let key = std::env::var("OPENROUTER_API_KEY")?;
    let base = std::env::var("CACHE_PROBE_BASE").unwrap_or_else(|_| "https://openrouter.ai/api/v1".into());
    let model = std::env::var("CACHE_PROBE_MODEL").unwrap_or_else(|_| "google/gemini-3.8-flash".into());
    let turns: usize = std::env::var("CACHE_PROBE_TURNS").ok().and_then(|v| v.parse().ok()).unwrap_or(5);
    let backend = support::stub_backend().await;
    let mut config = support::offline_config();
    config.agent.compact_context = false;
    config.default_temperature = 0.2;
    let runtime = Runtime::builder()
        .config(config)
        .workspace(Workspace::Ephemeral)
        .backend_url(backend.uri())
        .provider(Provider::openai_compatible(base, key).model(&model))
        .build()
        .await?;
    let agent = runtime.agent(
        AgentSpec::new("probe")
            .definition(
                AgentDefinitionSpec::new()
                    .bare_prompt(long_prompt())
                    .tools(ToolScopeSpec::HostOnly)
                    .max_iterations(6)
                    .temperature(0.2),
            )
            .tools(|_| openhuman_embed::HostTurnTools::advertised(vec![Box::new(Lookup)])),
    )?;
    for turn in 0..turns {
        let msg = format!(
            "## Your desk\nturn {turn}\n\n## New on your desk since your last turn\n[{}] @peer (note): item {} looks fine to me, please check the ledger via lookup_ledger for item {} and then answer in one line.\n\nEnd with a one line verdict.",
            turn * 3,
            turn,
            turn
        );
        let out = agent.turn(msg).session("probe-session").send().await?;
        println!("turn {turn}: {}", out.reply.chars().take(60).collect::<String>());
    }
    support::passed("cache_probe");
    Ok(())
}

struct Lookup;
#[async_trait::async_trait]
impl openhuman_embed::Tool for Lookup {
    fn name(&self) -> &str {
        "lookup_ledger"
    }
    fn description(&self) -> &str {
        "Look up the ledger entry for an item number"
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({"type":"object","properties":{"item":{"type":"integer"}},"required":["item"]})
    }
    fn policy(&self) -> openhuman_embed::ToolPolicy {
        openhuman_embed::ToolPolicy::read_only()
    }
    async fn execute(&self, a: serde_json::Value) -> anyhow::Result<openhuman_embed::ToolResult> {
        Ok(openhuman_embed::ToolResult::success(format!(
            "ledger entry {}: verdict pending, amount 42, notes: nothing unusual recorded for this item in the ledger.",
            a["item"]
        )))
    }
}
