//! An embedded agent on the harness domains gets the scheduler and clock tools
//! it names, although their domain family is off, and nothing else from that
//! family: not the shell or file tools, even when it names them too.
//!
//! One runtime per process, so one test in this binary.

mod common;

use std::sync::Arc;

use common::{chat_requests, offline_config, provider, runtime, stub_backend, tool_names};
use openhuman_embed::{
    AgentDefinitionSpec, AgentSpec, DomainSet, HostTurnTools, InMemorySessionStores, Runtime,
    ToolGroups, ToolScopeSpec, Workspace,
};

#[test]
fn a_named_scope_admits_the_clock_and_scheduler_but_never_the_shell() {
    runtime().block_on(async {
        tokio::spawn(async {
            let backend = stub_backend().await;
            let model = provider("ok").await;
            let runtime = Runtime::builder()
                .config(offline_config())
                .workspace(Workspace::Stateless)
                .session_store(Arc::new(InMemorySessionStores::new()))
                .backend_url(backend.uri())
                .domains(DomainSet {
                    inference: true,
                    ..DomainSet::harness()
                })
                .tool_groups(ToolGroups::advertised())
                .build()
                .await
                .expect("runtime builds");
            let named = ["cron", "current_time", "resolve_time", "shell", "file_read"];
            let agent = runtime
                .agent(
                    AgentSpec::new("teeny")
                        .provider(common::route(&model, "fixture"))
                        .definition(AgentDefinitionSpec::new().system_prompt("TEENY").tools(
                            ToolScopeSpec::Named(named.iter().map(|n| (*n).to_string()).collect()),
                        ))
                        .tools(|_| HostTurnTools::advertised(Vec::new())),
                )
                .expect("agent instantiates");
            agent.run("hello").await.expect("the turn runs");

            let requests = chat_requests(&model).await;
            let offered = tool_names(&requests[0]);
            for admitted in ["cron", "current_time", "resolve_time"] {
                assert!(
                    offered.iter().any(|name| name == admitted),
                    "{admitted} missing: {offered:?}"
                );
            }
            for refused in ["shell", "file_read"] {
                assert!(
                    !offered.iter().any(|name| name == refused),
                    "{refused} offered: {offered:?}"
                );
            }
            drop(agent);
            drop(runtime);
        })
        .await
        .expect("library host task did not panic");
    });
}
